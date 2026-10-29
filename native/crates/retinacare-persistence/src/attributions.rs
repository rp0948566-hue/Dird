//! Phase 4: durable storage of RAW attribution maps inside the encrypted database (schema v4).
//!
//! The row keeps the typed metadata as opaque JSON (the AI crate owns its meaning), the raw map as a
//! little-endian f32 blob and its SHA-256. Identity columns (patient, eye, examination, run, image
//! hash) are checked on save against the examination and on load against the caller, and the blob hash
//! is re-computed on every load. Rendered colours are never stored. A locked database cannot be opened,
//! so nothing is readable without the key. Many results per examination are kept (history), newest
//! first, pruned to `MAX_PER_EXAMINATION`.

use crate::{SqlCipherStore, StoreError};
use rusqlite::params;
use sha2::{Digest, Sha256};

pub const MAX_PER_EXAMINATION: usize = 20;
/// 8x8 .. a 512x512 pixel map is the largest this table is meant to hold (4 MiB of f32).
const MAX_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;
const MAX_META_BYTES: usize = 64 * 1024;

pub(crate) const V4_SQL: &str = "
CREATE TABLE IF NOT EXISTS attribution_runs (
  id INTEGER PRIMARY KEY,
  artifact_id TEXT NOT NULL UNIQUE,
  examination_id INTEGER NOT NULL REFERENCES examinations(id) ON DELETE CASCADE,
  patient_id INTEGER NOT NULL,
  eye TEXT NOT NULL,
  run_id TEXT NOT NULL,
  image_sha256 TEXT NOT NULL,
  method TEXT NOT NULL,
  target TEXT NOT NULL,
  scope TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('ready', 'stale')),
  meta_json TEXT NOT NULL,
  payload_sha256 TEXT NOT NULL,
  payload BLOB NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS attribution_runs_exam_idx ON attribution_runs (examination_id, id DESC);
";

