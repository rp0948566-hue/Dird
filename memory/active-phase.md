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
- Masks are not stored: reopening a saved exam shows the summary only.
- Previous Chats: no LLM backend, no storage (ephemeral notice, no seeded chats outside dev/tests).
- Negative route and ungradable: verified with scripted events and backend tests, not with a real image (none available).
- CUDA, Windows, licensed-image acceptance, folds 1-4/ensemble, explainability (Phase 4): not done.
- Plain-JSON session export screen shows its warning; its write path was not exercised in the final desktop session.
- Not committed or pushed. Parent HEAD `8fb2fee`, GUI HEAD `82db168` (branch `dezine`).

Next: [[Handoffs/next-actions]].
