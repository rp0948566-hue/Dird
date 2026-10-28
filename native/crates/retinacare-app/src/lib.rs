pub mod cascade;
pub mod database;
pub mod models;
pub mod resources;
use resources::ResourcePaths;
use retinacare_ai_bridge::WorkerClient;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

enum WorkerAction {
    Health,
    AnalyzeImage(String),
    Stop,
}

pub struct WorkerSupervisor {
    actions: SyncSender<WorkerAction>,
    updates: Receiver<Value>,
    cancelled: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WorkerSupervisor {
    pub fn start_default() -> Self {
        let (actions, action_rx) = mpsc::sync_channel(4);
        let (update_tx, updates) = mpsc::sync_channel(8);
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let thread = thread::spawn(move || run_worker(action_rx, update_tx, stop));
        Self {
            actions,
            updates,
            cancelled,
            thread: Some(thread),
        }
    }
    pub fn refresh(&self) {
        let _ = self.actions.try_send(WorkerAction::Health);
    }
    pub fn analyze_image(&self, path: String) {
        let _ = self.actions.try_send(WorkerAction::AnalyzeImage(path));
    }
    pub fn take_latest(&self) -> Option<Value> {
        self.updates.try_iter().last()
    }
}

impl Drop for WorkerSupervisor {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        let _ = self.actions.try_send(WorkerAction::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_worker(
    actions: Receiver<WorkerAction>,
    updates: SyncSender<Value>,
    cancelled: Arc<AtomicBool>,
) {
    let mut client = None;
    let mut action = WorkerAction::Health;
    loop {
        if cancelled.load(Ordering::Relaxed) || matches!(action, WorkerAction::Stop) {
            break;
        }
        let result = (|| -> Result<Value, String> {
            if client.is_none() {
                let paths = ResourcePaths::discover().map_err(|error| error.to_string())?;
                paths.prepare_data().map_err(|error| error.to_string())?;
                client = Some(
                    WorkerClient::spawn_configured(
                        &paths.python,
                        &paths.worker_source,
                        Some(&paths.models),
                        Some(&paths.data.join("logs/worker.log")),
                    )
                    .map_err(|error| error.to_string())?,
                );
            }
            let (command, payload) = match &action {
                WorkerAction::AnalyzeImage(path) => ("analyze_image", json!({"image_path":path})),
                _ => ("health", json!({})),
            };
            client
                .as_mut()
                .ok_or("worker unavailable")?
                .request_cancellable(command, payload, &cancelled, Duration::from_secs(60))
                .map_err(|error| error.to_string())
        })();
        let update = match result {
            Ok(update) => update,
            Err(error) => {
                client = None;
                json!({"status":"error","error":{"code":"worker_unavailable","message":error}})
            }
        };
        let _ = updates.try_send(update);
        match actions.recv() {
            Ok(next) => action = next,
            Err(_) => break,
        }
    }
}

pub fn smoke_worker(paths: &ResourcePaths) -> Result<WorkerClient, Box<dyn std::error::Error>> {
    paths.prepare_data()?;
    Ok(WorkerClient::spawn_configured(
        &paths.python,
        &paths.worker_source,
        Some(&paths.models),
        Some(&paths.data.join("logs/worker.log")),
    )?)
}

pub fn smoke_database() -> Result<(), Box<dyn std::error::Error>> {
    use retinacare_persistence::SqlCipherStore;
    let root = std::env::temp_dir().join(format!(
        "RetinaCare smoke 患者 {}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let path = root.join("test.sqlite");
        let store = SqlCipherStore::open(&path, "smoke-test-only")?;
        let id = store.add_patient("SMOKE", "Synthetic Patient")?;
        store.add_examination(id, "smoke", "synthetic record")?;
        drop(store);
        let reopened = SqlCipherStore::open(&path, "smoke-test-only")?;
        if reopened.list_patients()?.len() != 1 || reopened.list_examinations()?.len() != 1 {
            return Err("database round-trip failed".into());
        }
        Ok(())
    })();
    if root.exists() {
        std::fs::remove_dir_all(root)?;
    }
    result
}
