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

## Added 2026-10-28
| Concern | Path |
|---|---|
| Cascade state machine, `AnalysisEvent`, `ModelRuntimeRunner` | native/crates/retinacare-app/src/cascade.rs; entry `models.rs` (`analyze`, `cancel_analysis`, `try_next_analysis`) |
| Audit ledger, schema v2, analysis/image setters, migration | native/crates/retinacare-persistence/src/lib.rs (`append_audit_event`, `list_audit_events`, `set_examination_analysis`, `set_examination_image`, `SCHEMA_VERSION`) |
| PDF writer | native/crates/retinacare-reports/src/lib.rs (`render_pdf`, `validate_pdf`, `write_pdf_atomic`) |
| GUI cascade adapter, outcome, identity checks | GUI rust/src/engine/cascade.rs |
| GUI encrypted store + unlock | GUI rust/src/engine/store.rs; live services in engine/patient.rs, engine/export.rs |
| GUI result cards | GUI rust/src/ui/cascade_cards.rs; overlays/toolbar in ui/viewer.rs |
| Report document builder, UTC time | GUI rust/src/engine/report.rs (`build_report_document`) |
| App-level flow tests (scripted backend) | GUI rust/src/cascade_flow_tests.rs |
| Vault tooling | scripts/consolidate-vaults.py (`check memory`) |
