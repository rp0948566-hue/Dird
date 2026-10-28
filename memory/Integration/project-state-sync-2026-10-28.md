---
title: Project state sync 2026-10-28 (post-compaction)
type: status
status: VERIFIED
verified_on: 2026-10-28
---
# Project state sync (2026-10-28)

Re-verified against filesystem, git and a fresh test run. Entry points: [[README]], [[active-phase]], [[Handoffs/next-actions]]. Research prototype only; no clinical validation.

## Paths
- Project root `/home/claude/Desktop/Dird`; backend workspace `native/`; models `models/` (gitignored); canonical vault `memory/`.
- GUI crate `GUI/Hackthon dezine/rust` (`retinacare-desktop`, binary `retinacare`, eframe/egui 0.37, Glow). Nested independent repo; parent excludes `/GUI/` in `.git/info/exclude`.

## Git (verified now)
| Repo | Local | Remote state |
|---|---|---|
| Parent `Dird` (`main`) | `01f4cb7`, clean tree, `git diff --check` ok | `origin` = `Debaq/Dird` -> push 403, 4 commits unpushed. Local-only branch `combined-main` = `0997866`. |
| GUI nested (`dezine`) | `4814064`; only 2 untracked Task 07 guide PDFs | `origin` = `rp0948566-hue/AI-Manthan-2.0`: `dezine` = `4814064` |

`AI-Manthan-2.0` remote heads: `main` = `0997866` (merge commit: backend tree at root + GUI under `GUI/Hackthon dezine/`, both histories as parents, fast-forward from `4814064`), `backend-rust` = `01f4cb7`, `dezine` = `4814064`, `backend`/`frontend` = `bc302fa`. Old remote `main` (`c25bbb0`, unrelated history) was replaced by user-approved force push; verified bundle: `/home/claude/Desktop/retinacare-backups/2026-10-28-old-remote-main/`. Local nested layout and remote tree layout differ: remote has one repo, local has two.
No model binaries, DBs, keys or files >5 MB tracked in the parent; only fixture `native/crates/retinacare-ai-bridge/fixtures/tiny_mul.onnx` matches the model-file pattern (test fixture).

## Tests (rerun this task, 2026-10-28)
| Check | Result |
|---|---|
| Backend `cargo fmt --check`, `clippy -D warnings`, `build --release` | pass |
| Backend `cargo test --workspace` | 97 passed, 0 failed, 17 ignored (heavy/env-gated, not run) |
| GUI `fmt --check`, `clippy -D warnings`, `build --release` | pass |
| GUI `cargo test` | 166 passed, 0 failed, 2 ignored (real-model tests, not rerun now; passed in earlier session) |
Not rerun: heavy real-model tests, CUDA, Windows, desktop E2E. `cargo check` is covered by clippy.

## Feature status
| Feature | Status | Limit |
|---|---|---|
| Stage I screening | IMPLEMENTED_NOT_FULLY_TESTED | real ONNX on CPU; no licensed image, no CUDA |
| Stage II (MA, HE, EX, CWS, vessel) | IMPLEMENTED_NOT_FULLY_TESTED | all 5 CPU parity vs reference; masks not stored |
| Stage III fold 0 | PARTIAL | single fold, technical grade only; folds 1-4 and ensemble NOT_STARTED |
| Cascade via `ModelService` | IMPLEMENTED_NOT_FULLY_TESTED | queue 2, cancel at stage boundary; negative/ungradable only with scripted events |
| Native GUI | IMPLEMENTED_NOT_FULLY_TESTED | single binary, live adapters; cosmetic issues deferred |
| Patients / exams (SQLCipher) | IMPLEMENTED_NOT_FULLY_TESTED | schema v2, migration with backup+rollback, restart durability shown on desktop |
| Previous Chats | MOCK_OR_FIXTURE_ONLY | no LLM backend, no storage; honest empty state |
| Clinical Timeline | PARTIAL | built from stored records |
| PDF reports | IMPLEMENTED_NOT_FULLY_TESTED | text-only PDF 1.4, independently opened |
| Audit ledger | PARTIAL | append-only (triggers), not hash-chained |
| Explainable AI (Phase 4) | NOT_STARTED | viewer panel 4 says unavailable; no heatmap |
| CUDA (RTX 5070) | BLOCKED | driver visible, ONNX CUDA provider never run |
| Windows release | NOT_STARTED | no Windows build |
| Licensed-image acceptance | BLOCKED | OD-1.png provenance unknown |

## Legacy vaults (backups, never active)
Marked with a pointer to this vault (verified): `/home/claude/Desktop/Hackthon dezine/{memory,Hackthon Backend}` and `Dird/GUI/Hackthon dezine/{memory,Hackthon Backend}`. Nothing deleted. Vault check: 727 wikilinks, 0 unresolved, 0 broken relative links; `.obsidian/app.json` added this task so Obsidian opens the folder cleanly.

## Blockers needing the user
1. Writable remote for the backend (`Debaq/Dird` 403): collaborator access or an own repo.
2. Authorized licensed retinal images (formal acceptance, real negative/ungradable runs).
3. Per-fold download approval for folds 1-4 (CC BY-NC weights).
4. Real GPU session for CUDA; Windows machine for packaging.
5. LLM backend decision before Previous Chats storage.

## Next coding milestone (not started)
Persist masks (versioned files + manifest, atomic writes) so saved exams reopen with pixels. Inspect: `GUI/Hackthon dezine/rust/src/engine/{cascade,patient,store}.rs`, `native/crates/retinacare-persistence/src/lib.rs`. Then folds 1-4/ensemble, then Phase 4 explainability, CUDA, Windows. See [[Integration/gui-backend-integration]], [[Models/stage-iii-grading]], [[Testing/build-and-test]].
