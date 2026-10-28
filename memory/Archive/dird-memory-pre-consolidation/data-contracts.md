---
title: Data and inference contracts
type: contracts
domain: engineering
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../native/crates/retinacare-ai-bridge/src/stage2_contract.rs"
  - "../../../native/crates/retinacare-app/src/models.rs"
  - "../../../GUI/Hackthon dezine/rust/src/engine/backend.rs"
owner: Codex/Claude backend; Antigravity GUI
tags: [contracts, IPC, masks]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/data-contracts.md (sha256 7a0c93b47e6c); verify current state in [[active-phase]] -->
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
unavailable. See [[Archive/dird-memory-pre-consolidation/architecture]], [[Archive/dird-memory-pre-consolidation/source-map]], [[Archive/dird-memory-pre-consolidation/stage-ii-segmentation]],
[[Archive/dird-memory-pre-consolidation/stage-iii-grading]], [[Archive/dird-memory-pre-consolidation/gui-implementation]].

Patient images and personal data must not enter memory notes, public fixtures,
logs or unsanitized exports. See [[Archive/dird-memory-pre-consolidation/security]].
