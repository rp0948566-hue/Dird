# M5A — source audit and plan (Stage I → II → III fold 0 cascade)

Technical single-fold integration. Not a five-fold ensemble, not CUDA, not licensed-image acceptance, not a diagnosis.

## Audit (2026-10-28, current files)
- Git: `main` at `8fb2fee`, 2 ahead of origin; Phase 3 work uncommitted (preserved).
- `ModelService` (`retinacare-app/src/models.rs`): one worker thread owning a `ModelRuntime`; commands through `sync_channel(2)` (QueueFull is explicit); events through `sync_channel(4)`; `Drop` drains events so the worker can exit. No cancel, no request id. Stage I, Stage II and (new) Stage III runtime calls exist as separate methods.
- The GUI repo (`rust/src/engine/backend.rs`) matches `ModelEvent` and `ModelSubmitError` directly. **Adding variants to those enums could break the GUI build**, so M5A leaves both enums untouched and adds a separate analysis channel and error type.
- Stage I (`stage1.rs`): one logit → sigmoid → `>= 0.2391716` is `ContinueCascade`, below is `NegativeRoute`; non-finite or wrong-length output is an error. Pinned upstream `cascade.py` has no third "uncertain" branch: blocked images are scored Grade 0, routed images go through Stage II and III. A failed Stage I is therefore an error, never Grade 0.
- Stage III ungradable: grade 5 (P(gradable) < 0.10) overrides severity; no severity is reported.
- Source image is re-read and re-decoded by each stage (7 reads). Mitigation: the cascade hashes the file once and requires every stage result to carry the same SHA-256 and frame size.
- Masks stay in memory as typed `Stage2MaskResult` (no PNG round trip). Stage III reuses `assemble_stage3_input` (validated, no zero fill).

## Design (smallest change)
- `retinacare-app/src/cascade.rs`: request/event/report types, a `CascadeRunner` trait (screening, five segmentations, grading), a pure `run_cascade` state machine, and `ModelRuntimeRunner` (load → run → unload each model sequentially; a model already loaded is reused and left loaded).
- `ModelService::analyze` / `cancel_analysis` / `try_next_analysis`: one new command, a separate bounded analysis event channel, duplicate request ids rejected, cancellation checked at stage boundaries (an ONNX call already running cannot be interrupted).
- Identity: request id, examination id, optional patient id, optional eye; every stage result must match source SHA-256, frame size, examination and eye.
- Terminal states (exactly one `Finished` event): `Graded`, `Ungradable`, `NegativeRoute` (Stage I below threshold; cascade grade 0 by Stage I), `Cancelled`, `Failed`. Failed or cancelled runs keep earlier stage evidence as diagnostic only and report no grade.
- Every report carries `research_only`, `origin`, `validation_status = technical_only`, fold-0 scope. Error messages are fixed text (no paths or IDs).
- Skipped for now (not justified by this need): Windows verification, GUI idle recovery (GUI-owned), VRAM measurement, OOM injection beyond a failing-runner double, event coalescing (events per run are few and bounded).

## Tests
Deterministic doubles for T01–T20, T25; heavy: five Stage II regressions, fold-0 parity, eight-channel parity, full cascade trace, RSS measurement.
