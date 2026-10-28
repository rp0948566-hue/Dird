# VESSEL (SwinHRUNet++) — research, plan and runtime (2026-10-28)

Status: **PARTIAL (technical).** All five Stage II models now run through Rust ONNX on CPU and match upstream-derived reference masks on the local demo image, individually and in one sequential five-mask ModelService run. Licensed-image acceptance and RTX 5070 CUDA remain PENDING. Stage II is NOT declared complete.

## Checkpoint

- Official v1.0 asset `stage2_swinhrunetpp_best_dice.pth`, 441,078,919 B, SHA-256 `32bad65f…1422` (matches upstream `weights_manifest.json`).
- No prior copy found under `models/` or `/tmp/retinacare-models/`. Downloaded from the official release and verified.
- Stored at `models/stage2_swinhrunetpp/` (gitignored by `models/**/*.pth`). CC BY-NC 4.0.

## Upstream research (pinned `bf84b1d3`, `hierarchiretina/stage2/`)

- Architecture `swinhrunetpp.py`: SwinV2-Base window 8 (`swinv2_base_window8_256`, `img_size=512`) plus a high-resolution CNN branch (`hr_base=64`), attention-gated decoder, vessel refinement, topology block. Output main logit `[1,1,512,512]` in eval mode.
- Pre-processing (`vessel_inference.preprocess_for_inference`, `vessel_data`):
  1. FOV mask on the BGR image with `generate_fov_mask(is_rgb=True)` (quirk kept: RGB2GRAY on BGR input). Threshold 12, 25×25 ellipse close+open, largest external contour filled, erode 9×9 ellipse.
  2. BGR→RGB, LAB-L CLAHE (clip 2.0, 8×8), green blend with equalised green at strength 0.35.
  3. `fov_crop_box(margin=0.01)`: square box around the FOV bounding box, clipped.
  4. Resize image bilinear and FOV nearest to 512. Keep FOV as `fov_r > 0.5`.
  5. ImageNet normalisation.
- Inference: `predict_tta_batch`, sigmoid mean of 4 views (identity, H-flip dims[3], V-flip dims[2], HV dims[2,3]). Sum then divide by 4.
- Restore (`to_original_resolution`): multiply probability by `fov_r` at 512, bilinear-resize to the crop size, paste into a zero canvas of the original size.
- Post-process (`postprocess_mask`): threshold `prob >= 0.46` (non-strict), multiply by the full-resolution FOV (`generate_fov_mask`, same quirk), 3×3 ellipse MORPH_OPEN (radius 1), drop 8-connected components < 30 px.

## Plan (M3-VESSEL)

1. Export: load `SwinHRUNetPP(pretrained=False)` from the verified checkpoint (check key format), export main logits to ONNX (opset 17, input `image` `[1,3,512,512]`). Parity on a deterministic tensor. Use a `.venv` timm (1.0.30) as before.
2. Rust: add `Stage2ModelKind::Vessel` expected-model entry (id, arch, task, checkpoint SHA). Add FOV preprocessing (threshold 12, close/open 25×25, largest contour, erode 9×9), CLAHE on LAB-L, green blend, FOV crop box, 512 resize, normalisation. Use the existing `VesselFovSquare` transform variant.
3. Rust inference: 4-view TTA average; FOV-mask at model resolution; inverse crop back to the frame; `>= 0.46`; full-frame FOV; 3×3 open; drop components < 30 px.
4. Tests: unit tests (FOV crop box, inverse transform, boundary, empty FOV handling), heavy test against an upstream fixture on the demo image, nonblocking ModelService test.
5. Regression: MA, HE, EX, CWS heavy tests unchanged.
6. Gates: fmt, strict Clippy, workspace tests, release build, diff check.

## Scope boundary

- No artery/vein or calibre claims. No Stage III. No GUI change. No five-mask fusion unless the M4 contract is in place.

## Export and parity (`scripts/stage2_vessel_export_parity.py`)

- Deployed weights are the checkpoint's `ema` entry (upstream `load_best_model_and_threshold`). Loaded with `weights_only=True` after size and SHA checks, `strict=True`.
- Threshold stored in the checkpoint history at the best epoch: 0.4599609375. Upstream deploys the constant 0.46. RetinaCare uses 0.46 with `>=`.
- ONNX `stage2_vessel.onnx`, 449,950,205 B, SHA-256 `cc7ef0bb…5d76e0`. Opset 17, legacy exporter (`dynamo=False`), same as lesion exports.
- Seeded randn `[1,3,512,512]`: max abs logit diff 4.4e-5, RMSE 1.4e-6, max prob diff 9.0e-6, threshold mask agreement 1.0.

## Rust implementation (source)

- `native/crates/retinacare-ai-bridge/src/stage2_vessel.rs` (new): FOV mask (RGB grey weights on BGR input, kept), LAB-L CLAHE, green blend, `fov_crop_box` (including upstream's inclusive-max-as-exclusive-end quirk), 512 resize (bilinear image, nearest FOV), ImageNet normalisation, 4-view flip TTA, FOV-masked restore, `>= 0.46`, full-frame FOV, 3×3 opening, components < 30 px removed. Emits the existing `VesselFovSquare` transform (`fov_policy = hierarchiretina-v1.0-vessel-fov`).
- `stage2_preprocessing.rs`: shared `read_stage2_source` and `decode_stage2_source` (same limits as before), size-aware `flip_planes_hv` (`flip_planes` kept as a wrapper).
- `model_runtime.rs`: vessel expected-model entry; model shape check uses the kind's input size (512 vessel, 1024 lesions); vessel branch in `infer_stage2_image`; unreachable match arm removed; heavy test `local_vessel_checkpoint_runs_to_registered_fov_mask`.
- `retinacare-app/src/models.rs`: heavy vessel `ModelService` test; heavy `local_stage2_five_masks_run_sequentially_in_fixed_order` (MA, HE, EX, CWS, VESSEL; load → infer → unload one at a time; same source SHA; each mask equals its reference).
- `lib.rs`: `pub mod stage2_vessel`.
- Scripts: `scripts/stage2_vessel_export_parity.py`, `scripts/stage2_vessel_reference_fixture.py` (dev-only).

## Verification

| Check | Result |
|---|---|
| Unit: FOV box exact upstream values, empty FOV fallback, bad FOV policy rejected, small components removed | pass |
| Heavy VESSEL (ai-bridge) on `OD-1.png` vs upstream fixture | tensor max abs 0; 0 px mask disagreement; 47,521 foreground px; 640×640; ~5.6 s CPU |
| Heavy VESSEL through nonblocking `ModelService` | pass |
| Heavy five-mask sequential `ModelService` run | pass (all five masks equal references, same source SHA, sessions unloaded between models) |
| Heavy MA, HE, EX, CWS (ai-bridge) after refactor | pass |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 67 passed, 0 failed, 14 ignored |
| `cargo build --release --workspace`, `git diff --check` | pass |

Individual MA/HE/EX/CWS `ModelService` heavy tests were not re-run after the vessel changes; the five-mask `ModelService` run covers the same path with exact mask equality.

## Not done (deliberately)

- No on-disk artifact manifest or atomic mask writes (M4 persistence). The five-mask contract is checked in memory only.
- No Stage III checkpoint or 8-channel tensor builder.

## Limits

- Technical only. `OD-1.png` provenance and licence are undocumented. No ground truth, no Dice/IoU, no clinical, artery/vein or calibre claim.
- Reference fixtures are produced by upstream code in this session, not an independent implementation.
- CUDA not exercised. Stage II is NOT complete until licensed-image acceptance passes.
