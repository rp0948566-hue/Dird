---
title: Active phase and verified state
type: status
status: PARTIAL
verified_on: 2026-10-28
---
# Active phase (2026-10-28): GUI + backend integrated into one native app

Verdict **A (partial features listed)**: `GUI/Hackthon dezine/rust/target/release/retinacare` runs the real Stage I -> II (5 masks) -> III fold 0 cascade, saves to an encrypted DB, writes a real PDF and audit ledger. Layout: [[Integration/repo-topology]]. API + files: [[Integration/gui-backend-integration]]. Evidence: `docs/features/GUI-BACKEND-INTEGRATION/` (README = 42 checks + 30 edge cases, screenshots, sample PDF).

## Working (real desktop proof)
One binary; Stage I score/route; 5 real masks in Quad Viewer; Fold 0 grade (technical, research-only banner); cancel at stage boundary; patient/exam persistence across restart; saved result summary restore; real PDF (opened by an independent reader); append-only audit persisted; timeline from stored records; honest empty chats; scroll-safe toolbar at 1280/1024/700.
Measured (CPU, release GUI, OD-1.png 640x640): 67-74 s end to end, peak RSS 2.95 GB, idle GUI RSS ~1.4 GB (Stage I preloaded).

## Partial / blocked
- Masks ARE stored and restored (P2, 2026-10-28): verified in the real desktop app on a synthetic exam. Labelled Dice/IoU BLOCKED. See [[Backend/mask-storage]].
- Previous Chats: no LLM backend, no storage (ephemeral notice, no seeded chats outside dev/tests).
- Negative route and ungradable: verified with scripted events and backend tests, not with a real image (none available).
- CUDA, Windows, licensed-image acceptance: not done. Folds 1-4/ensemble: done in Phase 3. Explainability: Phase 4 (see below).
- Plain-JSON session export screen shows its warning; its write path was not exercised in the final desktop session.
- Git: parent `main` `01f4cb7` (4 ahead of `Debaq/Dird`, push 403); GUI `dezine` `4814064` pushed. `AI-Manthan-2.0` `main` = `0997866` (backend + GUI combined), `backend-rust` = `01f4cb7`. Vault edits made after 2026-10-28 sync (README, `.obsidian`, state note) are uncommitted. Full table: [[Integration/project-state-sync-2026-10-28]].
- Tests (after P2, run now): backend 120 passed/0 failed/18 ignored; GUI 170 passed/0 failed/2 ignored; heavy real 7-model GUI run passed with a real five-mask save/reload; fmt, clippy -D warnings, release builds clean.

P1 Stage I hardening (2026-10-28): hashes, routing, failure paths and CPU profile VERIFIED; licensed images and CUDA BLOCKED. See [[Models/stage-i-screening]], [[Testing/cuda-and-packaging]].

P2 mask persistence (2026-10-28): engineering COMPLETE and DESKTOP-VERIFIED on a synthetic exam (save, close, relaunch, unlock, restore with no model run). Commits: parent `a3c7b27` (local, 6 ahead of Debaq/Dird main), GUI `1565326` pushed to origin `dezine`. CUDA, Windows, labelled metrics, real power-loss and full-disk BLOCKED. Evidence: `docs/features/P2-mask-persistence/README.md` and `evidence/`.

Next: [[Handoffs/next-actions]].

Phase 3 (2026-10-28): authentic FIVE-FOLD LG-DRG ensemble implemented and verified on CPU: all five checkpoints verified, ONNX and Rust parity per fold, ensemble equals the PyTorch reference, real cascade 54.9 s / 2.15 GiB, live GUI save, restart and restore on a synthetic exam. Research only. CUDA, Windows, labelled evaluation BLOCKED. Evidence: `docs/features/P3-five-fold-ensemble/README.md`, [[Models/stage-iii-grading]]. Uncommitted.

Patients page redesign (2026-10-28): Directory restyled and verified in the live app; uncommitted in the GUI repo. See [[GUI/patient-examinations]].

Phase 4 (2026-10-28): authentic explainability implemented and verified on CPU: real occlusion (one fold and the five-fold mean target, 325 forwards, 322 s in the GUI), real Grad-CAM for one fold, raw maps in the encrypted DB (schema v4), restore after restart, PDF section with the real overlay, sidebar Explainability page, Quad fourth panel. Stage I explanation, five-fold Grad-CAM, CUDA, Windows, labelled evaluation NOT done. Evidence: `docs/features/P4-explainability/README.md`, [[Models/explainability]]. Uncommitted.
