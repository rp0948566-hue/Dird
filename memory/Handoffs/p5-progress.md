---
title: Phase 5 progress ledger (live)
type: status
status: PARTIAL
verified_on: 2026-10-29
---
# Phase 5 progress (uncommitted; Phase 4 commits `8581790` backend, `0f14417` GUI)

Spec: user PDF + prompts (not in repo). Plan: `docs/features/P5-product-completion/plan.md`.

## Verified in the real desktop app (Xvfb, release build, synthetic OD-1.png demo image)
- M1: first launch creates the encrypted DB in the XDG data folder (dir 0700, db+salt 0600), location pointer 0600; unit tests cover relocation/cwd independence.
- M3 Mode R: Home composer, mode menu, attach, send; live per-stage and per-fold (N of 5) progress from real backend events; result card with grounded text, thumbnails, links. Conversation stored in SQLCipher v5 and restored after restart.
- M5: Lesion Analysis, Vessel Analysis, Research Findings pages (real stored masks, measurements, wipe compare, retina crop/boundary, PNG export).
- M6/M7: Explainability gains Stage I Grad-CAM (score 0.998319 = ONNX), five-fold Grad-CAM (mean of per-fold maps, baseline 0.762148 = ensemble), score histogram, compare, PDF preview; provenance rejects wrong fold/checkpoint.
- M8: PDF dossier with segmentation evidence grid + explanation overlay (5 pages, read with pdftotext/pdftoppm).
## Built and unit-tested, not live-verified
- M2: Supabase client + keyring + outbox + pull (fake HTTP server, real encrypted store); live two-user test BLOCKED (needs user's accounts).
- Mode C: local Ollama client; the machine's Ollama has no runtime (`llama-server` missing); running the downloaded official runtime was refused by the permission classifier -> BLOCKED until the user allows it.
## Notes
- Reboot at 2026-10-29 09:23 wiped /tmp (scratchpad, /tmp/retinacare-models). GPU is healthy again after the reboot (CUDA gate can be retried).
- Related: [[Models/explainability]], [[Backend/supabase-architecture]], [[Handoffs/next-actions]].
