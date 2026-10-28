---
title: GUI↔backend integration readiness report
type: report
status: PARTIAL
verified_on: 2026-10-28
---
# Integration readiness report\n\n> UPDATE 2026-10-28 (later): every adapter below is now implemented and demonstrated on the real binary except where [[active-phase]] lists a gap. This note stays as the original plan; implementation facts are in [[Integration/gui-backend-integration]].\n\n(Original evidence: code read 2026-10-28; baselines in [[active-phase]])

Both trees build/test independently. GUI crate links backend `retinacare-ai-bridge` + `retinacare-app` by path. No GUI code besides `rust/Cargo.toml` changed. GUI = `Dird/GUI/Hackthon dezine/rust/src`; backend = `Dird/native/crates`.

Constraint: GUI matches `ModelEvent`/`ModelSubmitError` exhaustively — add nothing to them. Cascade uses separate `AnalysisEvent` / `AnalysisSubmitError` (`retinacare-app/src/{models,cascade}.rs`).

## Stage I — ready, smallest step
Already: `engine/backend.rs` calls `ModelService::infer_stage1_image`. Changes:
- Replace hardcoded model SHA / manifest reads (`backend.rs` ~l.241–300, `model_sha256`) with the value from `ModelEvent` output (`output.sha256`, already used l.435).
- Fix `engine/manifest.rs` stale `.pth` "BLOCKED" entries → point at `models/stage1_convnextv2l_768/stage1.onnx`.
- Extend `PipelineStage` (l.83–88; `is_available` only Stage1): names Stage3 "Vascular", Stage4 "Clinical grading" don't match the real pipeline (I gate, II five masks, III grade).

## Stage II + III — via cascade
Adapter in `engine/backend.rs`: build `AnalysisRequest` + `CascadeModelPaths`, call `ModelService::analyze`, poll `try_next_analysis()`, map `AnalysisEvent`/`AnalysisReport` (`ScreeningSummary`, `MaskSummary`, `GradeSummary`, `StageTiming`, `AnalysisState`) into GUI state. Support `cancel_analysis`. Replace `PendingBackendIntegration` in `engine/report.rs` (l.23, 65–69, 126–133) with real summaries. Show "research-only, single fold 0" label. Load time dominates (~232 s CPU cold); needs progress UI, not a blocking call.
Backend gap: no mask persistence yet (masks only summarized in report).

## Patient persistence
GUI `engine/patient.rs` is GUI-local; hardcoded demo path l.302. Backend `SqlCipherStore` (`retinacare-persistence`): `open`, `add_patient`, `list_patients`, `update_patient`, `add_examination`, `add_screening_examination`, `list_examinations`, `change_password`. Adapter: new GUI trait over `SqlCipherStore`; password/key flow must reuse GUI `security.rs` (AES-GCM) or drop it — decide first. Do NOT replace persistence until agreed (user rule).

## PDF generation
GUI `engine/export.rs` l.189–196 writes mock PDF bytes. Backend `retinacare-reports` (`ReportData::from_records`) is the real source. Adapter: build `ReportData` from stored exam + `AnalysisReport`, render, write atomically to `destination_path`. GUI uses `lopdf`; pick one renderer.

## Audit history
GUI `export.rs` ~l.330 audit "unavailable"; `AuditEvent` types l.286+ and tests use a mock ledger. Backend has no audit store yet → backend work first (append-only SQLCipher table), then GUI adapter.

## Packaging
Two workspaces, two lockfiles; `ort 2.0.0-rc.15` + `opencv 0.101.1` in both. Need a single release build target (GUI bin `retinacare`), model bundle layout + hash manifest, CUDA runtime check, Windows run. See [[Testing/cuda-and-packaging]]. CC BY-NC weights: [[Models/licensing-and-data]].

## Order
Stage I → cascade (II+III) → persistence → PDF → audit → packaging. Next-step proposal in [[Handoffs/next-actions]].
