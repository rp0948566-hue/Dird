---
title: Data and inference contracts
type: contracts
domain: engineering
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../native/crates/retinacare-ai-bridge/src/stage2_contract.rs"
  - "../../native/crates/retinacare-app/src/models.rs"
  - "../../GUI/Hackthon dezine/rust/src/engine/backend.rs"
owner: Codex/Claude backend; Antigravity GUI
tags: [contracts, IPC, masks]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/data-contracts.md (sha256 9b4d21a78f7c); verify current state in [[active-phase]] -->
# Data contracts

Backend Stage I image results retain request/examination/model IDs, model hash,
provider, raw logit, score, threshold, route and timings. ModelService emits
typed events asynchronously. Python worker is a local JSONL utility protocol;
it is not the production model runner.

Stage II request and mask result types carry image/examination identity, model
kind/version/hash, dimensions, registered transform, mask encoding, threshold
and processing status. Only MA and HE runtime branches exist today. Review
native/crates/retinacare-ai-bridge/src/stage2_contract.rs before adding consumers.

GUI Task 04 has separate request/event/result adapter types. It currently
sends Stage I requests; there is no GUI Stage II mask event adapter. The GUI
report/export contracts are separate and live PDF/audit backends are
unavailable. See [[Architecture/architecture]], [[Architecture/source-map]], [[Models/stage-ii-segmentation]],
[[Models/stage-iii-grading]], [[Integration/gui-source-findings]].

Patient images and personal data must not enter memory notes, public fixtures,
logs or unsanitized exports. See [[Backend/security]].

## Update 2026-10-28 (supersedes "only MA/HE" and "no GUI Stage II adapter")
- All five Stage II kinds exist; `Stage2MaskResult` is binary 0/255 in the source frame (width/height equal the image).
- Cascade contract: `AnalysisRequest { request_id, examination_id, patient_id, eye, image_path, policy }` -> events `Started`, `StageStarted/Completed/Skipped`, exactly one `Finished(AnalysisReport)`; terminal states Graded / Ungradable / NegativeRoute / Cancelled / Failed. `AnalysisReport` carries image sha256 + size, masks, grading, failure kind (fixed text, no paths), timings, peak RSS, `research_only`.
- GUI stores `CascadeOutcome` JSON (no pixels, ~2 KB) in `examinations.analysis_json`. Identity (request/exam/patient/eye/image sha256/frame size, per mask) is verified before pixels or a grade attach.
- Persistence: schema v2; `audit_events(id, recorded_at, event_type, patient_id, examination_id, status, detail)` append-only.
- Details: [[Integration/gui-backend-integration]].
