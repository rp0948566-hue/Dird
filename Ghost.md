# Ghost

Purpose: project AI rules and safe migration boundaries.

## Source of truth
See `AGENTS.md` and `docs/index.md`.

## Clinical safety
Never create unsupported model results or imply regulatory clearance.

## P0 exam-state rules (from 7-screenshot diagnostic report)
- One image, one canonical exam, one verified result shared by Chat, Review, Lesion, Vessel, Findings, Explainability, Assessment, Patients, Timeline, Reports.
- Laterality stays Unknown until confirmed. Never default to OD.
- Stale worker events must not overwrite newer runs: check exam_id + image hash + run_id.
- 100% = required outputs saved and validated. Explainability is optional, never blocks Ready.
- Show `GPU: CUDA` only when a stage really used CUDA. No fake progress, no fake metrics.
- Saved exams keep their own image copy: `~/.local/share/RetinaCare/exam_images/<sha256>.<ext>` (`keep_exam_image`, GUI `src/engine/store.rs`). Timeline shows "Image file missing" instead of black.
- Failed thumbnail loads are not cached (`ui2/composer.rs::thumb`).
- Status: image-binding fix built (debug), NOT yet tested in installed app. Remaining: stage timings/provider display, GPU sampling, stale-run guard audit, T01-T12 on installed app, restart check.
- Separate backlog, not done: browser, PDF viewer, chat images/charts, dataset training (DDR quota-blocked).
- No commit/push without permission.
