# Phase 5: product integration (research prototype, uncommitted)

Verdicts: engineering-complete CPU for M0-M1, M3 (Mode R), M4-M8; research validation pending; clinical approval not claimed.
Plan and risks: [plan.md](plan.md). Live ledger: `memory/Handoffs/p5-progress.md`.

## Milestones
| M | Result | Evidence |
|---|---|---|
| M0 audit | PASS | baseline rerun: backend 157/0/21, GUI 188/0/2 |
| M1 DB path/first run | PASS | XDG default folder 0700, db+salt 0600, pointer 0600; unit tests (cwd independence, stale pointer) |
| M2 Supabase B/C | BUILT, live BLOCKED | client, OS keyring, outbox, pull tested vs fake HTTP server + real SQLCipher; two-user RLS test needs the user's accounts |
| M3 Home | Mode R PASS; Mode C BLOCKED | live Stage I-III cards with per-fold progress; Ollama here has no `llama-server`; running the downloaded runtime was refused by the permission check |
| M4 History/Timeline | PASS | chats stored v5 and restored; timeline adds mask and explanation events |
| M5 three pages | PASS | evidence/*lesion*, *vessel*, *findings* at 1600/1024/700 |
| M6 Explainability | PASS | score histogram, compare, PDF preview, "Showing" line; Quad links |
| M7 carryover | C1 PASS, C2 PASS (defined), C3 PASS, C4 PARTIAL, C5 PARTIAL | Stage I Grad-CAM baseline 0.998319 = ONNX; five-fold Grad-CAM baseline 0.762148 = ensemble |
| M8 PDF | PASS | 5-page PDF: evidence grid + overlay, read with pdftotext/pdftoppm |
| M9 security | PARTIAL | tests below; live RLS BLOCKED |
| M10-M11 | screenshots PASS; release build PASS; Windows BLOCKED |
## Measured (CPU, demo image OD-1.png, technical only)
- Mode R cascade 85 s (GUI), five-fold Grad-CAM 87.9 s, Stage I Grad-CAM 30 s.
- CUDA (RTX 5070, healthy after reboot): Stage I GPU logit 6.386831 vs CPU 6.386447, 281 ms warm vs seconds on CPU, 2.3 GB GPU memory. Only Stage I was run on CUDA; the app still uses CPU.
## Definitions and limits
- Five-fold Grad-CAM = equal-weight mean of five per-fold Grad-CAM maps for the same five-way target. Not the gradient through one shared layer.
- Grad-CAM of a saturated gate gives tiny values (1e-6); shown in scientific notation. Occlusion for Stage I is not implemented.
- Not done: mask-channel ablation job, occlusion speed-up (only an `RETINACARE_ORT_INTRA_THREADS` knob; benchmark lost in a reboot), labelled evaluation, Windows, live Supabase.
## Tests
Backend 172 passed/0 failed/21 ignored; GUI 225/0/3 (`cargo test`); fmt, clippy -D warnings, `git diff --check` clean.
New: persistence chats (8), supabase client (8), cloud sync e2e (4), keyring (ignored, passed locally), Stage I geometry (2), provenance fold/checkpoint (1), science measurements, chat guards.
