//! Encrypted local database with repository methods; no generic SQL API.

use argon2::{Algorithm, Argon2, Params, Version};
use rand::{RngCore, rngs::OsRng};
use retinacare_domain::Examination;
use rusqlite::{Connection, params};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const KDF_MEMORY_KIB: u32 = 65_536;
const KDF_ITERATIONS: u32 = 3;
const KDF_LANES: u32 = 4;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("encrypted database operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("key derivation failed: {0}")]
    Kdf(String),
    #[error("database salt is missing; refusing to create a new key for an existing database")]
    MissingSalt,
    #[error("encrypted database password was rejected")]
    WrongPassword,
    #[error("password rotation recovery data is invalid")]
    InvalidRotation,
    #[error("input is empty or exceeds the allowed length")]
    InvalidInput,
    #[error(
        "database key changed; close and reopen with the new password to recover salt metadata: {0}"
    )]
    SaltCommitAfterRekey(std::io::Error),
}

/// Current on-disk schema version (`PRAGMA user_version`). v2 adds the append-only audit ledger.
pub const SCHEMA_VERSION: i64 = 2;

/// One append-only audit record. Holds identifiers and statuses only, never image bytes or secrets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    pub id: i64,
    pub recorded_at: String,
    pub event_type: String,
    pub patient_id: Option<i64>,
    pub examination_id: Option<i64>,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientRecord {
    pub id: i64,
    pub patient_code: String,
    pub display_name: String,
    pub created_at: String,
}

pub struct SqlCipherStore {
    connection: Connection,
    database_path: PathBuf,
    salt_path: PathBuf,
}

