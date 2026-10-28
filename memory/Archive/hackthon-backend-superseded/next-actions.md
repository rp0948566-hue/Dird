---
title: Next actions and gates
type: plan
domain: project
status: PARTIAL
verified_on: 2026-10-28
source_refs: ["../../../AGENTS.md", "../../../docs/features/P0-02-stage2-segmentation/09-blockers-and-remaining.md"]
owner: Codex backend; Antigravity GUI
tags: [priorities, acceptance]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/next-actions.md (sha256 b43f6dfd6564); verify current state in [[active-phase]] -->
# Next actions

1. **Vault selection — RESOLVED**: Hackthon Backend is the canonical backend vault (owner decision).
   GUI vault `Hackthon dezine/memory` stays Antigravity-owned.
00. **Phase 3 next (decision needed)**: approve folds 1-4 one at a time (fold 1: 795,798,428 B, SHA `9efb5c93…14eb`) for M4A/M4B, then upgrade the cascade to the verified ensemble (M5B). M5A is done on fold 0. Licensed image, CUDA and a Windows run are still needed. Optional: cache verified models to cut the ~25-40 s per-model load.
0. **Phase 2 next**: (a) licensed or authorized image for formal Stage II acceptance (BLOCKED on owner); (b) CUDA run on the RTX 5070 (`nvidia-smi` sees it; ONNX CUDA execution untested); (c) decide whether Stage II masks must be persisted (versioned artifact manifest, atomic writes) before Stage III. Stage III checkpoints are NOT authorized.
2. **P0.1 acceptance**: obtain a documented-use retinal image and run verified
   Stage I flow; verify CUDA/RTX 5070 on target Linux hardware. Do not call P0.1
   complete before both gates pass.
3. **P0.2 continuation**: seek separate approval for each EX, CWS and vessel
   checkpoint, then checksum, convert, compare, integrate and test one at a
   time. Stage II acceptance requires all five models and licensed-image test.
4. **GUI/backend reconciliation**: resolve GUI Stage I hardcoded metadata and
   outdated stage enum, test the actual path dependency layout, then agree typed
   interfaces for Stage II, PDF and audit storage. GUI code remains Antigravity
   owned.
5. **GUI Task 06**: GUI Task 05 handoff proposes patient/examination workflow;
   confirm scope and reuse existing backend SQLCipher APIs before implementation.

Do not start Stage III until its prerequisites and explicit task are agreed.
Details: [[active-phase]], [[Decisions/risks-and-blockers]], [[Handoffs/agent-ownership]].
