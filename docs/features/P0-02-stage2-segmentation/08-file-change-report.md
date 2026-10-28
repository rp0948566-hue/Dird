# Stage II file change report

P0.2 now includes authentic MA and HE model paths. Checkpoints, ONNX graphs,
temporary image/tensor/mask fixtures remain outside Git under `/tmp`. GUI
directories were not modified.

| File | Purpose |
|---|---|
| `docs/features/P0-02-stage2-segmentation/README.md` | Phase status and document map. |
| `docs/features/P0-02-stage2-segmentation/01-upstream-research.md` | Pinned v1.0 model, transform, threshold and license findings. |
| `docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md` | Exact upstream filenames, byte counts and hashes; availability/approval. |
| `docs/features/P0-02-stage2-segmentation/03-design-and-implementation-plan.md` | Stage milestones, existing Rust reuse and sequencing. |
| `docs/features/P0-02-stage2-segmentation/04-mask-spatial-contract.md` | Typed five-stream output and inverse mapping requirements. |
| `docs/features/P0-02-stage2-segmentation/05-conversion-parity.md` | Safe checkpoint conversion and numerical parity gates. |
| `docs/features/P0-02-stage2-segmentation/06-test-plan.md` | Model, geometry, lifecycle and acceptance tests. |
| `docs/features/P0-02-stage2-segmentation/07-test-results.md` | Measured MA conversion, parity, Rust runtime and remaining verification. |
| `docs/features/P0-02-stage2-segmentation/08-file-change-report.md` | This current phase inventory. |
| `docs/features/P0-02-stage2-segmentation/09-blockers-and-remaining.md` | Explicit missing weights and permission gates. |
| `docs/features/P0-02-stage2-segmentation/10-stage3-handoff.md` | Future five-mask contract only. |
| `native/crates/retinacare-ai-bridge/src/lib.rs` | Exports Stage II contract and preprocessing modules. |
| `native/crates/retinacare-ai-bridge/src/stage2_contract.rs` | Typed model/request/mask/transform contracts, validation, errors and unit tests. |
| `native/crates/retinacare-ai-bridge/src/stage2_preprocessing.rs` | Shared bounded MA/HE OpenCV decode, circle crop, 1024 RGB normalization, probability resize and source-frame mask restore. |
| `native/crates/retinacare-ai-bridge/src/model_runtime.rs` | Verified-session MA/HE inference, strict per-model provenance/threshold selection and ignored local PyTorch parity tests. |
| `native/crates/retinacare-ai-bridge/src/model_manifest.rs` | Retains verified manifest validation/provenance metadata for Stage II model identity checks. |
| `native/crates/retinacare-app/src/models.rs` | Nonblocking MA/HE lesion inference command/event, queue behavior and ignored local service integration tests. |
| `scripts/stage2_export_lesion_parity.py` | Checksum-gated MA/HE PyTorch-to-ONNX development export, CPU parity and private local fixtures. |
| `ai_engine/tests/test_stage2_export_integrity.py` | Regression tests rejecting checkpoint size/hash mismatches before load. |
| `/tmp/retinacare-models/stage2_hsmoe_aunet_HE_best_model.pth` | User-authorized official HE checkpoint, exact size/hash verified; outside repository/Git. |
| `/tmp/retinacare-models/he/` | HE ONNX graph, manifest and local-only parity fixtures; private temporary directory. |
| `docs/index.md` | Adds the Stage II document entry. |
| `memory/00-index.md` | Links the indexed P0.2 active-phase note. |
| `memory/active-phase.md` | Tracks P0.2 work while retaining P0.1 partial gates. |
