# Upstream research — pinned HierarchiRetina v1.0 (`bf84b1d3`)

Source files read: `hierarchiretina/stage3/{model,preprocessing,corn,data,engine,cascade}.py`,
`scripts/weights_manifest.json`, `results/README.md`, `README.md`.

## Model (`stage3/model.py`)
- `LGDRG(cfg)`: timm `convnextv2_large.fcmae_ft_in22k_in1k_384` (`features_only`, last stage 1536 ch, 16×16 at 512 px), `drop_path 0.2`.
- Mask stream `LesionEncoder`: three stride-2 blocks (3×3 conv, BN, GELU; 32/64/128 ch) over 5 masks.
- `CrossAttnGate`: masks are queries, image tokens are keys/values (8 heads, width 256); `F = F_img + gamma * proj(attn)`, learned scalar `gamma` (init 0).
- BatchNorm2d(1536), global average pool, then `GradabilityHead` (1536-256-1) and `CORNHead` (1536-512-3).
- Input `(B, 8, 512, 512)`: channels 0-2 RGB, 3-7 masks. Output `(sev_logits[B,3], grad_logit[B])`.
- Checkpoints: `load_lgdrg(path, cfg, key="ema")`, `strict=True`. The five fold files hold the **EMA** weights.

## Input contract (`stage3/preprocessing.py`, `data.py`)
- Mask channel order `LESIONS = [ma, he, ex, cws, vessel]` (same as RetinaCare's Stage II order).
- Per image: `cv2.imread` (BGR) → RGB; `crop_fundus_circle`: box of pixels whose channel mean > 7 (inclusive row/col bounds); same box applied to every mask; `clahe_lab` (CLAHE clip 2.5, 8×8 tiles on LAB L).
- Masks: read grayscale PNG 0/255 at source resolution, cropped by the same box, `(m > 127)` → float32 {0,1}. Missing masks silently become zeros upstream (RetinaCare must reject instead).
- Test transform (`build_aug(train=False)`): `LongestMaxSize(512)` then `PadIfNeeded(512, 512, BORDER_CONSTANT)`, then ImageNet `Normalize` on the RGB only. Masks stay {0,1} float.
- albumentations 1.4.24 semantics (verified in the venv): `scale = 512 / max(h, w)`; new size `max(1, round(dim * scale))` (Python round); image `INTER_LINEAR`, masks `INTER_NEAREST`; pad before `int((512 - dim) / 2.0)`, after the rest; constant zero padding.

## Output and decoding (`stage3/corn.py`)
- Per fold: `p_k = sigmoid(sev_k)`, `cum = cumprod(p)`, `psev = [1-p1, p1(1-p2), p1 p2 (1-p3), p1 p2 p3]` (clamp min 1e-9, renormalise), `full[0..3] = psev * P(gradable)`, `full[4] = 1 - P(gradable)`, renormalise (`lgdrg_predict`).
- Ensemble (`engine.ensemble_probs`): mean of the five folds' 5-way `full` vectors. **No test-time augmentation in Stage III.** AMP only on CUDA.
- Deployed decode (`decode_full` = `corn_from_full` then `decode_consecutive`): thresholds **t = (0.38, 0.60, 0.19)** and **t_g = 0.10** (`PAPER_T`, `PAPER_TG`), documented as the OOF-fitted values used for every reported test number. Grade 5 (ungradable) if `P(gradable) < 0.10`; otherwise `1 + number of leading thresholds passed` (stops at first failure). `decode_count` (t=0.5) is training-time only.
- `corn_from_full` inverts the 5-way vector (clip constants `1e-9`, `1e-12`); the Rust port must reproduce it because the ensemble is decoded after averaging.
- Ungradable (Grade 5) overrides severity. Argmax of the averaged vector is a separate, non-deployed decode.

## Stage I routing (`stage3/cascade.py`)
- `GATE_THRESHOLD = 0.2391715943813324`; Stage I probability `>= tau` passes (`pred_binary = 1`). Images below are not segmented or graded and the cascade outputs **Grade 0**. RetinaCare's Stage I manifest already records the same threshold.
- Final grade = decoded Stage III grade (1-5) for routed images, 0 for blocked images. Grade 5 means ungradable, not a severity.

## Weights and gamma-fix
- Release assets (all CC BY-NC 4.0): `stage3_lgdrg_best_fold{0..4}.pt` (795,798,428 B each) and `stage3_lgdrg_gammafix_best_fold0.pt` (795,802,235 B).
- The gamma-fix file is the fold-0 retrain with `gamma_init=0.1` (paper Table V ablation). It is **not** part of the deployed five-fold ensemble. Not required.

## Open items to resolve with real evidence
- Exact PyTorch→ONNX export of the cross-attention (softmax over 256×256 tokens) and BatchNorm in eval: needs a checkpoint.
- Fold weights in the ensemble are equal (plain mean).
- Whether to keep a Rust re-implementation of `corn_from_full` identical to numpy float64 clips: unit-test against Python-generated vectors.
