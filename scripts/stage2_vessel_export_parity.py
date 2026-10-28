#!/usr/bin/env python3
"""Development-only SwinHRUNet++ vessel PyTorch -> ONNX export and CPU parity check.

Deployed weights are the checkpoint's ``ema`` entry (as ``load_best_model_and_threshold`` in the
pinned upstream ``engine_vessel.py``). The checkpoint is size- and SHA-256-verified before loading,
and loaded with ``weights_only=True``. Outputs default to the persistent gitignored model directory.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
UPSTREAM = REPO / "ai_engine/.venv/hierarchiretina-src"
UPSTREAM_COMMIT = "bf84b1d3c7409d2575dff84c7575c67577493a30"
CHECKPOINT_SIZE = 441_078_919
CHECKPOINT_SHA256 = "32bad65f3a1cf099e847fd292ff5ff84199ed0882112c0a22499e423e481422c"
INPUT_SHAPE = (1, 3, 512, 512)
LOGIT_TOLERANCE = 1e-3
DEPLOYED_THRESHOLD = 0.46


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(16 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    size = args.checkpoint.stat().st_size
    if size != CHECKPOINT_SIZE:
        raise SystemExit(f"checkpoint size mismatch: expected {CHECKPOINT_SIZE}, got {size}")
    checkpoint_hash = sha256_file(args.checkpoint)
    if checkpoint_hash != CHECKPOINT_SHA256:
        raise SystemExit(f"checkpoint SHA-256 mismatch: got {checkpoint_hash}")
    if not UPSTREAM.is_dir():
        raise SystemExit(f"pinned upstream checkout missing: {UPSTREAM}")

    import numpy as np
    import onnx
    import onnxruntime as ort
    import torch
    from torch import nn

    sys.path.insert(0, str(UPSTREAM))
    from hierarchiretina.stage2.swinhrunetpp import build_swinhrunetpp

    torch.set_num_threads(max(1, min(8, os.cpu_count() or 1)))
    checkpoint = torch.load(args.checkpoint, map_location="cpu", weights_only=True)
    if not isinstance(checkpoint, dict) or "ema" not in checkpoint:
        raise SystemExit("verified vessel checkpoint has no 'ema' weights")
    state_dict = checkpoint["ema"]
    if not isinstance(state_dict, dict) or not all(
        isinstance(key, str) and isinstance(value, torch.Tensor) for key, value in state_dict.items()
    ):
        raise SystemExit("vessel EMA weights are not a tensor-only state dictionary")
    best = checkpoint.get("best", {})
    history_thr = checkpoint.get("history", {}).get("opt_thr", [])
    best_index = int(best.get("epoch", -1))
    trained_thr = float(history_thr[best_index]) if 0 <= best_index < len(history_thr) else None

    model = build_swinhrunetpp(pretrained=False, img_size=INPUT_SHAPE[-1])
    model.load_state_dict(state_dict, strict=True)
    model.eval()

    class MainLogits(nn.Module):
        def __init__(self, source):
            super().__init__()
            self.source = source

        def forward(self, image):
            result = self.source(image)
            return result[0] if isinstance(result, tuple) else result

    wrapped = MainLogits(model).eval()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    os.chmod(args.output_dir, 0o700)
    onnx_path = args.output_dir / "stage2_vessel.onnx"

    generator = torch.Generator().manual_seed(0)
    dummy = torch.randn(*INPUT_SHAPE, generator=generator, dtype=torch.float32)
    with torch.no_grad():
        reference = wrapped(dummy).float().numpy()
        torch.onnx.export(
            wrapped,
            dummy,
            str(onnx_path),
            input_names=["image"],
            output_names=["logits"],
            opset_version=17,
            do_constant_folding=True,
            dynamo=False,
        )
    graph = onnx.load(str(onnx_path), load_external_data=True)
    onnx.checker.check_model(graph)
    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    produced = session.run(None, {"image": dummy.numpy()})[0]

    diff = np.abs(reference - produced)
    prob_ref = 1.0 / (1.0 + np.exp(-reference.astype(np.float64)))
    prob_onnx = 1.0 / (1.0 + np.exp(-produced.astype(np.float64)))
    mask_ref = prob_ref >= DEPLOYED_THRESHOLD
    mask_onnx = prob_onnx >= DEPLOYED_THRESHOLD
    report = {
        "max_abs_logit_diff": float(diff.max()),
        "rmse_logit_diff": float(np.sqrt(np.mean(diff ** 2))),
        "max_abs_probability_diff": float(np.abs(prob_ref - prob_onnx).max()),
        "threshold_mask_agreement": float(np.mean(mask_ref == mask_onnx)),
        "tolerance": LOGIT_TOLERANCE,
        "provider": "CPUExecutionProvider",
        "input": "seeded randn(1,3,512,512) float32, seed 0 (synthetic, technical parity only)",
        "trained_threshold_from_checkpoint": trained_thr,
        "deployed_threshold": DEPLOYED_THRESHOLD,
        "weights": "ema",
        "onnx_sha256": sha256_file(onnx_path),
        "onnx_size_bytes": onnx_path.stat().st_size,
    }
    if report["max_abs_logit_diff"] > LOGIT_TOLERANCE:
        raise SystemExit(f"parity failed: {report}")
    manifest = {
        "schema_version": 1,
        "model_id": "hierarchiretina-stage2-vessel-swinhrunetpp",
        "version": "1.0.0",
        "task": "vessel_segmentation",
        "architecture": "SwinHRUNet++-vessel",
        "checkpoint": onnx_path.name,
        "size_bytes": report["onnx_size_bytes"],
        "sha256": report["onnx_sha256"],
        "license": "CC-BY-NC-4.0 weights; MIT code",
        "source_url": "https://github.com/asifuddin01/HierarchiRetina/releases/download/v1.0/stage2_swinhrunetpp_best_dice.pth",
        "input": {"name": "image", "data_type": "float32", "shape": list(INPUT_SHAPE),
                  "color_space": "RGB",
                  "normalization": "ImageNet mean/std after vessel FOV crop, CLAHE and green enhancement, 512 resize"},
        "outputs": {"schema": "main_logits", "name": "logits", "data_type": "float32",
                    "shape": [1, 1, 512, 512]},
        "validation": {
            "stage": "development_conversion",
            "source_commit": UPSTREAM_COMMIT,
            "pytorch_vs_onnxruntime_cpu": report,
            "checkpoint_sha256": CHECKPOINT_SHA256,
            "deployed_probability_threshold": DEPLOYED_THRESHOLD,
            "threshold_operator": "greater_than_or_equal",
        },
    }
    (args.output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
