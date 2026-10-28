---
title: Stage III grading status
type: model-stage
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/features/P0-03-lg-drg-grading/README.md"
  - "../../docs/features/P0-03-lg-drg-grading/upstream-research.md"
owner: Claude backend
tags: [stage-iii, grading, lg-drg]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/stage-iii-grading.md (sha256 c453054f83be); verify current state in [[active-phase]] -->
# Stage III LG-DRG grading

**Phase 3: M0 (research, plan), M2 (eight-channel input, CORN decode) and fold 0 (M1 export, M3 single-fold Rust grading) are done and tested.** Only fold 0 is local; the five-fold ensemble, cascade wiring, CUDA and licensed-image acceptance are pending. M5A (2026-10-28): `retinacare-app` `cascade.rs` + `ModelService::analyze/cancel_analysis/try_next_analysis` run Stage I → II → III fold 0 with typed events and one terminal report (states Graded, Ungradable, NegativeRoute, Cancelled, Failed). Rust modules `stage3_input.rs` and `stage3_grading.rs` plus `ModelRuntime::infer_stage3_fold` in `retinacare-ai-bridge`. Fold 0 output is a SINGLE-FOLD TECHNICAL TEST (logits within 5.7e-6 of PyTorch on the demo image). Pinned source: HierarchiRetina v1.0 `bf84b1d3`. Details:
`Dird/docs/features/P0-03-lg-drg-grading/` ([[Models/model-inventory]] lists the six release assets).

Verified from the pinned source (not from the paper alone):

- Model: ConvNeXt V2-L image stream, 5-mask lesion encoder, cross-attention gate with learned scalar gamma, two heads: CORN severity (3 logits, grades 1-4) and gradability (1 logit). Not a five-class softmax.
- Input: 8 channels at 512×512: RGB then masks in order MA, HE, EX, CWS, VESSEL (matches [[Models/stage-ii-segmentation]] order). Preprocessing: fundus box crop (channel mean > 7, same box for masks), CLAHE LAB L (clip 2.5, 8×8), longest side to 512, zero pad, ImageNet normalisation of RGB only. Masks are binary {0,1} (`> 127`).
- Deployed decode: mean of the five folds' 5-way vectors (no test-time augmentation), CORN consecutive decode with t = (0.38, 0.60, 0.19) and gradability threshold 0.10; ungradable (Grade 5) overrides severity. Weights are EMA.
- Stage I routing: probability below 0.2391716 means Grade 0 and Stages II/III are skipped.
- Gamma-fix fold-0 file is an ablation, not part of the deployed ensemble.

Do not fabricate a grade, probabilities or heatmaps. No clinical claim. Paper ablation reports weak mask reliance, so make no causal claim about lesion masks. See
[[Models/model-pipeline]], [[Backend/data-contracts]], [[Decisions/risks-and-blockers]], [[Handoffs/next-actions]].

## Update 2026-10-28: cascade integrated, fold 0 only
- M5A done: `ModelService::analyze` runs Stage I -> five masks -> fold 0 (`retinacare-app/src/cascade.rs`). Negative Stage I route skips II/III (grade 0 from the gate); ungradable overrides severity; cancelled/failed runs never carry a grade.
- The GUI shows the result with the banner "SINGLE-FOLD TECHNICAL TEST (LG-DRG fold 0) - RESEARCH ONLY" and stores a compact summary with the exam. Demo image result: technical grade 3, gradable probability 1.000, Stage III 1.8 s on CPU.
- Folds 1-4, ensemble (M5B), CUDA, licensed-image acceptance, explainability (Phase 4) remain open. See [[Integration/gui-backend-integration]].

## Update 2026-10-28 (Phase 3: five-fold ensemble)
All five official checkpoints are local and SHA-256 verified; ONNX exports and Rust inference match PyTorch (max logit diff 8.6e-6). `grade_five_fold_ensemble` / `combine_five_folds` implement the official mean-of-probabilities decode with strict thresholds; the cascade runs the five folds one after another, cancels only between folds, and never publishes a partial ensemble. The GUI shows FiveFoldEnsemble only when all four extra fold manifests exist; stored fold-0 results keep the single-fold label. Real CPU run: grade 3 (technical, demo image), 54.9 s, 2.15 GiB. Full evidence and per-fold table: `docs/features/P3-five-fold-ensemble/README.md`. Open: licensed labelled evaluation, CUDA, Windows.
