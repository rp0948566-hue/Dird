---
title: RetinaCare vault README
type: readme
status: VERIFIED
verified_on: 2026-10-28
---
# RetinaCare canonical Obsidian vault

Path: `/home/claude/Desktop/Dird/memory` (open this folder as a vault in Obsidian). Research prototype only; not a medical device; no clinical validation.

## Startup order (small, fixed)
1. [[00-index]] - map of all topics.
2. [[01-claude-start-here]] - rules, commands, launch.
3. [[active-phase]] - what is true now.
4. [[Handoffs/next-actions]] - what to do next.
5. Only then the topic notes the task needs. Full project snapshot: [[Integration/project-state-sync-2026-10-28]].

## Layout
`Architecture/` `Backend/` `GUI/` `Integration/` `Models/` `Testing/` `Decisions/` `Handoffs/` hold live notes. `Archive/` is read-only history (stale vault copies and `migration-manifest.json`). Never edit `Archive/`; never trust it over live notes.

## Maintenance
- Link check: `python3 scripts/consolidate-vaults.py check memory` from `/home/claude/Desktop/Dird`.
- Legacy vault copies (backups, each index carries a pointer to this vault): see [[Integration/repo-topology]].
- After every task update [[active-phase]], [[Handoffs/next-actions]], [[Handoffs/changelog-and-handoffs]] with real results only.
