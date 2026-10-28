---
title: Next actions and gates
type: plan
domain: project
status: PARTIAL
verified_on: 2026-10-28
source_refs: ["../AGENTS.md", "../docs/features/P0-02-stage2-segmentation/09-blockers-and-remaining.md"]
owner: Codex backend; Antigravity GUI
tags: [priorities, acceptance]
---
# Next actions

1. **Resolve vault selection with user**: Dird `memory/` is the backend
   handoff vault; GUI repo has a separate sibling memory folder. Do not merge
   or replace either until user confirms their active Obsidian vault.
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
Details: [[active-phase]], [[risks-and-blockers]], [[agent-ownership]].
