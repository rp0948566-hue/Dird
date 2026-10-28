# M2 — EX download, export and parity (2026-10-28)

Status: **PARTIAL.** EX checkpoint downloaded and verified. Dev-only ONNX export and PyTorch/ONNX parity pass.
Rust EX runtime, EX-specific preprocessing and EX tests are NOT started. CWS and VESSEL remain unapproved.

## Download (user-approved 2026-10-28)

- Source: `https://github.com/asifuddin01/HierarchiRetina/releases/download/v1.0/stage2_hsmoe_aunet_EX_best_model.pth`
- Size 751,909,002 B (matches upstream `scripts/weights_manifest.json`). SHA-256 `6a4bd1b4…4453e` (matches).
- Stored at `models/stage2_hsmoe_aunet/ex/` (gitignored). License CC BY-NC 4.0.

## Export and parity (`scripts/stage2_export_lesion_parity.py --model-kind EX`)

- The exporter now includes an EX spec (threshold `0.325` from upstream `LESION_CONFIGS`).
- Checkpoint loaded with `weights_only=True`, size and SHA checked first.
- Deterministic tensor input `[1,3,1024,1024]`. Opset 17. CPUExecutionProvider.
- Max abs logit diff 2.57e-5, RMSE 2.8e-6, max abs prob diff 3.3e-8, threshold mask agreement 1.0.
- ONNX `stage2_ex.onnx` 750,097,739 B, SHA-256 `2aa92099…2612207`. Manifest written beside it (gitignored).

## Limits

- Parity is on a deterministic tensor only. No image-level EX run yet.
- EX upstream inference averages identity, horizontal and vertical flip (TTA), then uses rectangle crop and letterbox.
  The Rust path must reproduce those steps before any EX mask is claimed.
- No licensed EX ground truth. No Dice/IoU reported.
