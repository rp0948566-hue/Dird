# M1 and M3 — fold 0 export and single-fold Rust grading (2026-10-28)

**SINGLE-FOLD TECHNICAL TEST. NOT the final five-fold ensemble. Research prototype, no clinical claim.**

## Checkpoint (owner-approved, fold 0 only)
- Official release asset `stage3_lgdrg_best_fold0.pt`, 795,798,428 B, SHA-256 `c1593eac…f57c` (matches upstream manifest). No prior copy existed.
- Stored at `models/stage3_lgdrg/` (gitignored: `models/**/*.pt` and `models/stage3_lgdrg/`). CC BY-NC 4.0.
- Loaded with `weights_only=True` (works: keys `ema`, `cfg`, `epoch`, `best`). Deployed weights: `ema`, `strict=True`. Saved config equals upstream defaults (backbone `convnextv2_large.fcmae_ft_in22k_in1k_384`, 512 px, 5 masks, drop_path 0.2, drop_rate 0.3). Epoch 56, best macro-AUC 0.9422 (training value, not re-evaluated).
- Learned gate scalar `gamma` (EMA) = 0.0102. Consistent with the paper's finding of weak mask reliance. No causal claim about the masks.

## ONNX export (`scripts/stage3_export_parity.py`)
- Opset 17, legacy exporter, constant folding, torch 2.14.1+cpu, onnxruntime 1.31.1. Input `image` `[1,8,512,512]`, ONE output `logits` `[1,4]` = three CORN logits then the gradability logit, so the existing single-output Rust session adapter is reused.
- `stage3_lgdrg_fold0.onnx`: 795,936,525 B, SHA-256 `31f20d62…129cb7`. Manifest `manifest_fold0.json` (gitignored folder).
- PyTorch vs ONNX Runtime CPU: seeded synthetic input max abs logit diff 2.7e-6; demo-image tensor 2.9e-6. Single-fold decisions equal on both.

## Rust runtime
- `ModelRuntime::infer_stage3_fold` (new): checks model id, version, architecture, task, shapes, output schema, checkpoint SHA and pinned source commit; reads the image with the Stage II source limits; assembles the eight-channel tensor from the five typed Stage II results (rejecting missing, duplicate, stale, other-image, wrong-size masks); runs the fold; decodes with the pinned thresholds. Returns `Stage3GradeResult` (scope `SingleFoldTechnicalTest { fold }`, raw logits, full vector, ordinal probabilities, gradable probability, thresholds, provider, timing, `research_only`).
- New types in `stage3_grading.rs`: `Stage3Request`, `Stage3GradeResult`, `Stage3Scope`, `Stage3InferenceError`, `SingleFoldGrade`, `FOLD_CHECKPOINT_SHA256`.

## Verification (CPU, local demo image, technical only)
| Check | Result |
|---|---|
| Heavy `local_stage3_single_fold_grades_demo_image_and_matches_pytorch_logits` | Rust logits `[10.0475, 2.5154, -12.5487, 12.3994]`, max abs diff vs PyTorch 5.7e-6, grade 3 (equals PyTorch decision), P(gradable) 0.999996, ~1.8 s on CPU |
| Heavy eight-channel parity (rerun) | all 8 channels max abs 0 |
| Negative unit tests (bad exam id, unloaded model, bad output length, non-finite, ungradable override) | pass |
| fmt, strict Clippy, release build, `git diff --check` | pass |
| `cargo test --workspace` | 78 passed, 0 failed, 16 ignored |

Grade 3 here is the technical output of one fold on an image with undocumented provenance and masks from the verified Stage II models. It is not a diagnosis and not evidence of accuracy.

## Not done
- Folds 1-4 (one approval each), five-fold ensemble parity (M4), ModelService nonblocking Stage III and Stage I→II→III cascade (M5), CUDA, labeled evaluation, licensed-image acceptance.
