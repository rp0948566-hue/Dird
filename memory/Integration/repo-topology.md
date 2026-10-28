---
title: Repo topology and git rules
type: reference
status: VERIFIED
verified_on: 2026-10-28
---
# Repo topology

| Path | What | Git |
|---|---|---|
| `/home/claude/Desktop/Dird` | backend workspace `native/`, legacy app, `ai_engine/`, `memory/` | repo, branch `main` |
| `Dird/GUI/Hackthon dezine` | Antigravity eframe GUI crate `retinacare-desktop` (`rust/`) | NESTED repo, branch `dezine` |
| `Dird/native/crates/retinacare-ui` | backend egui UI (kept; not a duplicate of GUI) | parent repo |
| `/home/claude/Desktop/Hackthon dezine` | OLD GUI location + old vaults | backup; do not edit |

## Rules
- Nested GUI `.git` is intact. No submodule, no history merge. Parent excludes it in `.git/info/exclude` (`/GUI/`), untracked, so never staged by accident.
- GUI Cargo path deps: `../../../native/crates/retinacare-{ai-bridge,app}` (verified by build).
- Two Cargo lockfiles/targets (backend `native/`, GUI `rust/`) are independent; do not share target dirs.
- GUI CLAUDE.md: never `git add`/`git push` without asking the user.
- Restore/rollback: `/home/claude/Desktop/retinacare-backups/2026-10-28-pre-consolidation/` (bundles, patches, tarballs, SHA256SUMS).
