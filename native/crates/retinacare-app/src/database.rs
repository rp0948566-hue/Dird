//! Own the SQLCipher connection on one thread; KDF/database work never blocks UI frames.
use crate::resources::ResourcePaths;
use retinacare_domain::Examination;
use retinacare_persistence::{PatientRecord, SqlCipherStore};
use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
};
use zeroize::Zeroizing;

pub enum DatabaseAction {
    Open(Zeroizing<String>),
    AddPatient(String, String),
    UpdatePatient(i64, String, String),
    AddExamination(i64, String, String, String, String, String),
    Lock,
    Stop,
}
#[derive(Default)]
pub struct DatabaseSnapshot {
    pub unlocked: bool,
    pub patients: Vec<PatientRecord>,
    pub examinations: Vec<Examination>,
}
pub struct DatabaseController {
    actions: SyncSender<DatabaseAction>,
    updates: Receiver<Result<DatabaseSnapshot, String>>,
    thread: Option<JoinHandle<()>>,
}
impl Default for DatabaseController {
    fn default() -> Self {
        let (actions, rx) = mpsc::sync_channel(4);
        let (tx, updates) = mpsc::sync_channel(8);
        let thread = thread::spawn(move || {
            let mut store = None;
            while let Ok(action) = rx.recv() {
                if matches!(action, DatabaseAction::Stop) {
                    break;
                }
                let result = (|| -> Result<DatabaseSnapshot, String> {
                    match action {
                        DatabaseAction::Open(password) => {
                            if store.is_some() {
                                return Err("Lock the current database first".into());
                            }
                            let paths = ResourcePaths::discover().map_err(|e| e.to_string())?;
                            paths.prepare_data().map_err(|e| e.to_string())?;
                            store = Some(
                                SqlCipherStore::open(
                                    paths.data.join("database/retinacare.sqlite"),
                                    &password,
                                )
                                .map_err(|e| e.to_string())?,
                            );
                        }
                        DatabaseAction::AddPatient(code, name) => {
                            store
                                .as_ref()
                                .ok_or("Database locked")?
                                .add_patient(&code, &name)
                                .map_err(|e| e.to_string())?;
                        }
                        DatabaseAction::UpdatePatient(id, code, name) => {
                            store
                                .as_ref()
                                .ok_or("Database locked")?
                                .update_patient(id, &code, &name)
                                .map_err(|e| e.to_string())?;
                        }
                        DatabaseAction::AddExamination(
                            id,
                            operator,
                            notes,
                            eye,
                            image_path,
                            analysis_json,
                        ) => {
                            store
                                .as_ref()
                                .ok_or("Database locked")?
                                .add_screening_examination(
                                    id,
                                    &operator,
                                    &notes,
                                    &eye,
                                    &image_path,
                                    &analysis_json,
                                )
                                .map_err(|e| e.to_string())?;
                        }
                        DatabaseAction::Lock => store = None,
                        DatabaseAction::Stop => unreachable!(),
                    }
                    match &store {
                        Some(store) => Ok(DatabaseSnapshot {
                            unlocked: true,
                            patients: store.list_patients().map_err(|e| e.to_string())?,
                            examinations: store.list_examinations().map_err(|e| e.to_string())?,
                        }),
                        None => Ok(DatabaseSnapshot::default()),
                    }
                })();
                let _ = tx.try_send(result);
            }
        });
        Self {
            actions,
            updates,
            thread: Some(thread),
        }
    }
}
impl DatabaseController {
    pub fn send(&self, action: DatabaseAction) -> Result<(), String> {
        self.actions
            .try_send(action)
            .map_err(|_| "Database busy; try again".into())
    }
    pub fn take_latest(&self) -> Option<Result<DatabaseSnapshot, String>> {
        self.updates.try_iter().last()
    }
}
impl Drop for DatabaseController {
    fn drop(&mut self) {
        // Pending database writes must complete before closing the connection.
        let _ = self.actions.send(DatabaseAction::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
