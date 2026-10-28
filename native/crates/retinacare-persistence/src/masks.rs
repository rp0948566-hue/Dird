//! Durable, versioned storage of the five Stage II masks inside the encrypted database.
//!
//! One run per examination (`mask_runs`) plus exactly five artifacts (`mask_artifacts`) are written in
//! ONE transaction, so a crash or failure leaves either the complete set or nothing. Masks live in the
//! SQLCipher file, so they share its encryption, its unlock state and its deletion lifecycle
//! (`ON DELETE CASCADE` from the examination). Loading re-checks every payload hash, the manifest hash,
//! completeness and the caller's patient/eye/image identity before any pixel is returned.

use crate::{SqlCipherStore, StoreError};
use rusqlite::params;
use sha2::{Digest, Sha256};

pub const MASK_FORMAT_VERSION: i64 = 1;
/// Fixed order: it matches the Stage III eight-channel input (MA, HE, EX, CWS, VESSEL).
pub const MASK_KINDS: [&str; 5] = ["ma", "he", "ex", "cws", "vessel"];
/// Same decoded-pixel ceiling as image intake, so a mask can never be bigger than its image.
const MAX_MASK_PIXELS: u64 = 40_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct StoredMask {
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub model_version: String,
    pub model_sha256: String,
    pub checkpoint_sha256: String,
    pub preprocessing_version: String,
    pub threshold: f32,
    pub provider: String,
    pub elapsed_microseconds: u64,
    /// Opaque JSON of the registered spatial transform, kept for provenance only.
    pub transform_json: String,
    /// `width * height` bytes, each 0 or 255, in the original image frame.
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskRunInput {
    pub examination_id: i64,
    pub patient_id: i64,
    /// Database spelling: `left`, `right` or empty.
    pub eye: String,
    pub request_id: u64,
    pub source_image_sha256: String,
    pub image_width: u32,
    pub image_height: u32,
    pub masks: Vec<StoredMask>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoredMaskRun {
    pub run_id: i64,
    pub created_at: String,
    pub input: MaskRunInput,
}

/// What the caller already knows about the examination; every field must match the stored run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaskExpectation {
    pub patient_id: i64,
    pub eye: String,
    pub source_image_sha256: Option<String>,
}

pub(crate) const V3_SQL: &str = "
CREATE TABLE IF NOT EXISTS mask_runs (
  id INTEGER PRIMARY KEY,
  examination_id INTEGER NOT NULL UNIQUE REFERENCES examinations(id) ON DELETE CASCADE,
  patient_id INTEGER NOT NULL,
  eye TEXT NOT NULL,
  request_id INTEGER NOT NULL,
  source_image_sha256 TEXT NOT NULL,
  image_width INTEGER NOT NULL,
  image_height INTEGER NOT NULL,
  manifest_sha256 TEXT NOT NULL,
  format_version INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  complete INTEGER NOT NULL CHECK (complete = 1)
);
CREATE TABLE IF NOT EXISTS mask_artifacts (
  run_id INTEGER NOT NULL REFERENCES mask_runs(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  model_version TEXT NOT NULL,
  model_sha256 TEXT NOT NULL,
  checkpoint_sha256 TEXT NOT NULL,
  preprocessing_version TEXT NOT NULL,
  threshold REAL NOT NULL,
  provider TEXT NOT NULL,
  elapsed_microseconds INTEGER NOT NULL,
  transform_json TEXT NOT NULL,
  foreground_pixels INTEGER NOT NULL,
  payload_sha256 TEXT NOT NULL,
  payload BLOB NOT NULL,
  PRIMARY KEY (run_id, kind)
);
";

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Run-length codec for 0/255 masks: u32 little-endian run lengths, alternating 0 then 255, starting
/// with a (possibly zero-length) background run. Lossless; a typical lesion mask shrinks by >99 %.
pub fn encode_mask(pixels: &[u8]) -> Result<Vec<u8>, StoreError> {
    let mut out = Vec::new();
    let mut value = 0_u8;
    let mut run = 0_u32;
    for &pixel in pixels {
        if pixel != 0 && pixel != 255 {
            return Err(StoreError::InvalidInput);
        }
        if pixel == value {
            run += 1;
        } else {
            out.extend_from_slice(&run.to_le_bytes());
            value = pixel;
            run = 1;
        }
    }
    out.extend_from_slice(&run.to_le_bytes());
    Ok(out)
}

pub fn decode_mask(payload: &[u8], expected_pixels: usize) -> Result<Vec<u8>, StoreError> {
    let integrity = || StoreError::MaskIntegrity("mask payload is malformed".to_string());
    if payload.is_empty() || !payload.len().is_multiple_of(4) {
        return Err(integrity());
    }
    let mut pixels = Vec::with_capacity(expected_pixels);
    let mut value = 0_u8;
    for chunk in payload.as_chunks::<4>().0 {
        let run = u32::from_le_bytes(*chunk) as usize;
        if pixels
            .len()
            .checked_add(run)
            .is_none_or(|n| n > expected_pixels)
        {
            return Err(integrity());
        }
        pixels.resize(pixels.len() + run, value);
        value = if value == 0 { 255 } else { 0 };
    }
    if pixels.len() != expected_pixels {
        return Err(integrity());
    }
    Ok(pixels)
}

struct ArtifactDigest<'a> {
    kind: &'a str,
    payload_sha256: &'a str,
    model_sha256: &'a str,
    checkpoint_sha256: &'a str,
    model_version: &'a str,
    width: u32,
    height: u32,
}

fn manifest_hash(run: &MaskRunInput, artifacts: &[ArtifactDigest]) -> String {
    let mut text = format!(
        "v{MASK_FORMAT_VERSION}|{}|{}|{}|{}|{}|{}|{}",
        run.examination_id,
        run.patient_id,
        run.eye,
        run.request_id,
        run.source_image_sha256,
        run.image_width,
        run.image_height
    );
    for a in artifacts {
        text.push_str(&format!(
            "|{}:{}:{}:{}:{}:{}x{}",
            a.kind,
            a.payload_sha256,
            a.model_sha256,
            a.checkpoint_sha256,
            a.model_version,
            a.width,
            a.height
        ));
    }
    sha256_hex(text.as_bytes())
}

fn validate(run: &MaskRunInput) -> Result<(), StoreError> {
    let pixels = u64::from(run.image_width) * u64::from(run.image_height);
    if run.examination_id <= 0
        || run.patient_id <= 0
        || !matches!(run.eye.as_str(), "" | "left" | "right")
        || !is_digest(&run.source_image_sha256)
        || run.image_width == 0
        || run.image_height == 0
        || pixels > MAX_MASK_PIXELS
        || run.masks.len() != MASK_KINDS.len()
    {
        return Err(StoreError::InvalidInput);
    }
    for kind in MASK_KINDS {
        let mut found = run.masks.iter().filter(|m| m.kind == kind);
        let Some(mask) = found.next() else {
            return Err(StoreError::InvalidInput);
        };
        if found.next().is_some()
            || mask.width != run.image_width
            || mask.height != run.image_height
            || mask.pixels.len() as u64 != pixels
            || !is_digest(&mask.model_sha256)
            || !is_digest(&mask.checkpoint_sha256)
            || mask.model_version.trim().is_empty()
            || mask.model_version.len() > 128
            || mask.preprocessing_version.len() > 128
            || mask.provider.len() > 64
            || mask.transform_json.len() > 4096
            || !mask.threshold.is_finite()
        {
            return Err(StoreError::InvalidInput);
        }
    }
    Ok(())
}

impl SqlCipherStore {
    /// Save (or atomically replace) the five masks of one examination. Nothing is stored unless the
    /// whole set is valid and the examination exists with the same patient and eye.
    pub fn save_mask_run(&self, run: &MaskRunInput) -> Result<i64, StoreError> {
        validate(run)?;
        let mut encoded = Vec::new();
        for kind in MASK_KINDS {
            let mask = run
                .masks
                .iter()
                .find(|m| m.kind == kind)
                .expect("validated");
            let payload = encode_mask(&mask.pixels)?;
            let digest = sha256_hex(&payload);
            let foreground = mask.pixels.iter().filter(|p| **p == 255).count() as i64;
            encoded.push((mask, payload, digest, foreground));
        }
        let artifacts: Vec<ArtifactDigest> = encoded
            .iter()
            .map(|(mask, _, digest, _)| ArtifactDigest {
                kind: &mask.kind,
                payload_sha256: digest,
                model_sha256: &mask.model_sha256,
                checkpoint_sha256: &mask.checkpoint_sha256,
                model_version: &mask.model_version,
                width: mask.width,
                height: mask.height,
            })
            .collect();
        let manifest = manifest_hash(run, &artifacts);

        let transaction = self.connection.unchecked_transaction()?;
        let owner: Option<(i64, String)> = transaction
            .query_row(
                "SELECT patient_id, eye FROM examinations WHERE id = ?1",
                params![run.examination_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();
        match owner {
            Some((patient_id, eye)) if patient_id == run.patient_id && eye == run.eye => {}
            _ => return Err(StoreError::MaskMismatch),
        }
        transaction.execute(
            "DELETE FROM mask_runs WHERE examination_id = ?1",
            params![run.examination_id],
        )?;
        transaction.execute(
            "INSERT INTO mask_runs (examination_id, patient_id, eye, request_id, source_image_sha256, image_width, image_height, manifest_sha256, format_version, created_at, complete) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 1)",
            params![
                run.examination_id,
                run.patient_id,
                run.eye,
                run.request_id as i64,
                run.source_image_sha256,
                run.image_width,
                run.image_height,
                manifest,
                MASK_FORMAT_VERSION
            ],
        )?;
        let run_id = transaction.last_insert_rowid();
        for (mask, payload, digest, foreground) in &encoded {
            transaction.execute(
                "INSERT INTO mask_artifacts (run_id, kind, width, height, model_version, model_sha256, checkpoint_sha256, preprocessing_version, threshold, provider, elapsed_microseconds, transform_json, foreground_pixels, payload_sha256, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    run_id,
                    mask.kind,
                    mask.width,
                    mask.height,
                    mask.model_version,
                    mask.model_sha256,
                    mask.checkpoint_sha256,
                    mask.preprocessing_version,
                    f64::from(mask.threshold),
                    mask.provider,
                    mask.elapsed_microseconds as i64,
                    mask.transform_json,
                    foreground,
                    digest,
                    payload
                ],
            )?;
        }
        transaction.commit()?;
        Ok(run_id)
    }

    /// `Ok(None)` when the examination has no stored masks (legacy or never analysed).
    /// `Err(MaskMismatch)` when the run belongs to another patient, eye or image, and
    /// `Err(MaskIntegrity)` when any hash, count or size check fails: pixels are never returned then.
    pub fn load_mask_run(
        &self,
        examination_id: i64,
        expected: &MaskExpectation,
    ) -> Result<Option<StoredMaskRun>, StoreError> {
        let header = self.connection.query_row(
            "SELECT id, patient_id, eye, request_id, source_image_sha256, image_width, image_height, manifest_sha256, format_version, created_at FROM mask_runs WHERE examination_id = ?1",
            params![examination_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, u32>(5)?,
                    row.get::<_, u32>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        );
        let (
            run_id,
            patient_id,
            eye,
            request_id,
            source_sha,
            width,
            height,
            stored_manifest,
            version,
            created_at,
        ) = match header {
            Ok(h) => h,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if patient_id != expected.patient_id
            || eye != expected.eye
            || expected
                .source_image_sha256
                .as_ref()
                .is_some_and(|sha| !sha.eq_ignore_ascii_case(&source_sha))
        {
            return Err(StoreError::MaskMismatch);
        }
        let integrity = |text: &str| StoreError::MaskIntegrity(text.to_string());
        if version != MASK_FORMAT_VERSION {
            return Err(integrity("unsupported mask format version"));
        }
        let mut statement = self.connection.prepare(
            "SELECT kind, width, height, model_version, model_sha256, checkpoint_sha256, preprocessing_version, threshold, provider, elapsed_microseconds, transform_json, foreground_pixels, payload_sha256, payload FROM mask_artifacts WHERE run_id = ?1",
        )?;
        let rows = statement
            .query_map(params![run_id], |row| {
                Ok((
                    StoredMask {
                        kind: row.get(0)?,
                        width: row.get(1)?,
                        height: row.get(2)?,
                        model_version: row.get(3)?,
                        model_sha256: row.get(4)?,
                        checkpoint_sha256: row.get(5)?,
                        preprocessing_version: row.get(6)?,
                        threshold: row.get::<_, f64>(7)? as f32,
                        provider: row.get(8)?,
                        elapsed_microseconds: row.get::<_, i64>(9)? as u64,
                        transform_json: row.get(10)?,
                        pixels: Vec::new(),
                    },
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Vec<u8>>(13)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if rows.len() != MASK_KINDS.len() {
            return Err(integrity("mask set is incomplete"));
        }
        let expected_pixels = (u64::from(width) * u64::from(height)) as usize;
        let mut masks = Vec::new();
        for kind in MASK_KINDS {
            let (mut mask, foreground, digest, payload) = rows
                .iter()
                .find(|(m, ..)| m.kind == kind)
                .cloned()
                .ok_or_else(|| integrity("mask set is incomplete"))?;
            if sha256_hex(&payload) != digest {
                return Err(integrity("mask checksum mismatch"));
            }
            if mask.width != width || mask.height != height {
                return Err(integrity("mask size differs from the image"));
            }
            mask.pixels = decode_mask(&payload, expected_pixels)?;
            if mask.pixels.iter().filter(|p| **p == 255).count() as i64 != foreground {
                return Err(integrity("mask foreground count mismatch"));
            }
            masks.push((mask, digest));
        }
        let input = MaskRunInput {
            examination_id,
            patient_id,
            eye,
            request_id: request_id as u64,
            source_image_sha256: source_sha,
            image_width: width,
            image_height: height,
            masks: masks.iter().map(|(m, _)| m.clone()).collect(),
        };
        let artifacts: Vec<ArtifactDigest> = masks
            .iter()
            .map(|(mask, digest)| ArtifactDigest {
                kind: &mask.kind,
                payload_sha256: digest,
                model_sha256: &mask.model_sha256,
                checkpoint_sha256: &mask.checkpoint_sha256,
                model_version: &mask.model_version,
                width: mask.width,
                height: mask.height,
            })
            .collect();
        if manifest_hash(&input, &artifacts) != stored_manifest {
            return Err(integrity("mask manifest mismatch"));
        }
        Ok(Some(StoredMaskRun {
            run_id,
            created_at,
            input,
        }))
    }

    /// Cheap check used by lists and badges; does not decode or verify pixels.
    pub fn has_mask_run(&self, examination_id: i64) -> Result<bool, StoreError> {
        Ok(self.connection.query_row(
            "SELECT count(*) FROM mask_runs WHERE examination_id = ?1",
            params![examination_id],
            |row| row.get::<_, i64>(0),
        )? > 0)
    }

    pub fn delete_mask_run(&self, examination_id: i64) -> Result<(), StoreError> {
        self.connection.execute(
            "DELETE FROM mask_runs WHERE examination_id = ?1",
            params![examination_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SCHEMA_VERSION;
    use std::path::PathBuf;

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "retinacare-mask-{name}-{}-{}/db.sqlite",
            std::process::id(),
            rand::random::<u64>()
        ))
    }

    fn digest(seed: u8) -> String {
        format!("{seed:02x}").repeat(32)
    }

    /// Synthetic test pattern (never a clinical image): sparse MA, empty HE, a block for the rest.
    fn masks(width: u32, height: u32) -> Vec<StoredMask> {
        let total = (width * height) as usize;
        MASK_KINDS
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let mut pixels = vec![0_u8; total];
                match *kind {
                    "he" => {}
                    "ma" => pixels[3] = 255,
                    _ => {
                        for p in pixels.iter_mut().skip(i * 7).step_by(5) {
                            *p = 255;
                        }
                    }
                }
                StoredMask {
                    kind: (*kind).to_string(),
                    width,
                    height,
                    model_version: format!("test-{kind}-v1"),
                    model_sha256: digest(i as u8 + 1),
                    checkpoint_sha256: digest(i as u8 + 21),
                    preprocessing_version: "pre-v1".to_string(),
                    threshold: 0.5,
                    provider: "cpu".to_string(),
                    elapsed_microseconds: 1234,
                    transform_json: r#"{"kind":"test"}"#.to_string(),
                    pixels,
                }
            })
            .collect()
    }

    fn setup(name: &str) -> (SqlCipherStore, PathBuf, i64, i64, MaskRunInput) {
        let path = temp_db(name);
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        let pid = store.add_patient("P-1", "Synthetic").unwrap();
        let eid = store.add_examination(pid, "op", "").unwrap();
        store
            .set_examination_image(eid, "right", "/x/y.png")
            .unwrap();
        let run = MaskRunInput {
            examination_id: eid,
            patient_id: pid,
            eye: "right".to_string(),
            request_id: 7,
            source_image_sha256: digest(99),
            image_width: 16,
            image_height: 12,
            masks: masks(16, 12),
        };
        (store, path, pid, eid, run)
    }

    fn expect(pid: i64) -> MaskExpectation {
        MaskExpectation {
            patient_id: pid,
            eye: "right".to_string(),
            source_image_sha256: Some(digest(99)),
        }
    }

    #[test]
    fn codec_round_trips_and_rejects_bad_input() {
        for pixels in [
            vec![0_u8; 10],
            vec![255_u8; 10],
            vec![255, 0, 0, 255, 255, 0],
            vec![0, 255],
        ] {
            let encoded = encode_mask(&pixels).unwrap();
            assert_eq!(decode_mask(&encoded, pixels.len()).unwrap(), pixels);
        }
        assert!(encode_mask(&[0, 1]).is_err());
        let good = encode_mask(&[0, 0, 255]).unwrap();
        assert!(decode_mask(&good, 4).is_err(), "too few pixels");
        assert!(decode_mask(&good, 2).is_err(), "too many pixels");
        assert!(decode_mask(&[], 0).is_err());
        assert!(decode_mask(&[1, 2, 3], 3).is_err());
    }

    #[test]
    fn five_masks_round_trip_exactly_across_a_restart() {
        let (store, path, pid, eid, run) = setup("roundtrip");
        store.save_mask_run(&run).unwrap();
        assert!(store.has_mask_run(eid).unwrap());
        drop(store);
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        let loaded = store.load_mask_run(eid, &expect(pid)).unwrap().unwrap();
        assert_eq!(
            loaded.input, run,
            "pixels, hashes and metadata must be identical"
        );
        assert!(
            loaded.input.masks[1].pixels.iter().all(|p| *p == 0),
            "empty HE stays empty"
        );
        assert_eq!(
            loaded.input.masks[0]
                .pixels
                .iter()
                .filter(|p| **p == 255)
                .count(),
            1,
            "sparse MA keeps its single pixel"
        );
    }

    #[test]
    fn exam_without_masks_is_legacy_none_and_wrong_password_cannot_read() {
        let (store, path, pid, eid, _) = setup("legacy");
        assert!(store.load_mask_run(eid, &expect(pid)).unwrap().is_none());
        assert!(!store.has_mask_run(eid).unwrap());
        drop(store);
        assert!(SqlCipherStore::open(&path, "wrong").is_err());
    }

    #[test]
    fn corrupted_payload_manifest_or_missing_artifact_is_detected_and_no_pixels_returned() {
        for (name, sql) in [
            (
                "payload",
                "UPDATE mask_artifacts SET payload = X'0100000001000000' WHERE kind = 'ex'",
            ),
            ("manifest", "UPDATE mask_runs SET manifest_sha256 = 'x'"),
            (
                "count",
                "UPDATE mask_artifacts SET foreground_pixels = foreground_pixels + 1 WHERE kind = 'cws'",
            ),
            (
                "missing",
                "DELETE FROM mask_artifacts WHERE kind = 'vessel'",
            ),
            ("version", "UPDATE mask_runs SET format_version = 99"),
        ] {
            let (store, _, pid, eid, run) = setup(name);
            store.save_mask_run(&run).unwrap();
            store.connection.execute(sql, []).unwrap();
            assert!(
                matches!(
                    store.load_mask_run(eid, &expect(pid)),
                    Err(StoreError::MaskIntegrity(_))
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn tampered_artifact_metadata_breaks_the_manifest_hash() {
        let (store, _, pid, eid, run) = setup("meta");
        store.save_mask_run(&run).unwrap();
        store
            .connection
            .execute(
                "UPDATE mask_artifacts SET model_sha256 = ?1 WHERE kind = 'ma'",
                params![digest(77)],
            )
            .unwrap();
        assert!(matches!(
            store.load_mask_run(eid, &expect(pid)),
            Err(StoreError::MaskIntegrity(_))
        ));
    }

    #[test]
    fn other_patient_eye_or_image_is_refused() {
        let (store, _, pid, eid, run) = setup("isolation");
        store.save_mask_run(&run).unwrap();
        let other = store.add_patient("P-2", "Other").unwrap();
        let mut wrong_patient = expect(pid);
        wrong_patient.patient_id = other;
        let mut wrong_eye = expect(pid);
        wrong_eye.eye = "left".to_string();
        let mut resized = expect(pid);
        resized.source_image_sha256 = Some(digest(5));
        for e in [wrong_patient, wrong_eye, resized] {
            assert!(matches!(
                store.load_mask_run(eid, &e),
                Err(StoreError::MaskMismatch)
            ));
        }
        // Saving for an examination that belongs to another patient or eye is refused too.
        let mut theirs = run.clone();
        theirs.patient_id = other;
        assert!(matches!(
            store.save_mask_run(&theirs),
            Err(StoreError::MaskMismatch)
        ));
        let mut left = run.clone();
        left.eye = "left".to_string();
        assert!(matches!(
            store.save_mask_run(&left),
            Err(StoreError::MaskMismatch)
        ));
    }

    #[test]
    fn failure_before_commit_leaves_no_partial_set_and_keeps_the_previous_run() {
        let (store, _, pid, eid, run) = setup("crash");
        store.save_mask_run(&run).unwrap();
        // Fail on the fifth artifact insert of the replacement run: simulates a crash mid-commit.
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER inject_failure BEFORE INSERT ON mask_artifacts
                 WHEN NEW.kind = 'vessel' BEGIN SELECT RAISE(ABORT, 'injected'); END;",
            )
            .unwrap();
        let mut second = run.clone();
        second.request_id = 8;
        for m in &mut second.masks {
            m.pixels[0] = 255;
        }
        assert!(store.save_mask_run(&second).is_err());
        store
            .connection
            .execute_batch("DROP TRIGGER inject_failure")
            .unwrap();
        let kept = store.load_mask_run(eid, &expect(pid)).unwrap().unwrap();
        assert_eq!(
            kept.input.request_id, 7,
            "the previous complete run must survive"
        );
        let runs: i64 = store
            .connection
            .query_row("SELECT count(*) FROM mask_runs", [], |r| r.get(0))
            .unwrap();
        let artifacts: i64 = store
            .connection
            .query_row("SELECT count(*) FROM mask_artifacts", [], |r| r.get(0))
            .unwrap();
        assert_eq!((runs, artifacts), (1, 5));
    }

    #[test]
    fn duplicate_submission_replaces_instead_of_stacking() {
        let (store, _, pid, eid, run) = setup("dup");
        store.save_mask_run(&run).unwrap();
        let mut again = run.clone();
        again.request_id = 9;
        store.save_mask_run(&again).unwrap();
        let loaded = store.load_mask_run(eid, &expect(pid)).unwrap().unwrap();
        assert_eq!(loaded.input.request_id, 9);
        let artifacts: i64 = store
            .connection
            .query_row("SELECT count(*) FROM mask_artifacts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(artifacts, 5);
    }

    #[test]
    fn invalid_sets_are_refused_before_anything_is_written() {
        let (store, _, _, eid, run) = setup("invalid");
        let mut cases = Vec::new();
        let mut four = run.clone();
        four.masks.pop();
        cases.push(four);
        let mut duplicate = run.clone();
        duplicate.masks[4].kind = "ma".to_string();
        cases.push(duplicate);
        let mut short = run.clone();
        short.masks[2].pixels.pop();
        cases.push(short);
        let mut resized = run.clone();
        resized.masks[3].width = 8;
        cases.push(resized);
        let mut grey = run.clone();
        grey.masks[0].pixels[0] = 128;
        cases.push(grey);
        let mut bad_digest = run.clone();
        bad_digest.source_image_sha256 = "zz".to_string();
        cases.push(bad_digest);
        let mut huge = run.clone();
        huge.image_width = 10_000;
        huge.image_height = 10_000;
        cases.push(huge);
        for case in cases {
            assert!(store.save_mask_run(&case).is_err());
        }
        assert!(!store.has_mask_run(eid).unwrap());
    }

    #[test]
    fn deleting_the_examination_removes_its_masks() {
        let (store, _, _, eid, run) = setup("cascade");
        store.save_mask_run(&run).unwrap();
        store
            .connection
            .execute("DELETE FROM examinations WHERE id = ?1", params![eid])
            .unwrap();
        let artifacts: i64 = store
            .connection
            .query_row("SELECT count(*) FROM mask_artifacts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(artifacts, 0);
        assert!(!store.has_mask_run(eid).unwrap());
        // delete_mask_run is idempotent
        store.delete_mask_run(eid).unwrap();
    }

    #[test]
    fn v2_database_is_backed_up_and_migrated_to_v3_without_losing_records() {
        let (store, path, pid, eid, _) = setup("migrate");
        store
            .set_examination_analysis(eid, r#"{"state":"graded"}"#)
            .unwrap();
        store
            .connection
            .execute_batch(
                "DROP TABLE mask_artifacts; DROP TABLE mask_runs; PRAGMA user_version = 2;",
            )
            .unwrap();
        drop(store);
        let store = SqlCipherStore::open(&path, "pw").unwrap();
        assert_eq!(store.schema_version().unwrap(), 3);
        let exam = store.list_examinations().unwrap().remove(0);
        assert_eq!(exam.analysis_json, r#"{"state":"graded"}"#);
        assert_eq!(store.list_patients().unwrap().len(), 1);
        assert!(store.load_mask_run(eid, &expect(pid)).unwrap().is_none());
        let mut backup = path.as_os_str().to_os_string();
        backup.push(format!(".pre-v{SCHEMA_VERSION}.bak"));
        assert!(PathBuf::from(backup).exists());
    }
}
