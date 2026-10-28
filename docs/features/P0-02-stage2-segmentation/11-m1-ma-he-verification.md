# M1 — MA and HE verification (2026-10-28)

Status: **PARTIAL** for Stage II overall. MA and HE technical parity and Rust runtime pass.
Acceptance on a licensed image is BLOCKED. EX, CWS and VESSEL are not started.

## Inputs and provenance

- Upstream: HierarchiRetina v1.0, commit `bf84b1d3c7409d2575dff84c7575c67577493a30`
  (`hierarchiretina/stage2/hsmoe_aunet.py`, `LESION_CONFIGS`). Thresholds re-read from source:
  MA `0.01`, HE `0.20` (strict `>`).
- Checkpoints (copies in gitignored `models/stage2_hsmoe_aunet/{ma,he}/`, originals in `/tmp/retinacare-models/`),
  SHA-256 re-verified against `scripts/weights_manifest.json` and the ONNX manifests:
  - MA `.pth` 874,071,456 B `42c776e9…092f8` · ONNX `ac874bee…aac5`
  - HE `.pth` 762,747,290 B `c8fc44d5…371d94a` · ONNX `16f767a8…260b966`
- ONNX signature: input `image` float32 `[1,3,1024,1024]`, output `logits` float32 `[1,1,1024,1024]`.
- License: CC BY-NC 4.0 weights, MIT code.

## Step 1 — PyTorch vs ONNX parity (`scripts/stage2_lesion_parity.py`)

Identical synthetic tensor (`torch.randn`, seed 0). Technical parity only. Not an image.

| Model | max abs logit err | RMSE | max rel err (`\|ref\|≥1e-3`) | max prob err | mask disagreement | positive px (PyTorch / ONNX) |
|---|---|---|---|---|---|---|
| MA (thr 0.01) | 1.07e-4 | 2.8e-6 | 2.9e-6 | 1.3e-7 | 0 / 1,048,576 | 2 / 2 |
| HE (thr 0.20) | 1.91e-5 | 1.7e-6 | 2.7e-6 | 2.0e-7 | 0 / 1,048,576 | 0 / 0 |

Limit: the synthetic input gives almost no positive pixels. Mask agreement here is weak evidence.

## Step 2 — Rust ORT runtime on the local demo image (technical only)

Image: `public/demo-images/session-1-final/OD-1.png`, SHA-256 `32d756fc…c899`, frame 640×640.
Its provenance and license are undocumented. Image acceptance stays BLOCKED.
Reference fixtures (`*_input.f32`, `*_pytorch_logits.f32`, `*_pytorch_mask.u8`) were produced
earlier by the Codex session and are in `/tmp/retinacare-models/{ma,he}/`.

| Test (`--ignored`, CPU) | Tensor max diff | Logit max diff | Logit RMSE | Restored mask vs PyTorch | Foreground px | Time |
|---|---|---|---|---|---|---|
| `retinacare-ai-bridge` `local_ma_checkpoint_runs_to_registered_original_frame_mask` | 0 | 4.08e-4 | 6.0e-5 | exact match | 58 | ~4.7 s |
| `retinacare-ai-bridge` `local_he_checkpoint_runs_to_registered_original_frame_mask` | 0 | 4.20e-4 | 3.6e-6 | exact match | 1,574 | ~4.2 s |
| `retinacare-app` `local_stage2_ma_runs_through_nonblocking_service_and_matches_reference_mask` | — | — | — | pass | — | — |
| `retinacare-app` `local_stage2_he_runs_through_nonblocking_service_and_matches_reference_mask` | — | — | — | pass | — | — |

Provider: CPU (`ProviderRequest::Cpu`). Rust preprocessing reproduces the fixture input exactly
(max diff 0). Fixture generation was not re-audited against upstream `lesion_inference.py`.

## Gates

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `git diff --check`: pass.
- Heavy tests above: pass when run with explicit env vars. They stay ignored in default runs.

## Open for M1

1. Licensed or authorized image for formal acceptance (BLOCKED).
2. Re-audit fixture generation against upstream `lesion_inference.py`.
3. Circle-crop/stretch for MA/HE is checked only against Codex fixtures, not re-derived here.

## GPU

`nvidia-smi` now reports NVIDIA GeForce RTX 5070 Laptop GPU, driver 580.178.04, 8151 MiB.
This contradicts the earlier "driver inaccessible" memory note. CUDA ONNX execution is NOT yet verified.
