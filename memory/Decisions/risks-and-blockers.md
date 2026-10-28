---
title: Risks and blockers
type: risk-register
domain: project
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../AGENTS.md"
  - "../../docs/features/P0-02-stage2-segmentation/09-blockers-and-remaining.md"
  - "../../GUI/Hackthon dezine/rust/src/engine/report.rs"
  - "../../GUI/Hackthon dezine/rust/src/main.rs"
owner: Codex/Claude backend; Antigravity GUI
tags: [risk, blockers, handoff]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/risks-and-blockers.md (sha256 51a2091bd62b); verify current state in [[active-phase]] -->
# Risks and blockers (updated 2026-10-28)

| Priority | Item | Status / impact | Unblock |
|---|---|---|---|
| P0 | No licensed-use image acceptance | Demo image provenance unknown; technical parity only. Real negative-route / ungradable behaviour never seen on a real image. | Owner provides a documented permitted image set. |
| P0 | CUDA / RTX 5070 not exercised | All runs CPU; no GPU timing or VRAM data. | Run a CUDA session on the target machine. |
| P1 | Masks not persisted | A saved exam reopens with the summary only; pixels need a rerun. | Versioned mask artifacts + manifest, atomic writes. |
| P1 | Fold 0 only | Folds 1-4 and the ensemble are not implemented; every grade is a single-fold technical output. | Separate download approval per fold. |
| P1 | Previous Chats | No LLM backend, no chat storage (ephemeral notice, no seeded chats outside dev/tests). | Decide the assistant backend first. |
| P1 | Uncommitted Antigravity work mixed in | GUI commit `574f8f9` contains Antigravity's Task 07 work and mine. | Owner review. |
| P1 | Two audit claims limited | Ledger is append-only (triggers) but not cryptographically chained; PDF is text-only (no images/masks). | Hash chain / richer PDF if required. |
| P2 | Windows execution untested | Cross-built PE only. | Run the checklist. |
| P2 | Design polish deferred | Previous Chats banner overlaps its controls; sidebar wide at <800 px; some headers clip at 1024. | Separate design phase. |
| P2 | Explainability absent | Panel 4 says unavailable by design. | Phase 4. |

RESOLVED 2026-10-28: GUI hardcoded checkpoint hash / ORT version / invented Stage IV (report and enum now follow the real result and pipeline); GUI reaching only Stage I; live PDF/audit unavailable; Stage II only MA/HE; fake dates, operator and "Dr. Tester" defaults; stale-image mixup on patient switch; shutdown waiting for the whole cascade; toolbar and narrow-layout overlap. Seeded chat sessions are now dev/test only.

No clinical certification, accuracy, sensitivity, fairness or safety claim is verified. See [[Handoffs/next-actions]], [[Handoffs/agent-ownership]], [[Models/licensing-and-data]].
