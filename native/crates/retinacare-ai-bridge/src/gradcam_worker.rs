//! Shared runner for the local PyTorch Grad-CAM workers (`scripts/stage1_gradcam.py`,
//! `scripts/stage3_gradcam.py`): trusted pinned upstream clone, no network, fixed arguments, a private
//! temporary folder, bounded output, a timeout, and a kill on cancel. Nothing partial is ever returned.

use crate::attribution::AttributionError;
use std::{
    ffi::OsString,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const WORKER_OUTPUT_LIMIT: usize = 1 << 20;

/// Run `python -I script <args> --input <tensor file>` and return its stdout (at most 1 MiB).
pub fn run_worker(
    python: &Path,
    script: &Path,
    args: &[OsString],
    tensor: &[f32],
    timeout: Duration,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Vec<u8>, AttributionError> {
    let started = Instant::now();
    let dir = std::env::temp_dir().join(format!(
        "retinacare-explain-{}-{}",
        std::process::id(),
        started.elapsed().as_nanos() ^ u128::from(rand_tag())
    ));
    std::fs::create_dir(&dir).map_err(|e| AttributionError::Scoring(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let outcome = (|| {
        let file = dir.join("input.f32");
        let bytes: Vec<u8> = tensor.iter().flat_map(|v| v.to_le_bytes()).collect();
        std::fs::write(&file, bytes).map_err(|e| AttributionError::Scoring(e.to_string()))?;
        let mut child = Command::new(python)
            .arg("-I")
            .arg(script)
            .args(args)
            .arg("--input")
            .arg(&file)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| AttributionError::Scoring(format!("worker did not start: {e}")))?;
        let mut stdout = child.stdout.take().expect("piped stdout");
        let reader = std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = stdout
                .by_ref()
                .take(WORKER_OUTPUT_LIMIT as u64 + 1)
                .read_to_end(&mut buffer);
            buffer
        });
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let buffer = reader.join().unwrap_or_default();
                    if !status.success() || buffer.len() > WORKER_OUTPUT_LIMIT {
                        return Err(AttributionError::Scoring("Grad-CAM worker failed".into()));
                    }
                    return Ok(buffer);
                }
                Ok(None) => {}
                Err(e) => return Err(AttributionError::Scoring(e.to_string())),
            }
            if cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AttributionError::Cancelled);
            }
            if started.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AttributionError::Scoring(
                    "Grad-CAM worker timed out".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    outcome
}

fn rand_tag() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The tests share the process-wide temp folder listing, so they run one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn script(name: &str, body: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rc-worker-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("w.sh");
        std::fs::write(&path, body).unwrap();
        path
    }

    // /bin/sh stands in for python: `-I` is not a valid sh option, so the test script ignores the call
    // convention and only checks the surrounding behaviour through a tiny wrapper.
    fn wrapper(name: &str, body: &str) -> std::path::PathBuf {
        let path = script(name, &format!("#!/bin/sh\nshift\n{body}\n"));
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn output_is_returned_and_the_temporary_folder_is_removed() {
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let python = wrapper("ok", "echo '{\"ok\":true}'");
        let before: Vec<_> = leftover();
        let out = run_worker(
            &python,
            Path::new("ignored"),
            &[],
            &[1.0, 2.0],
            Duration::from_secs(10),
            || false,
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out).trim(), "{\"ok\":true}");
        assert_eq!(
            leftover().len(),
            before.len(),
            "the private folder was removed"
        );
    }

    #[test]
    fn failure_cancel_and_timeout_return_errors_without_output() {
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fail = wrapper("fail", "exit 3");
        assert!(
            run_worker(
                &fail,
                Path::new("x"),
                &[],
                &[0.0],
                Duration::from_secs(10),
                || false
            )
            .is_err()
        );
        let slow = wrapper("slow", "sleep 30");
        let cancel = run_worker(
            &slow,
            Path::new("x"),
            &[],
            &[0.0],
            Duration::from_secs(20),
            || true,
        );
        assert_eq!(cancel.unwrap_err(), AttributionError::Cancelled);
        let timeout = run_worker(
            &slow,
            Path::new("x"),
            &[],
            &[0.0],
            Duration::from_millis(300),
            || false,
        );
        assert!(matches!(timeout, Err(AttributionError::Scoring(m)) if m.contains("timed out")));
    }

    #[test]
    fn oversized_output_is_rejected() {
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let big = wrapper("big", "head -c 2000000 /dev/zero | tr '\\0' 'a'");
        assert!(
            run_worker(
                &big,
                Path::new("x"),
                &[],
                &[0.0],
                Duration::from_secs(10),
                || false
            )
            .is_err()
        );
    }

    fn leftover() -> Vec<std::path::PathBuf> {
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    n.starts_with(&format!("retinacare-explain-{}-", std::process::id()))
                })
            })
            .collect()
    }
}