#[derive(Clone, Debug, PartialEq)]
pub struct AttributionInput {
    pub artifact_id: String,
    pub examination_id: i64,
    pub patient_id: i64,
    pub eye: String,
    pub run_id: String,
    pub image_sha256: String,
    pub method: String,
    pub target: String,
    pub scope: String,
    pub meta_json: String,
    pub payload: Vec<u8>,
    pub payload_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributionSummary {
    pub artifact_id: String,
    pub run_id: String,
    pub method: String,
    pub target: String,
    pub scope: String,
    pub status: String,
    pub created_at: String,
}

/// What the caller currently shows; every field must equal the stored row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributionExpectation {
    pub patient_id: i64,
    pub eye: String,
    pub image_sha256: String,
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl SqlCipherStore {
    /// Save one attribution atomically. Refused unless the examination exists with the same patient and
    /// eye, the digests are well formed and the payload hash matches the payload.
    pub fn save_attribution(&self, input: &AttributionInput) -> Result<i64, StoreError> {
        if input.examination_id <= 0
            || input.patient_id <= 0
            || !matches!(input.eye.as_str(), "" | "left" | "right")
            || !is_digest(&input.image_sha256)
            || !is_digest(&input.payload_sha256)
            || input.artifact_id.is_empty()
            || input.artifact_id.len() > 64
            || input.run_id.is_empty()
            || input.run_id.len() > 128
            || input.payload.is_empty()
            || input.payload.len() > MAX_PAYLOAD_BYTES
            || input.meta_json.len() > MAX_META_BYTES
            || [&input.method, &input.target, &input.scope]
                .iter()
                .any(|v| v.is_empty() || v.len() > 256)
        {
            return Err(StoreError::InvalidInput);
        }
        if sha256_hex(&input.payload) != input.payload_sha256.to_ascii_lowercase() {
            return Err(StoreError::AttributionIntegrity(
                "payload hash does not match".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let owner: Option<(i64, String)> = transaction
            .query_row(
                "SELECT patient_id, eye FROM examinations WHERE id = ?1",
                params![input.examination_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();
        match owner {
            Some((patient_id, eye)) if patient_id == input.patient_id && eye == input.eye => {}
            _ => return Err(StoreError::AttributionMismatch),
        }
        transaction.execute(
            "INSERT INTO attribution_runs (artifact_id, examination_id, patient_id, eye, run_id, image_sha256, method, target, scope, status, meta_json, payload_sha256, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'ready', ?10, ?11, ?12, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![
                input.artifact_id,
                input.examination_id,
                input.patient_id,
                input.eye,
                input.run_id,
                input.image_sha256.to_ascii_lowercase(),
                input.method,
                input.target,
                input.scope,
                input.meta_json,
                input.payload_sha256.to_ascii_lowercase(),
                input.payload
            ],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.execute(
            "DELETE FROM attribution_runs WHERE examination_id = ?1 AND id NOT IN (SELECT id FROM attribution_runs WHERE examination_id = ?1 ORDER BY id DESC LIMIT ?2)",
            params![input.examination_id, MAX_PER_EXAMINATION as i64],
        )?;
        transaction.commit()?;
        Ok(id)
    }

    /// Newest first. Summaries only: no payload is read, so listing cannot show a corrupted map.
    pub fn list_attributions(
        &self,
        examination_id: i64,
        patient_id: i64,
    ) -> Result<Vec<AttributionSummary>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT artifact_id, run_id, method, target, scope, status, created_at FROM attribution_runs WHERE examination_id = ?1 AND patient_id = ?2 ORDER BY id DESC",
        )?;
        let rows = statement.query_map(params![examination_id, patient_id], |row| {
            Ok(AttributionSummary {
                artifact_id: row.get(0)?,
                run_id: row.get(1)?,
                method: row.get(2)?,
                target: row.get(3)?,
                scope: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Load one result. Returns `(meta_json, payload, payload_sha256)` only when identity, status and the
    /// payload hash all check out; a `stale` row is refused (it belongs to an older analysis).
    pub fn load_attribution(
        &self,
        artifact_id: &str,
        expected: &AttributionExpectation,
    ) -> Result<Option<(String, Vec<u8>, String)>, StoreError> {
        let row = self.connection.query_row(
            "SELECT patient_id, eye, image_sha256, status, meta_json, payload_sha256, payload FROM attribution_runs WHERE artifact_id = ?1",
            params![artifact_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                ))
            },
        );
        let (patient_id, eye, image, status, meta, digest, payload) = match row {
            Ok(row) => row,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if patient_id != expected.patient_id
            || eye != expected.eye
            || image != expected.image_sha256.to_ascii_lowercase()
        {
            return Err(StoreError::AttributionMismatch);
        }
        if status != "ready" {
            return Err(StoreError::AttributionIntegrity("result is stale".into()));
        }
        if sha256_hex(&payload) != digest {
            return Err(StoreError::AttributionIntegrity(
                "payload hash does not match".into(),
            ));
        }
        Ok(Some((meta, payload, digest)))
    }

    /// Mark every stored result of an examination stale (called when the analysis is re-run).
    pub fn mark_attributions_stale(&self, examination_id: i64) -> Result<usize, StoreError> {
        Ok(self.connection.execute(
            "UPDATE attribution_runs SET status = 'stale' WHERE examination_id = ?1 AND status = 'ready'",
            params![examination_id],
        )?)
    }

    pub fn delete_attribution(&self, artifact_id: &str) -> Result<bool, StoreError> {
        Ok(self.connection.execute(
            "DELETE FROM attribution_runs WHERE artifact_id = ?1",
            params![artifact_id],
        )? > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "retinacare-attr-{name}-{}-{}/db.sqlite",
            std::process::id(),
            rand::random::<u64>()
        ))
    }

    fn setup(name: &str) -> (SqlCipherStore, PathBuf, i64, i64) {
        let path = temp_db(name);
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        let pid = store.add_patient("P-1", "Synthetic").unwrap();
        let eid = store.add_examination(pid, "op", "").unwrap();
        store
            .set_examination_image(eid, "right", "/x/y.png")
            .unwrap();
        (store, path, pid, eid)
    }

    fn input(pid: i64, eid: i64, artifact: &str) -> AttributionInput {
        let payload: Vec<u8> = (0..256u32)
            .flat_map(|v| (v as f32 - 100.0).to_le_bytes())
            .collect();
        AttributionInput {
            artifact_id: artifact.into(),
            examination_id: eid,
            patient_id: pid,
            eye: "right".into(),
            run_id: "run-1".into(),
            image_sha256: "ab".repeat(32),
            method: "occlusion_sensitivity".into(),
            target: "{\"kind\":\"stage3_severity\",\"grade\":3}".into(),
            scope: "{\"kind\":\"stage3_five_fold_ensemble\"}".into(),
            meta_json: "{\"synthetic\":true}".into(),
            payload_sha256: sha256_hex(&payload),
            payload,
        }
    }

    fn expectation(pid: i64) -> AttributionExpectation {
        AttributionExpectation {
            patient_id: pid,
            eye: "right".into(),
            image_sha256: "ab".repeat(32),
        }
    }

    // T31: the raw payload is restored bit-identically after the database is closed and reopened.
    #[test]
    fn raw_map_survives_a_restart_exactly() {
        let (store, path, pid, eid) = setup("restart");
        let saved = input(pid, eid, "a1");
        store.save_attribution(&saved).unwrap();
        drop(store);
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        let (meta, payload, digest) = store
            .load_attribution("a1", &expectation(pid))
            .unwrap()
            .unwrap();
        assert_eq!(
            (meta, payload, digest),
            (saved.meta_json, saved.payload, saved.payload_sha256)
        );
        assert_eq!(store.list_attributions(eid, pid).unwrap().len(), 1);
        assert!(
            store
                .load_attribution("missing", &expectation(pid))
                .unwrap()
                .is_none()
        );
    }

    // T33: without the password the database cannot be opened, so nothing can be shown.
    #[test]
    fn locked_database_exposes_nothing() {
        let (store, path, pid, eid) = setup("locked");
        store.save_attribution(&input(pid, eid, "a1")).unwrap();
        drop(store);
        assert!(SqlCipherStore::open(&path, "wrong").is_err());
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(8).any(|w| w == b"stage3_f"), "ciphertext only");
    }

    // T09/T10/T08: another patient, eye or image is refused on save and on load.
    #[test]
    fn identity_is_enforced_on_save_and_load() {
        let (store, _path, pid, eid) = setup("identity");
        let mut wrong_eye = input(pid, eid, "x");
        wrong_eye.eye = "left".into();
        assert!(matches!(
            store.save_attribution(&wrong_eye),
            Err(StoreError::AttributionMismatch)
        ));
        let mut wrong_patient = input(pid + 1, eid, "y");
        wrong_patient.patient_id = pid + 1;
        assert!(matches!(
            store.save_attribution(&wrong_patient),
            Err(StoreError::AttributionMismatch)
        ));
        store.save_attribution(&input(pid, eid, "ok")).unwrap();
        let mut other = expectation(pid);
        other.patient_id = pid + 1;
        assert!(matches!(
            store.load_attribution("ok", &other),
            Err(StoreError::AttributionMismatch)
        ));
        let mut other = expectation(pid);
        other.eye = "left".into();
        assert!(matches!(
            store.load_attribution("ok", &other),
            Err(StoreError::AttributionMismatch)
        ));
        let mut other = expectation(pid);
        other.image_sha256 = "cd".repeat(32);
        assert!(matches!(
            store.load_attribution("ok", &other),
            Err(StoreError::AttributionMismatch)
        ));
        assert!(store.list_attributions(eid, pid + 1).unwrap().is_empty());
    }

    // T32: a corrupted blob, a forged hash or a stale row is refused; nothing partial is saved.
    #[test]
    fn corrupted_forged_and_stale_results_are_rejected() {
        let (store, path, pid, eid) = setup("corrupt");
        let mut forged = input(pid, eid, "forged");
        forged.payload_sha256 = "00".repeat(32);
        assert!(matches!(
            store.save_attribution(&forged),
            Err(StoreError::AttributionIntegrity(_))
        ));
        assert!(store.list_attributions(eid, pid).unwrap().is_empty());
        store.save_attribution(&input(pid, eid, "a1")).unwrap();
        store
            .connection
            .execute(
                "UPDATE attribution_runs SET payload = zeroblob(1024) WHERE artifact_id = 'a1'",
                [],
            )
            .unwrap();
        assert!(matches!(
            store.load_attribution("a1", &expectation(pid)),
            Err(StoreError::AttributionIntegrity(_))
        ));
        store.save_attribution(&input(pid, eid, "a2")).unwrap();
        assert_eq!(store.mark_attributions_stale(eid).unwrap(), 2);
        assert!(matches!(
            store.load_attribution("a2", &expectation(pid)),
            Err(StoreError::AttributionIntegrity(_))
        ));
        let _ = path;
    }

    #[test]
    fn history_is_pruned_and_deleted_with_the_examination() {
        let (store, _path, pid, eid) = setup("prune");
        for i in 0..(MAX_PER_EXAMINATION + 3) {
            store
                .save_attribution(&input(pid, eid, &format!("a{i}")))
                .unwrap();
        }
        let list = store.list_attributions(eid, pid).unwrap();
        assert_eq!(list.len(), MAX_PER_EXAMINATION);
        assert_eq!(list[0].artifact_id, format!("a{}", MAX_PER_EXAMINATION + 2));
        store
            .connection
            .execute("DELETE FROM examinations WHERE id = ?1", params![eid])
            .unwrap();
        assert!(store.list_attributions(eid, pid).unwrap().is_empty());
        assert!(!store.delete_attribution("a0").unwrap());
    }

    #[test]
    fn duplicate_artifact_id_is_refused() {
        let (store, _path, pid, eid) = setup("dup");
        store.save_attribution(&input(pid, eid, "same")).unwrap();
        assert!(store.save_attribution(&input(pid, eid, "same")).is_err());
        assert_eq!(store.list_attributions(eid, pid).unwrap().len(), 1);
    }

    // Migration: a v3 database is backed up to `.pre-v4.bak`, keeps its data and gains the new table.
    #[test]
    fn v3_database_migrates_to_v4_with_backup_and_no_data_loss() {
        let (store, path, pid, _eid) = setup("migrate");
        store
            .connection
            .execute_batch("DROP TABLE attribution_runs; PRAGMA user_version = 3;")
            .unwrap();
        drop(store);
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        assert_eq!(store.schema_version().unwrap(), crate::SCHEMA_VERSION);
        assert_eq!(store.list_patients().unwrap().len(), 1);
        let exam = store.list_examinations().unwrap().remove(0);
        assert!(
            store
                .list_attributions(exam.id.unwrap(), pid)
                .unwrap()
                .is_empty()
        );
        let mut backup = path.as_os_str().to_os_string();
        backup.push(format!(".pre-v{}.bak", crate::SCHEMA_VERSION));
        assert!(PathBuf::from(backup).exists());
    }
}
