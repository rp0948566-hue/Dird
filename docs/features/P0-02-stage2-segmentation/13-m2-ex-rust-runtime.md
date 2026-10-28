# M2 — EX Rust runtime and tests (2026-10-28)

Status: **PARTIAL.** EX runs through the Rust ONNX path on the local demo image and matches the upstream PyTorch reference mask exactly.
Formal licensed-image acceptance is still BLOCKED. CWS and VESSEL are not started.

## What changed (source)

- `native/crates/retinacare-ai-bridge/src/stage2_preprocessing.rs`
  - EX preprocessing: upstream `detect_retinal_area` (threshold 15, 11×11 ellipse close×3 / open×2, largest contour bounds + 10 px pad), aspect-preserving letterbox to 1024 (floored resize, centred offsets), ImageNet normalisation.
  - Emits `Stage2SpatialTransform::LesionRectLetterbox` (existing contract variant).
  - Mask restore now works from probabilities (`restore_stage2_probability_mask`): slices the letterbox region, resizes to the crop, thresholds strictly `>`, pastes into the source frame. MA/HE behaviour unchanged.
  - Helpers: `foreground_mask`, `largest_contour`, `copy_region`, `flip_planes` (shared by circle and rect detection and TTA).
- `native/crates/retinacare-ai-bridge/src/model_runtime.rs`
  - EX expected id/architecture/task/checkpoint SHA added to `infer_stage2_image`.
  - EX TTA: identity, horizontal-flip and vertical-flip sigmoid maps averaged `(a+b+c)/3` in upstream order.
  - Ignored test `local_ex_checkpoint_runs_to_registered_letterboxed_mask` (env: `RETINACARE_STAGE2_EX_{MANIFEST,IMAGE,PARITY_DIR}`).
- `scripts/stage2_rect_reference_fixture.py` (new, dev-only): builds `ex_input.f32`, `ex_pytorch_prob.f32`, `ex_pytorch_mask.u8` with upstream `preprocess_for_deployment`, `predict_probs(tta=True)`, `postprocess_to_full_mask`.
- `scripts/stage2_export_lesion_parity.py`: EX spec added (from M2 export).
- `.gitignore`: `models/stage2_hsmoe_aunet/` ignored (weights and manifests).

## Verification

| Check | Result |
|---|---|
| Unit tests (new): EX letterbox registration, letterbox restore ignores padding, flip involution | pass |
| `cargo fmt --all --check` | pass (after `cargo fmt --all`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 62 passed, 0 failed, 8 ignored |
| `cargo build --release --workspace` | pass |
| `git diff --check` | pass |
| Heavy MA, HE (ai-bridge + app service), EX | pass after refactor |
| Heavy EX on `OD-1.png` vs upstream fixture | tensor max abs 2.4e-7; mask disagreement 0 px; 8,997 foreground px; 640×640 frame; ~13 s CPU (3 forward passes) |

Stage I local test `preprocessing::tests::local_fundus_matches_python_tensor_and_typed_onnx_result` needs `RETINACARE_STAGE1_IMAGE`. It was not run in this pass.

## Limits

- Technical only. `OD-1.png` provenance and licence are undocumented.
- No licensed EX ground truth, so no Dice/IoU and no clinical claim.
- Fixture is generated from upstream code in this session, not from a second independent implementation.
- App-service heavy test for EX not added yet.
- CUDA not exercised.
