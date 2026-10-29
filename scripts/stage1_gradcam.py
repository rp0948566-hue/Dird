#!/usr/bin/env python3
"""Phase 5 Grad-CAM for the Stage I screening gate (research only, local, no network).

Input: the exact 3x768x768 float32 tensor the Rust runtime feeds the Stage I ONNX session (little
endian, written by the app). The upstream checkpoint is size/SHA-256 verified before loading and the
model is built and loaded exactly as in ``stage1_export_parity.py`` (``build_model`` +
``load_eval_weights`` with the EMA shadow), which is how the deployed ONNX file was produced.

Explained score S: the screening probability sigmoid(logit) the gate thresholds at 0.2391715943813324.
Feature layer: ``backbone.forward_features(x)`` = 1536 x 24 x 24, the last spatial map before GeM and
attention pooling. alpha_k = mean_ij dS/dA_k,ij ; map = ReLU(sum_k alpha_k A_k). The signed pre-ReLU map is
returned too. Output: one JSON object on stdout (24x24 maps, score, logit, identity); nothing else.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
UPSTREAM = REPO / "ai_engine/.venv/hierarchiretina-src"
CHECKPOINT_SIZE = 1_614_202_699
CHECKPOINT_SHA256 = "89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff"
LAYER = "backbone.forward_features (ConvNeXt V2-L last stage, before GeM + attention pooling)"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(16 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True, help="3x768x768 float32 LE tensor")
    parser.add_argument("--randomize", choices=["none", "head", "backbone"], default="none",
                        help="sanity check only: re-initialise weights; the result is NOT a valid explanation")
    args = parser.parse_args()

    if args.checkpoint.stat().st_size != CHECKPOINT_SIZE or sha256_file(args.checkpoint) != CHECKPOINT_SHA256:
        raise SystemExit("checkpoint size or SHA-256 mismatch")
    if args.input.stat().st_size != 3 * 768 * 768 * 4:
        raise SystemExit("input tensor must be 3x768x768 float32")

    import numpy as np
    import torch

    sys.path.insert(0, str(UPSTREAM))
    from hierarchiretina.stage1.config import get_preset
    from hierarchiretina.stage1.model import build_model, load_eval_weights

    torch.set_num_threads(4)
    model = build_model(get_preset("convnextv2_768"), pretrained=False)
    meta = load_eval_weights(model, str(args.checkpoint), strict=True)
    model.eval()
    if args.randomize != "none":
        torch.manual_seed(0)
        targets = [model.head, model.attn, model.gem] if args.randomize == "head" else [model.backbone]
        for module in targets:
            for sub in module.modules():
                if hasattr(sub, "reset_parameters"):
                    sub.reset_parameters()

    x = torch.from_numpy(np.fromfile(args.input, dtype="<f4").reshape(1, 3, 768, 768).copy())
    if not torch.isfinite(x).all():
        raise SystemExit("input tensor is not finite")

    features = model.backbone.forward_features(x)
    features.retain_grad()
    logit = model.head(model.pool(features))
    score = torch.sigmoid(logit)[0, 0]
    model.zero_grad(set_to_none=True)
    score.backward()
    if features.grad is None:
        raise SystemExit("no gradient reached the feature layer")
    activations = features.detach()[0]
    gradients = features.grad.detach()[0]
    if activations.dim() != 3 or activations.shape[0] != 1536:
        raise SystemExit(f"unexpected feature shape {tuple(activations.shape)}")
    alpha = gradients.mean(dim=(1, 2))
    signed = (alpha[:, None, None] * activations).sum(0)
    cam = torch.relu(signed)
    if not (torch.isfinite(cam).all() and torch.isfinite(signed).all()):
        raise SystemExit("non-finite Grad-CAM")
    json.dump({
        "checkpoint_sha256": CHECKPOINT_SHA256,
        "checkpoint_epoch": meta.get("epoch"),
        "ema_applied": bool(meta.get("ema_applied")),
        "target": "stage1_dr_screening",
        "score": float(score.detach()),
        "logit": float(logit.detach()[0, 0]),
        "layer": LAYER,
        "native_width": int(cam.shape[1]),
        "native_height": int(cam.shape[0]),
        "relu_map": [float(v) for v in cam.flatten()],
        "signed_map": [float(v) for v in signed.flatten()],
        "alpha_abs_mean": float(alpha.abs().mean()),
        "randomized": args.randomize,
    }, sys.stdout)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
