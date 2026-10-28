# P2 five-mask persistence (2026-10-28)

Research prototype. Technical evidence only; no clinical validation. Test images are synthetic or the unlicensed demo image (provenance unknown, technical use only).

Status: engineering COMPLETE for save, restore, integrity, isolation, migration and failure paths. BLOCKED: labelled Dice/IoU (no licensed labelled set), CUDA (GPU in error state), Windows.

## Design (decisions)
- Masks live INSIDE the SQLCipher database (tables `mask_runs`, `mask_artifacts`, schema v3). One file, one key, one unlock state, one backup/delete lifecycle; no loose mask files, no paths in errors.
- One transaction per run: run row + exactly five artifacts. A failure before commit leaves the previous complete run (or nothing). `complete` is a CHECK-constrained column set in that same transaction.
- One run per examination (`UNIQUE(examination_id)`); a rerun atomically replaces it. `ON DELETE CASCADE` from the examination removes masks.
- Payload: lossless run-length encoding of the 0/255 original-frame mask (u32 LE runs). Real demo run: 5 masks, whole database 86 016 bytes.
- Integrity: SHA-256 per payload, SHA-256 manifest over exam/patient/eye/request/image hash/size and every artifact hash plus model SHA, checkpoint SHA, version and size; foreground count re-checked. Any mismatch returns an error and NO pixels.
- Identity on load: patient, eye and source-image SHA-256 must equal the caller's; the GUI also requires the stored run's request id and image size to equal the stored summary.
- Not stored: raw probabilities, the fundus image, heatmaps. Masks are binary only; probability masks are refused.

## Threat model and retention
- At rest: encrypted by SQLCipher (Argon2id key). Locked database = `DatabaseLocked`, no overlay.
- Tamper/corruption: detected by the hashes above; GUI shows a note and draws nothing.
- Cross-patient/eye leakage: refused with `MaskMismatch`; textures are dropped on every context switch (existing `reset_analysis_context`).
- Retention: masks live as long as their examination; deleting the examination deletes them; `delete_mask_run` is idempotent. No separate quota: a 640x640 run is ~80 KB RLE; images are capped at 40 Mpx.
- Limits: not tamper-proof against someone who holds the database password (they can rewrite hashes too); no per-field encryption beyond the database key.

## Files changed (file by file)
| File | Change |
|---|---|
| `native/crates/retinacare-persistence/src/masks.rs` (new) | types, RLE codec, `save_mask_run`, `load_mask_run`, `has_mask_run`, `delete_mask_run`, v3 SQL, 11 tests |
| `native/crates/retinacare-persistence/src/lib.rs` | `SCHEMA_VERSION` 3, stepwise v2/v3 migration (backup `<db>.pre-v3.bak`), `MaskIntegrity`/`MaskMismatch` errors, re-exports |
| `native/crates/retinacare-persistence/Cargo.toml` | `sha2` (workspace version) |
| GUI `rust/src/engine/cascade.rs` | `mask_run_from_report`, `mask_image_from_pixels`, `kind_from_label`, view fields `masks_restored`/`mask_note`, real-mask roundtrip in the ignored heavy test |
| GUI `rust/src/engine/patient.rs` | trait + live `save_examination_masks` / `load_examination_masks` (defaults keep mocks working) |
| GUI `rust/src/main.rs` | `save_masks_note`, `restore_stored_masks`, `install_mask_texture`, `restore_saved_analysis(ctx, id)` |
| GUI `rust/src/ui/cascade_cards.rs` | restored-state text follows `masks_restored` / `mask_note` |
| GUI `rust/src/cascade_flow_tests.rs` | persist test updated; 4 new tests (isolation, legacy, failed save, Stage III parity) |

## Tests (run now, 2026-10-28)
| Command | Result |
|---|---|
| backend `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `build --release`, `git diff --check` | pass |
| backend `cargo test --workspace` | 120 passed, 0 failed, 18 ignored (heavy, env-gated; not run except below) |
| GUI `cargo fmt --check`, `clippy -D warnings`, `build --release`, `git diff --check` | pass |
| GUI `cargo test` | 170 passed, 0 failed, 2 ignored |
| GUI heavy `cargo test --release live_cascade -- --ignored --nocapture` (real 7 models, CPU) | pass: 63.7 s total, peak RSS 2826 MiB; 5 real masks saved (2.9 ms), reloaded and verified (4.6 ms), pixels identical |
Not run: `local_*` ai-bridge heavy tests, CUDA, Windows, desktop screenshot run.

## Matrix
| # | Item | Status | Evidence |
|---|---|---|---|
| 01 | five-mask roundtrip | VERIFIED | persistence `five_masks_round_trip_exactly_across_a_restart`; real run above |
| 02 | manifest integrity | VERIFIED | `corrupted_payload_manifest_or_missing_artifact...`, `tampered_artifact_metadata...` |
| 03 | eye/exam isolation | VERIFIED | `other_patient_eye_or_image_is_refused`; GUI `restored_masks_of_another_patient...` |
| 04 | crash during commit | VERIFIED (injected abort trigger) | `failure_before_commit_leaves_no_partial_set...`; not a real power-loss test |
| 05 | disk full / permission | PARTIAL | save errors surface as "Masks NOT stored" (`a_failed_mask_save...`, via an invalid set); a real full disk was not simulated |
| 06 | db locked | VERIFIED | wrong password cannot open; `DatabaseLocked` path of the service |
| 07 | migration | VERIFIED | `v2_database_is_backed_up_and_migrated_to_v3...`; v1 and failure tests still pass |
| 08 | Stage III input | VERIFIED (raw assembler) | GUI `restored_masks_build_the_same_stage3_eight_channel_tensor...` bit-identical; uses synthetic masks, no model run |
| 09 | no rerun on history | VERIFIED | persist test asserts `backend.submitted()` unchanged |
| 10 | labelled Dice/IoU | BLOCKED | no licensed labelled masks |
| 11 | CUDA | BLOCKED | GPU in PCI error state |
| 12 | release regression | VERIFIED | counts above |

Edge cases covered: empty HE and sparse MA, image hash changed, duplicate submission, legacy exam without masks, cancelled/failed runs never saved (existing rule), concurrent patient selection. Not covered: a different model version for one mask (stored per mask, not enforced against a reference), CWS morphology change on migration (no such migration exists), real disk quota.

## Not claimed
Clinical accuracy; labelled segmentation quality; CUDA or Windows behaviour; that a desktop session was driven after this change (unit/integration level only).
