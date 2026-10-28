---
title: Source code map
type: source-map
domain: engineering
status: VERIFIED
verified_on: 2026-10-28
source_refs:
  - "../../native/Cargo.toml"
  - "../../GUI/Hackthon dezine/rust/Cargo.toml"
owner: Codex/Claude backend; Antigravity GUI
tags: [source, navigation]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/source-map.md (sha256 810a151986e9); verify current state in [[active-phase]] -->
# Source map — open symbols on demand

Backend workspace: [native/Cargo.toml](../../native/Cargo.toml).

| Concern | Path / symbols |
|---|---|
| Typed patient/exam data | native/crates/retinacare-domain/src/lib.rs |
| Encrypted database and imports | native/crates/retinacare-persistence/src/lib.rs (SqlCipherStore) |
| Report data contract | native/crates/retinacare-reports/src/lib.rs (ReportData) |
| Verified model identity and discovery | native/crates/retinacare-ai-bridge/src/model_manifest.rs, model_registry.rs |
| ONNX provider/session | native/crates/retinacare-ai-bridge/src/onnx_session.rs, device.rs |
| Stage I preprocessing / decode | native/crates/retinacare-ai-bridge/src/preprocessing.rs, stage1.rs |
| Stage II mask contract / preprocessing | native/crates/retinacare-ai-bridge/src/stage2_contract.rs, stage2_preprocessing.rs |
| Stage I/II runtime | native/crates/retinacare-ai-bridge/src/model_runtime.rs |
| Nonblocking model service | native/crates/retinacare-app/src/models.rs (ModelService, ModelEvent) |
| Persistent Python utility worker | ai_engine/src/retinacare_ai/worker.py, devices.py, preprocessing.py |
| Rust shell in Dird | native/crates/retinacare-ui/src/main.rs; frontend-owned per AGENTS.md |
| GUI service adapter | GUI repo rust/src/engine/backend.rs (LiveCodexBackendService) |
| GUI export/report layer | GUI repo rust/src/engine/export.rs, report.rs |
| GUI shell / mock switches | GUI repo rust/src/main.rs |
| GUI viewer/report surfaces | GUI repo rust/src/ui/viewer.rs |
| Stage III eight-channel input, CORN decode, fold runtime | native/crates/retinacare-ai-bridge/src/stage3_input.rs, stage3_grading.rs, model_runtime.rs (`infer_stage3_fold`); fixture `fixtures/stage3_corn_vectors.json`; dev scripts `scripts/stage3_export_parity.py`, `stage3_reference_fixture.py` |
| Stage I→II→III cascade, events, cancel | native/crates/retinacare-app/src/cascade.rs; `ModelService::analyze` in models.rs |
| Development conversion | scripts/stage1_export_parity.py, stage2_export_lesion_parity.py |
| Runtime manifests | models/stage1_convnextv2l_768/manifest.json, /tmp/retinacare-models/{ma,he}/manifest.json (Stage II manifests not copied yet) |

External GUI paths are documented in [[Handoffs/agent-ownership]]. Full flow: [[Architecture/architecture]],
[[Models/model-pipeline]], [[Backend/data-contracts]].
