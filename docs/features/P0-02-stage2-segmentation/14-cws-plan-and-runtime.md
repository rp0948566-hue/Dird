# CWS (cotton-wool spots) — research, plan and runtime (2026-10-28)

Status: **PARTIAL.** CWS runs through Rust and the nonblocking ModelService and matches the upstream reference mask exactly on the local demo image. Licensed-image acceptance BLOCKED. VESSEL not started.

## Checkpoint

- Official v1.0 asset `stage2_hsmoe_aunet_CWS_best_model.pth`, 771,587,386 B, SHA-256 `c42c9b77…5bba2`.
- No pre-existing copy was found in `models/` or `/tmp/retinacare-models/`. Downloaded from the official release and verified.
- Stored at `models/stage2_hsmoe_aunet/cws/` (gitignored). CC BY-NC 4.0.
- ONNX `stage2_cws.onnx`, 769,742,070 B, SHA-256 `6c928a75…7967882`. Deterministic-tensor parity: max abs logit 2.8e-5, threshold mask agreement 1.0.

## Upstream research (pinned `bf84b1d3`)

- Architecture: HSMoE-AUNet, CWS config (`skip 8/10/12`, `bottle 16`, `dec 12/10/8`, experts base + multiscale + bright_spot + soft_region + texture).
- Input `[1,3,1024,1024]`, ImageNet normalisation, RGB.
- Deployment (`lesion_inference.preprocess_for_deployment`): rectangle crop via `detect_retinal_area` (same as EX), aspect-preserving letterbox to 1024.
- Inference: `predict_probs(tta=True)` = mean of sigmoid for identity, H-flip, V-flip.
- Restore (`postprocess_to_full_mask`):
  1. Slice the letterbox region from the 1024 probability map.
  2. Resize to crop size (bilinear).
  3. `cws_postprocess`: binarise `prob > 0.40` (strict), 5×5 ellipse MORPH_CLOSE, 8-connected components, keep area ≥ `int(40 × max(1, crop_area / 1024²))`.
  4. Paste into a zero frame at the crop box.
- Upstream threshold 0.40 ("higher recall"); sweep optimum 0.575 is NOT deployed.

## Plan (M3-CWS)

1. Add CWS to Rust `model_runtime` expected-model table (id `hierarchiretina-stage2-cws-hsmoe-aunet`, arch `HSMoE-AUNet-CWS`, task `cotton_wool_spot_segmentation`, checkpoint SHA above).
2. Reuse EX rectangle letterbox preprocessing for CWS. Version string `hierarchiretina-v1.0-stage2-cws`.
3. Reuse EX flip TTA (3 views).
4. Add a CWS postprocess in the restore path: strict threshold, 5×5 ellipse close, 8-connected components with the scaled minimum area, all computed at crop resolution.
5. Dev fixture from upstream code (`scripts/stage2_ex_reference_fixture.py` generalised to CWS), heavy test comparing tensor, TTA probabilities and final mask.
6. Nonblocking ModelService integration test (app crate), following the MA/HE pattern.
7. Gates: fmt, Clippy `-D warnings`, workspace tests, release build, diff check.

## Scope boundary

- No VESSEL checkpoint. No fusion of masks. No Stage III. No GUI change.

## Implementation (source)

- `stage2_preprocessing.rs`: CWS shares the EX rectangle crop + letterbox. New `Stage2MaskPostprocess` and `restore_stage2_probability_mask_with`; `cws_components` applies the 5×5 ellipse closing and drops 8-connected components below `int(40 × max(1, crop_area/1024²))` at crop resolution. `restore_stage2_probability_mask` is unchanged for MA/HE/EX.
- `model_runtime.rs`: CWS expected-model entry; reuses the 3-view flip TTA; CWS selects `CwsComponents`. EX test refactored into a shared rectangle-lesion helper used by EX and CWS.
- `retinacare-app/src/models.rs`: heavy ModelService tests for EX and CWS (same helper as MA/HE).
- `scripts/stage2_rect_reference_fixture.py` (renamed from the EX script): reference fixture for EX and CWS from upstream code.
- `scripts/stage2_export_lesion_parity.py`: CWS spec.

## Verification

| Check | Result |
|---|---|
| Unit: CWS closing fills a 1 px hole, drops a 1 px component, keeps a 100 px block | pass |
| Heavy CWS (ai-bridge) on `OD-1.png` vs upstream fixture | tensor max abs 2.4e-7; 0 px mask disagreement; 400 foreground px; 640×640; ~14 s CPU |
| Heavy CWS through nonblocking `ModelService` | pass (mask equals reference) |
| Heavy MA, HE, EX (ai-bridge and app service) after changes | pass |
| `cargo fmt --all --check`, strict Clippy, release build, `git diff --check` | pass |
| `cargo test --workspace` | 63 passed, 0 failed, 10 ignored |

## Limits

- Technical only: `OD-1.png` provenance and licence are undocumented. No ground truth, no Dice/IoU, no clinical claim.
- The fixture comes from upstream code run in this session, not an independent implementation.
- Threshold 0.40 is the deployed upstream value. The sweep optimum (0.575) is not used.
- CUDA not exercised. VESSEL not started. Stage II is NOT complete.

## Result (2026-10-28)

- Implemented: CWS reuses EX rectangle letterbox + 3-view flip TTA; new `Stage2MaskPostprocess::CwsComponents` (strict `> 0.40`, 5×5 ellipse close, 8-connected components, `min_area = int(40 × max(1, crop_area/1024²))`) at crop resolution.
- Heavy CWS test on `OD-1.png` vs upstream fixture (`scripts/stage2_rect_reference_fixture.py … CWS`): tensor max abs 2.4e-7, 0 px mask disagreement, 400 foreground px, 640×640, ~14 s CPU.
- Nonblocking ModelService tests added for EX and CWS; all four lesion tests (MA, HE, EX, CWS) pass in both `retinacare-ai-bridge` and `retinacare-app` with fixture env vars.
- Unit test added: `cws_components_close_small_gaps_and_drop_small_components`.
- Gates: fmt, Clippy `-D warnings`, release build, `git diff --check` pass. `cargo test --workspace`: 63 passed, 0 failed, 11 ignored.

## File-change report

- `native/crates/retinacare-ai-bridge/src/stage2_preprocessing.rs`: CWS preprocessing dispatch, `Stage2MaskPostprocess`, `restore_stage2_probability_mask_with`, `cws_components`, unit test.
- `native/crates/retinacare-ai-bridge/src/model_runtime.rs`: CWS expected-model entry, postprocess selection, shared `verify_local_rect_lesion_runtime` (EX, CWS).
- `native/crates/retinacare-app/src/models.rs`: EX and CWS nonblocking-service heavy tests.
- `scripts/stage2_export_lesion_parity.py`: CWS spec.
- `scripts/stage2_rect_reference_fixture.py`: replaces `stage2_ex_reference_fixture.py`, takes EX or CWS.
- Docs 11–14 in `docs/features/P0-02-stage2-segmentation/`.

## Limits

- Technical only: `OD-1.png` provenance and licence undocumented. No ground-truth metrics. CUDA not exercised.
- Reference fixture generated from upstream code in this session.
