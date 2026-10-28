---
title: Next actions
type: plan
status: PARTIAL
verified_on: 2026-10-28
---
# Next actions (ordered)

1. **User decision**: backend push target. `Debaq/Dird` returns 403; code already lives on `AI-Manthan-2.0` (`main` `0997866`, `backend-rust`). Also approve committing the new vault edits (README, `.obsidian`, state note) if wanted. Snapshot: [[Integration/project-state-sync-2026-10-28]].
2. Obtain an authorized/licensed retinal image set: real negative-route and ungradable runs, formal Stage I/II/III acceptance (still BLOCKED).
3. DONE 2026-10-28: masks persisted (schema v3, atomic, checksummed) - [[Backend/mask-storage]]. Open: drive it once on the real desktop app (restart, screenshot) and decide about committing P1 + P2 work.
4. CUDA run on the RTX 5070: libs and build are ready; the GPU is in PCI error state, reboot then run `scripts/stage1_cuda_check.sh`. Windows run still open.
5. Folds 1-4 (each needs separate download approval) -> ensemble (M5B); update the research-only banner only when verified.
6. Previous Chats: needs an LLM backend decision before any chat storage is worth building.
7. Design phase (separate task): toolbar/header polish, Previous Chats banner overlap, sidebar width at <800 px.
8. Housekeeping: remove obsolete `render_pipeline_matrix_card` (kept, `#[allow(dead_code)]`); `RETINACARE_DB_PATH` default location; `rust/target` ~16 GB.

Do not: remove `retinacare-ui`, touch the legacy React/Tauri app, add a second GUI.
