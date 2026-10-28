# P0.1 file change report

| Path | Change | Purpose |
| --- | --- | --- |
| `native/crates/retinacare-ai-bridge/src/model_manifest.rs` | Hardened and typed | Bounded manifest parsing; safe one-file paths; size cap and exact size; ONNX/PyTorch distinction; input/output contract; SHA-256 gate; regression tests. |
| `native/crates/retinacare-ai-bridge/src/model_registry.rs` | Added | Bounded local discovery, invalid-entry reporting, duplicate ID detection and conversion-required states. |
| `native/crates/retinacare-ai-bridge/src/device.rs` | Added | Explicit CPU/CUDA request and truthful compiled-provider capability reporting. |
| `native/crates/retinacare-ai-bridge/src/onnx_session.rs` | Extended | CPU/CUDA session setup, no silent CPU fallback for CUDA, strict input/output metadata and buffer-length checks. |
| `native/crates/retinacare-ai-bridge/src/model_runtime.rs` | Added | Loaded-session lifecycle and typed load/infer/unload errors. |
| `native/crates/retinacare-ai-bridge/src/stage1.rs` | Added | Stable sigmoid, exact upstream gate threshold, typed routing result and decoder regression tests. |
| `native/crates/retinacare-ai-bridge/src/preprocessing.rs` | Added | Bounded JPEG/PNG preflight and OpenCV decode; source-equivalent retinal ROI, crop, Lanczos4/area resize, CLAHE, Ben Graham/FOV, LAB and unsharp steps; NCHW ImageNet tensor; corruption/size tests and opt-in Python/ONNX parity test. |
| `native/crates/retinacare-ai-bridge/src/lib.rs` | Extended | Exposes backend modules while preserving `WorkerClient`. |
| `native/crates/retinacare-ai-bridge/src/model_runtime.rs` | Extended | Adds JPEG/PNG Stage I entry point, image SHA/dimensions, preprocessing version and stage timing to typed result. |
| `native/crates/retinacare-ai-bridge/Cargo.toml` | Extended | Adds pinned OpenCV 0.101.1 with only `imgcodecs`, `imgproc`, `clang-runtime`; retains opt-in CUDA feature. |
| `native/Cargo.toml`, `native/Cargo.lock` | Extended | Pins OpenCV and image preflight dependency graph. |
| `native/crates/retinacare-app/src/models.rs` | Added | Bounded nonblocking model task queue and typed event variants. |
| `native/crates/retinacare-app/src/models.rs` | Extended | Adds nonblocking typed Stage I tensor/image inference command/events and ignored local end-to-end service test with explicit CPU/CUDA provider selection and actual-provider assertion. |
| `scripts/stage1_preprocess_reference.py` | Added | Development-only source-linked Python/OpenCV and PyTorch parity oracle; writes only to explicit local output paths. |
| `docs/features/P0-01-hierarchiretina-model-loading/04-implementation-plan.md` | Updated | Records completed Rust OpenCV decision/implementation and measured parity; lists open GPU/licensing gates. |
| `docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md` | Updated | Documents exact source operations, install prerequisites, CPU parity measurements, local-only image restrictions and open acceptance gaps. |
| `docs/features/P0-01-hierarchiretina-model-loading/07-testing-strategy.md` | Updated | Adds image validation, parity, and app-service integration coverage. |
| `docs/features/P0-01-hierarchiretina-model-loading/08-test-results.md` | Updated | Records current Rust/Python suite counts, exact numerical parity, app-service run, and unverified GPU/licensing/clinical status. |
| `docs/features/P0-01-hierarchiretina-model-loading/09-file-change-report.md` | Updated | This file-by-file inventory for the Stage I implementation. |
| `docs/features/P0-01-hierarchiretina-model-loading/10-risks-and-limitations.md` | Updated | Records remaining dataset provenance, CUDA, clinical and model-integrity limits. |
| `docs/features/P0-01-hierarchiretina-model-loading/11-next-phase-handoff.md` | Updated | Lists only remaining P0.1 work and keeps GUI ownership with Antigravity. |
| `docs/features/P0-01-hierarchiretina-model-loading/README.md` | Updated | Refreshes feature status and links to current evidence. |
| `docs/features/P0-01-hierarchiretina-model-loading/file-change-report.md` | Updated | Labels earlier initial-phase inventory as historical and points to this current report. |
| `native/crates/retinacare-app/src/lib.rs` | Extended | Exposes the app model service; existing Python worker remains intact. |
| `models/manifest.example.json` | Updated | Documents required size and tensor metadata fields. |
| `models/stage1_convnextv2l_768/manifest.json` | Updated | Adds actual artifact size and graph input/output contract. |
| `scripts/stage1_export_parity.py` | Updated | Fixed syntax error, emits runtime manifest fields, and adds non-destructive `--verify-existing` checkpoint/ONNX/golden identity and numerical parity check. Called only by developers; production inference remains Rust. |
| `docs/features/P0-01-hierarchiretina-model-loading/` | Added/updated | Research, contracts, actual verified CPU/PyTorch parity evidence, test results, risks and handoff. |
| `tools/model_conversion/hierarchiretina/README.md` | Added | Points to the existing dev-only export/parity script; no duplicate exporter. |
| `docs/index.md` | Updated | Changes Stage I document description from outdated blockers to parity evidence/open gates. |
| `docs/model-integration.md` | Updated | Documents the Rust image entry points and open license acceptance gate. |
| `docs/migration-progress.md` | Updated | Records exact Stage I CPU implementation, test results and remaining GPU/license gates. |
| `memory/active-phase.md` | Updated | Sets current P0.1 state, evidence and next gates. |
| `memory/retinacare-current-state.md` | Updated | Replaces stale preprocessing/test status with verified implementation state. |
| `memory/gotchas.md` | Updated | Records OpenCV build prerequisites and no-GPU caveat. |

No file under `native/crates/retinacare-ui/` or Antigravity's `rust/src/` was edited. The approved demo image was not copied into a fixture, generated artifact, or distribution. Model weights and ONNX artifacts were not downloaded, overwritten or modified. Existing worktree changes were preserved.
