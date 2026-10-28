---
title: Antigravity GUI workstream
type: workstream
domain: GUI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../GUI/Hackthon dezine/rust/Cargo.toml"
  - "../../../GUI/Hackthon dezine/docs/features/GUI-P01-home-page/08-test-results.md"
  - "../../../GUI/Hackthon dezine/docs/features/GUI-P02-ai-chat-interface/08-functional-test-results.md"
  - "../../../GUI/Hackthon dezine/docs/features/GUI-P03-retinal-image-viewer/08-functional-test-results.md"
  - "../../../GUI/Hackthon dezine/docs/features/GUI-P04-backend-integration/09-integration-test-results.md"
  - "../../../GUI/Hackthon dezine/docs/features/GUI-P05-clinical-report-export/09-test-results.md"
owner: Antigravity
tags: [GUI, handoff, tests]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/gui-implementation.md (sha256 4372199d73d5); verify current state in [[active-phase]] -->
# GUI workstream — separate repository

Repository /home/claude/Desktop/Hackthon dezine, branch dezine, HEAD b1cd4b7.
The GUI uses eframe/egui and path-depends on Dird AI/app crates. Source, docs
and screenshots exist for Tasks 01–05. Existing worktree contains uncommitted
GUI code, docs and memory; preserve it.

| Task | Source evidence and report | Current assessment |
|---|---|---|
| 01 Home | rust/src/main.rs, ui/composer; report says 40 tests | REPORTED; source present, not rerun here. |
| 02 Chat | domain.rs, engine/chat.rs, UI chat; report says 53 tests | REPORTED; mock streaming and unavailable service exist, not a connected clinical LLM. |
| 03 Viewer | ui/viewer.rs; report says 69 tests and screenshot artifacts | REPORTED; original image viewer exists; lesion/vessel/attribution panels show unavailable states when real artifacts are absent. |
| 04 Backend adapter | engine/backend.rs imports retinacare-app and AI bridge; file-change report claims 81 total tests, while the test-results file lists cases without an aggregate | PARTIAL; live adapter currently submits Stage I only. Stage II interface is not wired to GUI. |
| 05 Report/export/audit | engine/report.rs and export.rs; report says 94 tests | PARTIAL; live PDF and audit services return unavailable. 94-test result is a report, not rerun by this audit. Session JSON export has scoped isolation logic. |

Source findings: GUI PipelineStage enum includes a Stage IV grading variant
despite the three-stage upstream pipeline; stage labels also split lesion and
vessel outputs into two named GUI stages. ReportViewModel::new has a hardcoded
Stage I checkpoint hash that differs from the manifest and names ORT rc.9
while Dird workspace pins rc.15. Main.rs has explicit mock result, mock backend,
mock PDF and mock audit environment switches; default services are live or
unavailable adapters. Seeded chat text contains clinical-like claims and must
remain clearly demo-only, never evidence.

Task 05 guide says PDF engine and audit persistence are missing; tests use
mocks, including a placeholder PDF writer. Do not describe this as a real
clinical PDF implementation. Next GUI handoff proposes Task 06 patient and
examination management; confirm scope before starting. See [[Archive/dird-memory-pre-consolidation/agent-ownership]],
[[Archive/dird-memory-pre-consolidation/data-contracts]], [[Archive/dird-memory-pre-consolidation/risks-and-blockers]] and external
[Task 05 limitation record](<../../../GUI/Hackthon dezine/docs/features/GUI-P05-clinical-report-export/12-limitations-and-backend-gaps.md>).
