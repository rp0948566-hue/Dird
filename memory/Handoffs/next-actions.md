---
title: Next actions
type: plan
status: PARTIAL
verified_on: 2026-10-29
---
# Next actions (ordered)

0b. **2026-10-29 demo build verified end to end** (see changelog). Open: Occlusion explainability run not re-verified, Stage III and explainability GPU parity, PDF page 1 long filename overflow, Patients buttons differ from reference (Open in Report / Open profile vs Open in Clinical Assessment), system Ollama repair needs sudo.
0a. **GPU pushed 2026-10-29**: Dird `test-gpu` `7c7144c`, GUI `test` `5ce9ac9`. Decide merge to main.  (Previous Chats light-theme bug fixed 2026-10-29.)
0. **2026-10-29 state**: Phase 4 and Phase 5 committed locally, not pushed. Parent `main` `4a936ec` (11+ ahead of `Debaq/Dird`, push 403). GUI `dezine` `c3e4665`. Decide: commit/push policy, backend push target. Phase 5 open: Mode C needs a runtime (`llama-server` missing; permission refused), live two-user Supabase test needs user accounts, mask ablation, Windows. Ledger: [[Handoffs/p5-progress]].

1. **User decision**: backend push target. `Debaq/Dird` returns 403; code already lives on `AI-Manthan-2.0` (`main` `0997866`, `backend-rust`). Also approve committing the new vault edits (README, `.obsidian`, state note) if wanted. Snapshot: [[Integration/project-state-sync-2026-10-28]].
2. Obtain an authorized/licensed retinal image set: real negative-route and ungradable runs, formal Stage I/II/III acceptance (still BLOCKED).
3. DONE 2026-10-28: masks persisted and desktop-verified (schema v3, atomic, checksummed) - [[Backend/mask-storage]]. Open: decide whether to push backend `main` (6 ahead of Debaq/Dird `main`; remote reachable read-only on 2026-10-28, push not authorized).
4. DONE 2026-10-29 (partial): CUDA works for Stage I, Stage II, vessel (parity <=0.2%, [[Testing/cuda-and-packaging]]). Open: Stage III five-fold GPU parity, explainability GPU parity, real end-to-end GPU analysis, then lift the `GPU_VERIFIED_TASKS` gate for them. Windows run still open.
5. DONE 2026-10-28: folds 1-4 downloaded (approved), ensemble implemented and verified on CPU - see [[Models/stage-iii-grading]]. Committed locally (parent `1937ed9`, GUI `7097319`), not pushed.
5b. DONE 2026-10-28 (committed locally 2026-10-29 `8581790`/`0f14417`): Phase 4 explainability, see [[Models/explainability]]. Open: commit/push decision; Stage I explanation (needs exposed preprocessing geometry); five-fold Grad-CAM; mask-channel ablation; labelled evaluation (licensed data).
6. Previous Chats: needs an LLM backend decision before any chat storage is worth building.
7. Design phase (separate task): toolbar/header polish, Previous Chats banner overlap, sidebar width at <800 px.
8. Housekeeping: remove obsolete `render_pipeline_matrix_card` (kept, `#[allow(dead_code)]`); `RETINACARE_DB_PATH` default location; `rust/target` ~16 GB.

Do not: remove `retinacare-ui`, touch the legacy React/Tauri app, add a second GUI.
