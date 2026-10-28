---
title: GUI + backend integration (live) - API contracts and status
type: integration
status: PARTIAL
verified_on: 2026-10-28
---
# One native app: how the GUI calls the backend

Binary: `Dird/GUI/Hackthon dezine/rust/target/release/retinacare` (Antigravity eframe/Glow GUI, in-process Rust path deps on `retinacare-{ai-bridge,app,persistence,reports,domain}`). No web/Tauri/HTTP bridge.

## Cascade (Stage I -> II -> III fold 0)
- GUI adapter: `rust/src/engine/cascade.rs` (`CascadeController`, `CascadeView`, `CascadeOutcome`, `verify_identity`, `outcome_from_report`).
- Trait methods on `MedicalInferenceService` (defaults = unavailable, so mocks keep working): `cascade_status`, `submit_cascade(AnalysisRequest)`, `cancel_cascade(id)`, `try_recv_cascade()`.
- Live impl `LiveCodexBackendService`: `ModelService::analyze(request, CascadeModelPaths)`, `cancel_analysis`, `try_next_analysis`. Model manifests derived from the Stage I manifest dir (`discover_cascade_paths`).
- Rules enforced in the GUI: one heavy job at a time, stale/abandoned-event rejection, exactly one terminal event, identity check (request/exam/patient/eye/image sha256/frame size, per mask) before pixels or grade are attached; failed/cancelled never carry a grade. Never extend `ModelEvent`/`ModelSubmitError`.
- Unsaved image => scratch exam id >= `SCRATCH_EXAM_BASE` (1e9), never written to the DB.

## Persistence (encrypted SQLCipher)
- `retinacare-persistence`: schema v2 (`PRAGMA user_version`), additive migration with `<db>.pre-v2.bak` copy first, `audit_events` append-only (triggers reject UPDATE/DELETE), `set_examination_analysis`, `set_examination_image`, `get_patient`, `PatientRecord.created_at`.
- GUI `engine/store.rs` `SharedStore` (Argon2id unlock on a worker thread; locked = explicit error, no plaintext/fixture fallback); `LiveCodexPatientService`, `LiveCodexAuditService` share it. DB path `$RETINACARE_DB_PATH` else `~/.local/share/RetinaCare/database/retinacare.sqlite`.
- Stored per examination: eye, image path, compact outcome JSON (no pixels) plus, since P2, the five verified masks (`mask_runs`/`mask_artifacts`, schema v3). Reopening restores the summary and the masks (`restored`, `masks_restored`) without running a model; a legacy exam or failed verification shows a note and empty panels. Details: [[Backend/mask-storage]].

## PDF / audit
- `retinacare-reports`: `render_pdf`, `validate_pdf`, `write_pdf_atomic` (text-only PDF 1.4, temp+fsync+validate+rename). GUI builds content in `engine/report.rs::build_report_document` from verified results only.
- Audit events recorded: analysis completed/failed/cancelled, PDF export completed/failed, session export. Timestamp from the DB.

## Honest limits
Chats: no LLM backend; seeded conversations only in tests/dev fixtures; session-only. Explainability panel: unavailable (Phase 4). CUDA, Windows, licensed-image acceptance, folds 1-4: pending. Evidence: [[Integration/readiness-report]], [[active-phase]].

## Event contract and timings (verified)
Request -> `Started` -> per stage `StageStarted` / `StageCompleted` (or `StageSkipped` after a negative route) -> exactly one `Finished`. GUI release run, CPU, demo image: screening 3.8 s, MA 5.2, HE 4.7, EX 13.8, CWS 14.1, vessel 5.5, grading 1.8; total 67-74 s; peak RSS 2951 MiB. Debug-build heavy test: 224 s, 3.78 GB.
## Tests
`src/cascade_flow_tests.rs` (scripted backend: identity, positive/negative/ungradable/failed/cancel, queue full, duplicate, patient switch, out-of-order events, persistence + audit, PDF via lopdf, intake limits, shutdown cancel); `engine/cascade.rs`, `store.rs`, `patient.rs`, `export.rs`, `report.rs`, `ui/cascade_cards.rs` unit tests; ignored heavy: `live_cascade_runs_through_the_gui_adapter`, `live_service_runs_stage1_on_demo_image`.
## Commits
Parent `45f4feb`, GUI `574f8f9` (local only). Evidence: `Dird/docs/features/GUI-BACKEND-INTEGRATION/`.
