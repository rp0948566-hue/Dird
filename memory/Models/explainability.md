---
title: Phase 4 explainability (occlusion and Grad-CAM)
type: model-feature
domain: AI
status: PARTIAL
verified_on: 2026-10-29
source_refs:
  - "../../docs/features/P4-explainability/README.md"
owner: Claude backend + GUI
tags: [phase-4, explainability, occlusion, grad-cam]
---
# Phase 4 explainability (RetinaCare feature, not an official stage)

Real attribution only: occlusion through the pinned ONNX LG-DRG sessions (one fold or the exact five-fold mean-probability target) and Grad-CAM through the pinned PyTorch EMA weights (one fold, local worker `scripts/stage3_gradcam.py`). Raw maps are stored in the encrypted DB (schema v4); colours are a view. Evidence, definitions, matrix: `docs/features/P4-explainability/README.md`.

- Code: `retinacare-ai-bridge/src/attribution.rs` (contract, geometry, occlusion engine), `stage3_explain.rs` (real-model drivers, worker), `retinacare-persistence/src/attributions.rs`, GUI `ui/explain.rs`, `engine/explain.rs`, `explain_app.rs`.
- Page: sidebar "Explainability" (after Timeline). Quad fourth panel mirrors the same stored map; its tag opens the page.
- Measured (CPU, demo image): fold-0 occlusion 65 forwards 65 s; five-fold 325 forwards 322 s, peak RSS 1.5 GiB; Grad-CAM 12 s, 5.1 GiB; baselines equal the PyTorch references. Storage: [[Backend/mask-storage]] (same database, v4).
- Not done: Stage I explanation (no exposed preprocessing geometry), five-fold Grad-CAM, CUDA, Windows, labelled evaluation, mask-channel ablation.
- Caveats: occlusion cells are 64 px; Grad-CAM under global pooling gives the same pattern with opposite sign for competing grades; a saturated target (P(gradable) 1.0) gives an empty map that the app rejects; faithfulness numbers are one image, one fold.
- Related: [[Models/stage-iii-grading]], [[Models/model-inventory]], [[Backend/security]].

## Phase 5 additions (2026-10-29)
- Stage I Grad-CAM (`scripts/stage1_gradcam.py`, `retinacare-ai-bridge/src/stage1_explain.rs`): baseline 0.998319 = ONNX; map restored via `Stage1Geometry`.
- Five-fold Grad-CAM: equal-weight mean of five per-fold maps, same five-way target. Baseline 0.762148 = ensemble. Not a gradient through one shared layer.
- Provenance checks: fold list and pinned checkpoint hash must match scope (`attribution.rs`).
- Occlusion for Stage I not implemented. Mask ablation not done.
