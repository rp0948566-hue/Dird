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
# Risks and blockers

| Priority | Item | Evidence / impact | Owner and unblock |
|---|---|---|---|
| P0 | No licensed-use retinal image acceptance | Demo image provenance/license unknown; technical parity is not clinical validation. | Project owner: provide documented permitted image; test without redistribution. |
| P0 | RTX 5070/CUDA not exercised | Driver inaccessible; no GPU execution or VRAM data. | User on target Ubuntu: run documented CUDA smoke test. |
| P1 | Stage II only MA/HE | EX/CWS/vessel absent; full five-output stage gate open. | User authorizes each asset separately; Codex integrates one by one. |
| P1 | GUI metadata mismatch | ReportViewModel hardcodes a checkpoint SHA different from manifest and ORT rc.9 while backend pins rc.15. Report may misstate provenance. | Antigravity fixes from verified runtime result, then tests. |
| P1 | GUI enum invents Stage IV | GUI backend enum names Stage IV grading, while official cascade has three stages. Misleads status and handoff. | Antigravity reconciles labels with upstream model pipeline. |
| P1 | GUI integration only reaches Stage I | Path dependencies use sibling Dird checkout; GUI adapter lacks Stage II artifact events. Moving repos breaks paths. | Owners agree stable workspace/API and packaging. |
| P1 | Root migration status is stale | docs/migration-progress.md still says Stage II is absent / not to start; later user-authorized work completed MA/HE slices. | Update repo-level progress docs in a later documentation task. |
| P1 | PDF/audit unavailable in production | Live services return unavailable; test mocks include placeholder PDF and sample audit rows. | Backend implements real PDF/audit persistence before claiming delivery. |
| P1 | Demo/mock clinical-like data | Seeded chat entries contain specific findings; mock result/env flags can show fabricated score. Must stay explicitly demo-only and off clinical flows. | Antigravity isolates/removes from production defaults and verifies release behavior. |
| P2 | Duplicate memory folders | Dird/memory is a historical copy; Hackthon Backend is canonical backend vault (owner decision). | Retire Dird/memory only with owner approval. |
| P2 | Windows execution untested | Cross-built PE is not evidence of Windows runtime behavior. | User executes checklist after reboot. |

No clinical certification, accuracy, sensitivity, fairness or safety claim is
verified. See [[Handoffs/next-actions]], [[Handoffs/agent-ownership]], [[Models/licensing-and-data]].
