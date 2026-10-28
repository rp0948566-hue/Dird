//! Bounded, versioned local IPC. Cancellation terminates an unresponsive worker.
pub mod device;
pub mod model_manifest;
pub mod model_registry;
pub mod model_runtime;
pub mod onnx_session;
pub mod preprocessing;
pub mod stage1;
pub mod stage2_contract;
pub mod stage2_preprocessing;

use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

const PROTOCOL_VERSION: u32 = 1;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 4096;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("worker process I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("worker response timed out")]
    Timeout,
    #[error("worker request cancelled")]
    Cancelled,
    #[error("worker exited without a response")]
    Exited,
    #[error("worker returned malformed JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("worker returned an invalid response envelope")]
    InvalidResponse,
    #[error("IPC message exceeded its size limit")]
    MessageTooLarge,
}

pub struct WorkerClient {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    responses: Receiver<Result<String, WorkerError>>,
    sequence: u64,
}

impl WorkerClient {
    pub fn spawn(python: impl AsRef<OsStr>, package_source: &Path) -> Result<Self, WorkerError> {
        Self::spawn_configured(python, package_source, None, None)
    }

    pub fn spawn_configured(
        python: impl AsRef<OsStr>,
        package_source: &Path,
        models: Option<&Path>,
        log: Option<&Path>,
    ) -> Result<Self, WorkerError> {
        let mut command = Command::new(python);
        command
            .args(["-u", "-m", "retinacare_ai.worker"])
            .env("PYTHONPATH", package_source)
            .env("PYTHONNOUSERSITE", "1")
            .env("PYTHONUTF8", "1")
            .env_remove("PYTHONHOME");
        if let Some(models) = models {
            command.env("RETINACARE_MODEL_DIR", models);
        }
        if let Some(log) = log {
            command.stderr(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log)?,
            );
        } else {
            command.stderr(Stdio::null());
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW; keep IPC pipes attached.
        }
        Self::spawn_command(command)
    }

    fn spawn_command(mut command: Command) -> Result<Self, WorkerError> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        // Both handles exist because this function explicitly requested piped stdio.
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let (sender, responses) = mpsc::sync_channel(4);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                let result = reader
                    .by_ref()
                    .take((MAX_RESPONSE_BYTES + 1) as u64)
                    .read_until(b'\n', &mut bytes);
                if matches!(result, Ok(0)) {
                    break;
                }
                let line = match result {
                    Ok(_) if bytes.len() > MAX_RESPONSE_BYTES => Err(WorkerError::MessageTooLarge),
                    Ok(_) => String::from_utf8(bytes).map_err(|_| WorkerError::InvalidResponse),
                    Err(error) => Err(WorkerError::Io(error)),
                };
                let invalid = line.is_err();
                if sender.send(line).is_err() || invalid {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            responses,
            sequence: 0,
        })
    }

    pub fn request(&mut self, command: &str, payload: Value) -> Result<Value, WorkerError> {
        self.request_cancellable(
            command,
            payload,
            &AtomicBool::new(false),
            Duration::from_secs(60),
        )
    }

    pub fn request_cancellable(
        &mut self,
        command: &str,
        payload: Value,
        cancelled: &AtomicBool,
        timeout: Duration,
    ) -> Result<Value, WorkerError> {
        let result = self.exchange(command, payload, cancelled, timeout);
        if result.is_err() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        result
    }

    fn exchange(
        &mut self,
        command: &str,
        payload: Value,
        cancelled: &AtomicBool,
        timeout: Duration,
    ) -> Result<Value, WorkerError> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(WorkerError::Cancelled);
        }
        self.sequence += 1;
        let request_id = format!("rc-{}", self.sequence);
        let mut request = serde_json::to_vec(
            &json!({"protocol_version":PROTOCOL_VERSION,"request_id":request_id,"command":command,"payload":payload}),
        )?;
        request.push(b'\n');
        if request.len() > MAX_REQUEST_BYTES {
            return Err(WorkerError::MessageTooLarge);
        }
        self.stdin.write_all(&request)?;
        self.stdin.flush()?;
        let started = Instant::now();
        let line = loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(WorkerError::Cancelled);
            }
            if started.elapsed() >= timeout {
                return Err(WorkerError::Timeout);
            }
            match self.responses.recv_timeout(
                Duration::from_millis(50).min(timeout.saturating_sub(started.elapsed())),
            ) {
                Ok(line) => break line?,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(WorkerError::Exited),
            }
        };
        let response: Value = serde_json::from_str(&line)?;
        if response["request_id"].as_str() != Some(&request_id)
            || response["protocol_version"].as_u64() != Some(u64::from(PROTOCOL_VERSION))
            || !matches!(response["status"].as_str(), Some("ok" | "error"))
        {
            return Err(WorkerError::InvalidResponse);
        }
        Ok(response)
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        let _ = self.stdin.write_all(b"{\"protocol_version\":1,\"request_id\":\"stop\",\"command\":\"shutdown\",\"payload\":{}}\n");
        let _ = self.stdin.flush();
        for _ in 0..10 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn python() -> &'static str {
        if cfg!(windows) { "python" } else { "python3" }
    }
    #[test]
    fn python_worker_health_roundtrip() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../ai_engine/src");
        let mut worker = WorkerClient::spawn(python(), &source).unwrap();
        let response = worker.request("health", json!({})).unwrap();
        assert_eq!(response["status"], "ok");
        assert!(response["data"]["device"].is_object());
    }
    #[test]
    fn unresponsive_worker_times_out_and_is_reaped() {
        let mut command = Command::new(python());
        command.args(["-c", "import time; time.sleep(30)"]);
        let mut worker = WorkerClient::spawn_command(command).unwrap();
        assert!(matches!(
            worker.request_cancellable(
                "health",
                json!({}),
                &AtomicBool::new(false),
                Duration::from_millis(100)
            ),
            Err(WorkerError::Timeout)
        ));
        assert!(worker.child.try_wait().unwrap().is_some());
    }
    #[test]
    fn oversized_worker_response_is_rejected() {
        let mut command = Command::new(python());
        command.args([
            "-c",
            "import sys; sys.stdin.readline(); print('x' * 1048577, flush=True)",
        ]);
        let mut worker = WorkerClient::spawn_command(command).unwrap();
        assert!(matches!(
            worker.request("health", json!({})),
            Err(WorkerError::MessageTooLarge)
        ));
    }
    #[test]
    fn mismatched_protocol_is_rejected() {
        let mut command = Command::new(python());
        command.args(["-c", "import sys; sys.stdin.readline(); print('{\"protocol_version\":2,\"request_id\":\"rc-1\",\"status\":\"ok\"}', flush=True)"]);
        let mut worker = WorkerClient::spawn_command(command).unwrap();
        assert!(matches!(
            worker.request("health", json!({})),
            Err(WorkerError::InvalidResponse)
        ));
    }
}
