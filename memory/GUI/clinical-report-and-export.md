<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/clinical-report-and-export.md (sha256 525ae79c13bd); verify current state in [[active-phase]] -->
# Clinical Report Presentation and Export Architecture

## Architectural Standards
Task 05 implements clinical report overview, local PDF export, scoped session export, and audit ledger.
Report workspace toggles between Quad-Panel inspection and Clinical Report Overview via toolbar switcher.
Presentation data is separated from raw backend data through pre-computed ReportViewModel.
Stage I ConvNeXt-V2 screening results are faithfully presented with score, threshold 0.2391716, logit, and latency.
Stage II segmentations and Stage III five-class DR grading are truthfully marked unavailable.
Mandatory non-diagnostic research caveat is embedded in all clinical view models and export records.
Export state machine transitions asynchronously through Idle, Preparing, Generating, Saved, and Failed.
PDF export modal truthfully informs user of BackendUnavailable state without generating fabricated PDFs.
retinacare-reports crate in Codex contains only ReportData struct and lacks PDF rasterization engine.
retinacare-persistence SqlCipherStore lacks audit_events table and audit query APIs.
Audit ledger modal truthfully informs user of AuditHistoryUnavailable without generating fake audit rows.
Scoped patient session export packages active patient and examination data into isolated local JSON.
Zero cross-patient data leakage is enforced through validation gates.
All PDF, session, and audit operations remain local on device with zero cloud telemetry.
Minimal Codex handoff API contracts are documented in docs/features/GUI-P05-clinical-report-export/13-task06-handoff.md.

## Update 2026-10-28 (supersedes "unavailable" lines above)
PDF export is real (`retinacare-reports`, atomic write, text-only, save dialog; `RETINACARE_PDF_DEST` for automation). The audit ledger is real and persisted ("Append-Only", not tamper-evident). Report shows Stage I + all stages that ran, the research-only banner, and a restored-summary notice for reopened exams. Stage II/III are no longer marked unavailable when they ran. See [[Integration/gui-backend-integration]].
