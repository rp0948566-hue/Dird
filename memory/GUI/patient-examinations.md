<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/patient-examinations.md (sha256 6027a721fbe0); verify current state in [[active-phase]] -->
# Patient and Examination Management

Task ID: GUI Phase 1 Task 06.
Verdict: PARTIAL due to unauthenticated SQLCipher and missing Codex audit persistence.
Patient directory supports real-time search across MRN, given name, and family name.
Patient registration modal validates mandatory MRN and birth year in [1900..2026].
Patient detail editor tracks dirty modifications and prevents unconfirmed loss of data.
Examination history displays chronological exams with explicit OD (Right Eye) vs OS (Left Eye) badges.
Examination laterality is clinician-assigned and never inferred from image appearance.
Direct action buttons load exams into Clinical Report or Quad Viewer workspaces.
Switching active patient strictly purges fundus textures and invalidates pending inference requests.
SqlCipherStore in retinacare-persistence requires an Argon2id key and reports DatabaseLocked in production.
MockPatientService test fixtures activate only under cfg(test) or RETINACARE_DEV_FIXTURES=1.
Test fixture mode displays an amber non-clinical warning chip at all times.
Session export is labeled Plaintext Sensitive Patient Session Export (Unencrypted JSON).
Sensitive session export warns users to store files on encrypted media.
Backend handoff requires Codex to implement unlocked store session injection and indexed search queries.
Backend handoff requires Codex to implement audit event persistence in SQLCipher.
All 105 automated unit and integration tests pass cleanly with zero failures.
Clippy check passes with zero warnings under clippy -D warnings.
Format check passes with zero warnings under cargo fmt --all --check.
Visual desktop verification verified across 6 authentic Linux desktop screenshots.