impl SqlCipherStore {
    pub fn open(database_path: impl AsRef<Path>, password: &str) -> Result<Self, StoreError> {
        if password.is_empty() {
            return Err(StoreError::InvalidInput);
        }
        let database_path = database_path.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let salt_path = database_path.with_extension("salt");
        let next_path = rotation_path(&salt_path, "next");
        let previous_path = rotation_path(&salt_path, "previous");
        let recovering_path = rotation_path(&salt_path, "recovering");
        let mut candidates = Vec::new();
        for path in [&salt_path, &next_path, &previous_path, &recovering_path] {
            if path.exists() {
                match read_salt(path) {
                    Ok(salt) => candidates.push((path.clone(), salt)),
                    Err(StoreError::InvalidRotation) => continue,
                    Err(error) => return Err(error),
                }
            }
        }

        if candidates.is_empty() {
            if database_path.exists() {
                return Err(StoreError::MissingSalt);
            }
            let salt = random_salt();
            write_new_file(&salt_path, &salt)?;
            candidates.push((salt_path.clone(), salt.to_vec()));
        }

        let mut opened = None;
        for (candidate_path, salt) in &candidates {
            if let Ok(connection) = open_with_key(&database_path, password, salt) {
                opened = Some((connection, candidate_path.clone(), salt.to_vec()));
                break;
            }
        }
        let (connection, matched_path, matched_salt) = opened.ok_or(StoreError::WrongPassword)?;
        recover_salt_state(
            &salt_path,
            &matched_path,
            &matched_salt,
            &[next_path, previous_path, recovering_path],
        )?;
        let store = Self {
            connection,
            database_path,
            salt_path,
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn add_patient(&self, patient_code: &str, display_name: &str) -> Result<i64, StoreError> {
        if patient_code.trim().is_empty()
            || patient_code.len() > 64
            || display_name.trim().is_empty()
            || display_name.len() > 200
        {
            return Err(StoreError::InvalidInput);
        }
        self.connection.execute(
            "INSERT INTO patients (patient_code, display_name) VALUES (?1, ?2)",
            params![patient_code, display_name],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn list_patients(&self) -> Result<Vec<PatientRecord>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, patient_code, display_name, created_at FROM patients ORDER BY id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(PatientRecord {
                id: row.get(0)?,
                patient_code: row.get(1)?,
                display_name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn update_patient(
        &self,
        id: i64,
        patient_code: &str,
        display_name: &str,
    ) -> Result<(), StoreError> {
        if patient_code.trim().is_empty()
            || patient_code.len() > 64
            || display_name.trim().is_empty()
            || display_name.len() > 200
        {
            return Err(StoreError::InvalidInput);
        }
        if self.connection.execute(
            "UPDATE patients SET patient_code = ?1, display_name = ?2 WHERE id = ?3",
            params![patient_code, display_name, id],
        )? != 1
        {
            return Err(StoreError::InvalidInput);
        }
        Ok(())
    }

    pub fn change_password(&mut self, new_password: &str) -> Result<(), StoreError> {
        if new_password.is_empty() {
            return Err(StoreError::InvalidInput);
        }
        let mut salt = random_salt();
        let mut key = Zeroizing::new(derive_key(new_password, &salt)?);
        let hex = Zeroizing::new(to_hex(key.as_ref()));
        let next_path = rotation_path(&self.salt_path, "next");
        let previous_path = rotation_path(&self.salt_path, "previous");
        write_new_file(&next_path, &salt)?;
        if let Err(error) = self
            .connection
            .execute_batch(&format!("PRAGMA rekey = \"x'{}'\";", hex.as_str()))
        {
            let _ = fs::remove_file(&next_path);
            key.zeroize();
            salt.zeroize();
            return Err(StoreError::Sqlite(error));
        }

        // Keep both candidate salts until the selected one is durably installed.
        if self.salt_path.exists() && !previous_path.exists() {
            fs::rename(&self.salt_path, &previous_path)
                .map_err(StoreError::SaltCommitAfterRekey)?;
        }
        if let Err(error) = fs::rename(&next_path, &self.salt_path) {
            key.zeroize();
            salt.zeroize();
            return Err(StoreError::SaltCommitAfterRekey(error));
        }
        let _ = fs::remove_file(previous_path);
        key.zeroize();
        salt.zeroize();
        Ok(())
    }

    pub fn add_examination(
        &self,
        patient_id: i64,
        operator: &str,
        notes: &str,
    ) -> Result<i64, StoreError> {
        if operator.len() > 200 || notes.len() > 4000 {
            return Err(StoreError::InvalidInput);
        }
        self.add_screening_examination(patient_id, operator, notes, "", "", "")
    }

    pub fn add_screening_examination(
        &self,
        patient_id: i64,
        operator: &str,
        notes: &str,
        eye: &str,
        image_path: &str,
        analysis_json: &str,
    ) -> Result<i64, StoreError> {
        if operator.len() > 200
            || notes.len() > 4000
            || !matches!(eye, "" | "left" | "right")
            || image_path.len() > 4096
            || analysis_json.len() > 1_048_576
        {
            return Err(StoreError::InvalidInput);
        }
        self.connection.execute("INSERT INTO examinations (patient_id, examined_at, operator, notes, eye, image_path, analysis_json) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?2, ?3, ?4, ?5, ?6)", params![patient_id, operator, notes, eye, image_path, analysis_json])?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn list_examinations(&self) -> Result<Vec<Examination>, StoreError> {
        let mut statement = self.connection.prepare("SELECT id, patient_id, examined_at, operator, notes, eye, image_path, analysis_json FROM examinations ORDER BY id DESC")?;
        let rows = statement.query_map([], |row| {
            Ok(Examination {
                id: Some(row.get(0)?),
                patient_id: row.get(1)?,
                examined_at: row.get(2)?,
                operator: row.get(3)?,
                notes: row.get(4)?,
                eye: row.get(5)?,
                image_path: row.get(6)?,
                analysis_json: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Replace the stored analysis summary of one examination. The JSON must be a compact summary:
    /// the 1 MiB cap rejects raw mask or image payloads.
    pub fn set_examination_analysis(
        &self,
        examination_id: i64,
        analysis_json: &str,
    ) -> Result<(), StoreError> {
        if analysis_json.len() > 1_048_576 {
            return Err(StoreError::InvalidInput);
        }
        if self.connection.execute(
            "UPDATE examinations SET analysis_json = ?1 WHERE id = ?2",
            params![analysis_json, examination_id],
        )? != 1
        {
            return Err(StoreError::InvalidInput);
        }
        Ok(())
    }

    /// Attach the eye and source image path to an examination.
    pub fn set_examination_image(
        &self,
        examination_id: i64,
        eye: &str,
        image_path: &str,
    ) -> Result<(), StoreError> {
        if !matches!(eye, "" | "left" | "right") || image_path.len() > 4096 {
            return Err(StoreError::InvalidInput);
        }
        if self.connection.execute(
            "UPDATE examinations SET eye = ?1, image_path = ?2 WHERE id = ?3",
            params![eye, image_path, examination_id],
        )? != 1
        {
            return Err(StoreError::InvalidInput);
        }
        Ok(())
    }

    pub fn get_patient(&self, id: i64) -> Result<Option<PatientRecord>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, patient_code, display_name, created_at FROM patients WHERE id = ?1",
        )?;
        let mut rows = statement.query_map(params![id], |row| {
            Ok(PatientRecord {
                id: row.get(0)?,
                patient_code: row.get(1)?,
                display_name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Append one audit event. There is no update or delete API and database triggers reject both.
    pub fn append_audit_event(
        &self,
        event_type: &str,
        patient_id: Option<i64>,
        examination_id: Option<i64>,
        status: &str,
        detail: &str,
    ) -> Result<i64, StoreError> {
        if event_type.trim().is_empty()
            || event_type.len() > 64
            || status.len() > 64
            || detail.len() > 1024
        {
            return Err(StoreError::InvalidInput);
        }
        self.connection.execute(
            "INSERT INTO audit_events (recorded_at, event_type, patient_id, examination_id, status, detail) VALUES (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?1, ?2, ?3, ?4, ?5)",
            params![event_type, patient_id, examination_id, status, detail],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Newest first. `examination_id = None` returns every event.
    pub fn list_audit_events(
        &self,
        examination_id: Option<i64>,
    ) -> Result<Vec<AuditRecord>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, recorded_at, event_type, patient_id, examination_id, status, detail FROM audit_events WHERE (?1 IS NULL OR examination_id = ?1) ORDER BY id DESC",
        )?;
        let rows = statement.query_map(params![examination_id], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                recorded_at: row.get(1)?,
                event_type: row.get(2)?,
                patient_id: row.get(3)?,
                examination_id: row.get(4)?,
                status: row.get(5)?,
                detail: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    fn ensure_schema(&self) -> Result<(), StoreError> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS patients (
               id INTEGER PRIMARY KEY,
               patient_code TEXT NOT NULL UNIQUE,
               display_name TEXT NOT NULL,
               created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS examinations (
               id INTEGER PRIMARY KEY,
               patient_id INTEGER NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
               examined_at TEXT NOT NULL,
               operator TEXT NOT NULL DEFAULT '',
               notes TEXT NOT NULL DEFAULT '',
               eye TEXT NOT NULL DEFAULT '',
               image_path TEXT NOT NULL DEFAULT '',
               analysis_json TEXT NOT NULL DEFAULT ''
             );",
        )?;
        for (column, definition) in [
            ("eye", "TEXT NOT NULL DEFAULT ''"),
            ("image_path", "TEXT NOT NULL DEFAULT ''"),
            ("analysis_json", "TEXT NOT NULL DEFAULT ''"),
        ] {
            if !self.has_examination_column(column)? {
                self.connection.execute_batch(&format!(
                    "ALTER TABLE examinations ADD COLUMN {column} {definition}"
                ))?;
            }
        }
        self.migrate_to_current()
    }

    /// Additive, transactional migration. An existing database is copied to `<db>.pre-v<N>.bak`
    /// before any change so a failed or unwanted migration can be rolled back by restoring the copy.
    fn migrate_to_current(&self) -> Result<(), StoreError> {
        let version = self.schema_version()?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        let has_data = self
            .connection
            .query_row("SELECT count(*) FROM patients", [], |row| {
                row.get::<_, i64>(0)
            })?
            > 0;
        if has_data {
            let mut backup = self.database_path.as_os_str().to_os_string();
            backup.push(format!(".pre-v{SCHEMA_VERSION}.bak"));
            let backup = PathBuf::from(backup);
            if !backup.exists() {
                fs::copy(&self.database_path, &backup)?;
            }
        }
        let applied = self.connection.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS audit_events (
               id INTEGER PRIMARY KEY,
               recorded_at TEXT NOT NULL,
               event_type TEXT NOT NULL,
               patient_id INTEGER,
               examination_id INTEGER,
               status TEXT NOT NULL DEFAULT '',
               detail TEXT NOT NULL DEFAULT ''
             );
             CREATE TRIGGER IF NOT EXISTS audit_events_no_update BEFORE UPDATE ON audit_events
               BEGIN SELECT RAISE(ABORT, 'audit_events is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS audit_events_no_delete BEFORE DELETE ON audit_events
               BEGIN SELECT RAISE(ABORT, 'audit_events is append-only'); END;
             PRAGMA user_version = 2;
             COMMIT;",
        );
        if let Err(error) = applied {
            // A failed statement leaves the transaction open: undo it so nothing half-applied stays.
            let _ = self.connection.execute_batch("ROLLBACK");
            return Err(StoreError::Sqlite(error));
        }
        Ok(())
    }

    fn has_examination_column(&self, name: &str) -> Result<bool, StoreError> {
        let mut statement = self.connection.prepare("PRAGMA table_info(examinations)")?;
        let names = statement.query_map([], |row| row.get::<_, String>(1))?;
        Ok(names.filter_map(Result::ok).any(|column| column == name))
    }
}

fn open_with_key(path: &Path, password: &str, salt: &[u8]) -> Result<Connection, StoreError> {
    let mut key = Zeroizing::new(derive_key(password, salt)?);
    let hex = Zeroizing::new(to_hex(key.as_ref()));
    let connection = Connection::open(path)?;
    let result = connection
        .execute_batch(&format!("PRAGMA key = \"x'{}'\";", hex.as_str()))
        .and_then(|_| connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())));
    key.zeroize();
    result.map_err(|_| StoreError::WrongPassword)?;
    Ok(connection)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], StoreError> {
    let params = Params::new(KDF_MEMORY_KIB, KDF_ITERATIONS, KDF_LANES, Some(KEY_LEN))
        .map_err(|e| StoreError::Kdf(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0_u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| StoreError::Kdf(e.to_string()))?;
    Ok(key)
}

fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0_u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn rotation_path(salt_path: &Path, suffix: &str) -> PathBuf {
    let mut name = salt_path.as_os_str().to_os_string();
    name.push(format!(".{suffix}"));
    PathBuf::from(name)
}

fn read_salt(path: &Path) -> Result<Vec<u8>, StoreError> {
    let salt = fs::read(path)?;
    if salt.len() != SALT_LEN {
        return Err(StoreError::InvalidRotation);
    }
    Ok(salt)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn recover_salt_state(
    salt_path: &Path,
    matched_path: &Path,
    matched_salt: &[u8],
    recovery_paths: &[PathBuf],
) -> Result<(), StoreError> {
    if matched_path != salt_path {
        let temp_path = rotation_path(salt_path, "recovering");
        if temp_path.exists() {
            fs::remove_file(&temp_path)?;
        }
        write_new_file(&temp_path, matched_salt)?;
        if salt_path.exists() {
            fs::remove_file(salt_path)?;
        }
        fs::rename(temp_path, salt_path)?;
    }
    for path in recovery_paths {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "retinacare-{name}-{}-{}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn cleanup(path: &Path) {
        let salt = path.with_extension("salt");
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(&salt);
        let _ = fs::remove_file(rotation_path(&salt, "next"));
        let _ = fs::remove_file(rotation_path(&salt, "previous"));
        let _ = fs::remove_file(rotation_path(&salt, "recovering"));
    }

    #[test]
    fn repositories_persist_only_inside_encrypted_database() {
        let path = test_path("patient");
        {
            let store = SqlCipherStore::open(&path, "correct horse battery").unwrap();
            store.add_patient("P-001", "Example Patient").unwrap();
            assert_eq!(store.list_patients().unwrap().len(), 1);
        }
        assert!(SqlCipherStore::open(&path, "wrong password").is_err());
        cleanup(&path);
    }

    #[test]
    fn password_rotation_updates_the_key_and_keeps_records() {
        let path = test_path("rotation-success");
        let mut store = SqlCipherStore::open(&path, "old password").unwrap();
        store.add_patient("P-002", "Example Patient").unwrap();
        store.change_password("new password").unwrap();
        drop(store);
        let reopened = SqlCipherStore::open(&path, "new password").unwrap();
        assert_eq!(reopened.list_patients().unwrap().len(), 1);
        assert!(SqlCipherStore::open(&path, "old password").is_err());
        drop(reopened);
        cleanup(&path);
    }

    #[test]
    fn password_rotation_recovers_when_new_salt_was_staged_before_rekey() {
        let path = test_path("rotation-old");
        let store = SqlCipherStore::open(&path, "old password").unwrap();
        let next_path = rotation_path(&store.salt_path, "next");
        write_new_file(&next_path, &random_salt()).unwrap();
        drop(store);
        let recovered = SqlCipherStore::open(&path, "old password").unwrap();
        assert!(recovered.list_patients().unwrap().is_empty());
        assert!(!next_path.exists());
        drop(recovered);
        cleanup(&path);
    }

    #[test]
    fn password_rotation_recovers_new_key_when_salt_commit_was_interrupted() {
        let path = test_path("rotation-new");
        let store = SqlCipherStore::open(&path, "old password").unwrap();
        store.add_patient("P-001", "Example Patient").unwrap();
        let mut new_salt = random_salt();
        let next_path = rotation_path(&store.salt_path, "next");
        write_new_file(&next_path, &new_salt).unwrap();
        let key = derive_key("new password", &new_salt).unwrap();
        store
            .connection
            .execute_batch(&format!("PRAGMA rekey = \"x'{}'\";", to_hex(&key)))
            .unwrap();
        new_salt.zeroize();
        drop(store);

        let recovered = SqlCipherStore::open(&path, "new password").unwrap();
        assert_eq!(recovered.list_patients().unwrap().len(), 1);
        assert!(SqlCipherStore::open(&path, "old password").is_err());
        assert!(!next_path.exists());
        drop(recovered);
        cleanup(&path);
    }
}

#[cfg(test)]
mod production_tests {
    use super::*;
    #[test]
    fn recovery_file_and_unicode_path_preserve_examinations() {
        let dir = std::env::temp_dir().join(format!(
            "RetinaCare space 患者-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let path = dir.join("records.sqlite");
        let store = SqlCipherStore::open(&path, "passphrase").unwrap();
        let id = store.add_patient("P-test", "患者").unwrap();
        store
            .update_patient(id, "P-edited", "患者 Updated")
            .unwrap();
        store
            .add_screening_examination(
                id,
                "operator",
                "review pending",
                "left",
                "/tmp/fundus.png",
                r#"{"status":"error","error":{"code":"no_trained_weights"}}"#,
            )
            .unwrap();
        assert!(store.add_examination(id + 1, "", "").is_err());
        assert!(store.add_patient("", "").is_err());
        drop(store);
        let salt = path.with_extension("salt");
        fs::rename(&salt, rotation_path(&salt, "recovering")).unwrap();
        let reopened = SqlCipherStore::open(&path, "passphrase").unwrap();
        assert_eq!(
            reopened.list_patients().unwrap()[0].patient_code,
            "P-edited"
        );
        let examinations = reopened.list_examinations().unwrap();
        assert_eq!(examinations[0].eye, "left");
        assert_eq!(examinations[0].image_path, "/tmp/fundus.png");
        assert!(examinations[0].analysis_json.contains("no_trained_weights"));
        drop(reopened);
        let bytes = fs::read(&path).unwrap();
        assert_ne!(&bytes[..16], b"SQLite format 3\0");
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "retinacare-int-{name}-{}-{}/db.sqlite",
            std::process::id(),
            rand::random::<u64>()
        ))
    }

    #[test]
    fn analysis_summary_and_image_survive_reopen() {
        let path = temp_db("analysis");
        let (pid, eid) = {
            let store = SqlCipherStore::open(&path, "pw").unwrap();
            let pid = store.add_patient("P-1", "Name").unwrap();
            let eid = store.add_examination(pid, "op", "n").unwrap();
            store
                .set_examination_image(eid, "right", "/x/y.png")
                .unwrap();
            store
                .set_examination_analysis(eid, r#"{"state":"graded"}"#)
                .unwrap();
            assert!(store.set_examination_image(eid, "up", "").is_err());
            assert!(store.set_examination_analysis(eid + 99, "{}").is_err());
            assert_eq!(store.get_patient(pid).unwrap().unwrap().patient_code, "P-1");
            assert!(store.get_patient(pid + 1).unwrap().is_none());
            (pid, eid)
        };
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        let exam = store.list_examinations().unwrap().remove(0);
        assert_eq!((exam.id, exam.patient_id), (Some(eid), pid));
        assert_eq!(exam.eye, "right");
        assert_eq!(exam.analysis_json, r#"{"state":"graded"}"#);
    }

    #[test]
    fn audit_ledger_is_append_only_and_scoped() {
        let path = temp_db("audit");
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        let pid = store.add_patient("P-2", "Name").unwrap();
        let a = store.add_examination(pid, "", "").unwrap();
        let b = store.add_examination(pid, "", "").unwrap();
        store
            .append_audit_event("analysis_finished", Some(pid), Some(a), "graded", "req 1")
            .unwrap();
        store
            .append_audit_event("report_exported", Some(pid), Some(b), "ok", "")
            .unwrap();
        assert_eq!(store.list_audit_events(None).unwrap().len(), 2);
        let only_a = store.list_audit_events(Some(a)).unwrap();
        assert_eq!(only_a.len(), 1);
        assert_eq!(only_a[0].event_type, "analysis_finished");
        assert!(store.append_audit_event("", None, None, "", "").is_err());
        for sql in [
            "UPDATE audit_events SET status = 'x'",
            "DELETE FROM audit_events",
        ] {
            assert!(store.connection.execute(sql, []).is_err(), "{sql}");
        }
        assert_eq!(store.list_audit_events(None).unwrap().len(), 2);
    }

    #[test]
    fn v1_database_is_backed_up_then_migrated_without_data_loss() {
        let path = temp_db("migrate");
        {
            let store = SqlCipherStore::open(&path, "pw").unwrap();
            store.add_patient("P-3", "Old").unwrap();
            store
                .connection
                .execute_batch("DROP TABLE audit_events; PRAGMA user_version = 0;")
                .unwrap();
        }
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(store.list_patients().unwrap().len(), 1);
        let mut backup = path.as_os_str().to_os_string();
        backup.push(format!(".pre-v{SCHEMA_VERSION}.bak"));
        assert!(PathBuf::from(backup).exists());
        drop(store);
        assert!(SqlCipherStore::open(&path, "wrong").is_err());
    }
}

#[cfg(test)]
mod migration_failure_tests {
    use super::*;

    #[test]
    fn a_failed_migration_rolls_back_keeps_data_and_leaves_a_backup() {
        let path = std::env::temp_dir().join(format!(
            "retinacare-migfail-{}-{}/db.sqlite",
            std::process::id(),
            rand::random::<u64>()
        ));
        {
            let store = SqlCipherStore::open(&path, "pw").unwrap();
            store.add_patient("P-9", "Keep Me").unwrap();
            // Force the v2 migration to fail: an object named audit_events that is not a table.
            store
                .connection
                .execute_batch(
                    "DROP TABLE audit_events; CREATE VIEW audit_events AS SELECT 1 AS id; PRAGMA user_version = 0;",
                )
                .unwrap();
        }
        let key_probe = SqlCipherStore::open(&path, "pw");
        assert!(
            key_probe.is_err(),
            "the migration failure must be reported, not hidden"
        );
        // Re-open the raw connection: data intact, version unchanged, backup present.
        let salt = fs::read(path.with_extension("salt")).unwrap();
        let connection = open_with_key(&path, "pw", &salt).unwrap();
        let count: i64 = connection
            .query_row("SELECT count(*) FROM patients", [], |row| row.get(0))
            .unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!((count, version), (1, 0));
        assert!(
            connection.is_autocommit(),
            "no transaction may be left open"
        );
        let mut backup = path.as_os_str().to_os_string();
        backup.push(format!(".pre-v{SCHEMA_VERSION}.bak"));
        assert!(PathBuf::from(backup).exists());
    }
}
