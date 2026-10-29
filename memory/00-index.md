---
title: RetinaCare canonical vault index
type: index
status: VERIFIED
verified_on: 2026-10-29
---
# RetinaCare vault index (canonical: `Dird/memory`)

One vault for backend + GUI. Read in this order: this note, [[01-claude-start-here]], [[active-phase]], [[Handoffs/next-actions]]. Then only the notes the task needs.

| Need | Note |
|---|---|
| Vault entry / full state snapshot | [[README]], [[Integration/project-state-sync-2026-10-28]] |
| Where code lives, git rules | [[Integration/repo-topology]] |
| How GUI↔backend is wired (live) | [[Integration/gui-backend-integration]] |
| Original adapter plan | [[Integration/readiness-report]] |
| Who owns what | [[Handoffs/agent-ownership]] |
| Session history | [[Handoffs/changelog-and-handoffs]] |
| Architecture | [[Architecture/architecture]], [[Architecture/source-map]] |
| Models Stage I/II/III | [[Models/model-pipeline]], [[Models/stage-i-screening]], [[Models/stage-ii-segmentation]], [[Models/stage-iii-grading]], [[Models/explainability]] |
| Licensing, data limits | [[Models/licensing-and-data]] |
| Build/test/CUDA/packaging | [[Testing/build-and-test]], [[Testing/cuda-and-packaging]] |
| Risks, gotchas, decisions | [[Decisions/risks-and-blockers]], [[Decisions/gotchas]], [[Decisions/decisions]] |
| Backend data/security | [[Backend/data-contracts]], [[Backend/security]], [[Backend/mask-storage]], [[Backend/supabase-architecture]] |
| GUI notes (Antigravity) | [[GUI/gui-00-index]] |
| Phase 5 live ledger | [[Handoffs/p5-progress]] |
| Medical research chat | [[GUI/medical-research-chat]] |

## Rules
- Canonical vault = `/home/claude/Desktop/Dird/memory`. Edit only here.
- `Archive/` = read-only history (old vaults, stale mirrors). Never edit; never trust over canonical notes.
- Old vault copies stay on disk as backups; `scripts/consolidate-vaults.py check memory` flags sources changed after import.
- After each task: update [[active-phase]], [[Handoffs/next-actions]], [[Handoffs/changelog-and-handoffs]] with results, changed files, tests, blockers.
