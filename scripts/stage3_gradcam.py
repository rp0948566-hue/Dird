#!/usr/bin/env python3
"""Phase 4 Grad-CAM for one pinned LG-DRG fold (research only, local, no network).

Input: the exact 8x512x512 float32 tensor the Rust grader feeds the ONNX session (little endian,
file written by the app). The fold checkpoint is size/SHA-256 verified and loaded with
``weights_only=True``; deployed weights are the ``ema`` entry, as in ``stage3_export_parity.py``.

Explained score S (a probability, differentiable through the same CORN algebra as grading):
  severity:g     S = full[g-1],  g in 1..4           (5-way vector scaled by P(gradable))
  gradability    S = 1 - full[4]                      (P(gradable))
Feature layer: ``norm(gate(F_img, F_les))`` = 1536 x 16 x 16, the last spatial map before global
average pooling and both heads. alpha_k = mean_ij dS/dA_k,ij ; map = ReLU(sum_k alpha_k A_k).
Output: one JSON object on stdout (16x16 raw map, score, identity). Nothing else is printed.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
UPSTREAM = REPO / "ai_engine/.venv/hierarchiretina-src"
FOLDS = {
    0: (795_798_428, "c1593eacca1148213c89424f19c47b4997d30eaf62cdff971ed147e529ccf57c"),
    1: (795_798_428, "9efb5c932ef7142e36f4e6b0b1f170936fdf771df659dbd53a0b82fa889714eb"),
    2: (795_798_428, "2336a1a56cb82c2f6eb47eb981e57974d9f47b066382e2c2b1b9d923b6f91e0f"),
    3: (795_798_428, "67341f8973c759bcf3dcb72e1dc75058d37a4b573e511fd7e180e663a9acdb99"),
    4: (795_798_428, "239107b0c4f30cdf3aa92dd16b1bb7df564e4add5edac831ca59b85ca1ee888e"),
}
LAYER = "norm(gate(backbone_last_stage, lesion_encoder))"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(16 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_target(text: str) -> tuple[str, int]:
    if text == "gradability":
        return "gradability", 0
    kind, _, grade = text.partition(":")
    if kind != "severity" or not grade.isdigit() or not 1 <= int(grade) <= 4:
        raise SystemExit("target must be 'gradability' or 'severity:1..4'")
    return kind, int(grade)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fold", type=int, required=True, choices=sorted(FOLDS))
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True, help="8x512x512 float32 LE tensor")
    parser.add_argument("--target", required=True)
    parser.add_argument("--randomize", choices=["none", "heads", "backbone"], default="none",
                        help="sanity check only: re-initialise weights; the result is NOT a valid explanation")
    args = parser.parse_args()
    kind, grade = parse_target(args.target)

    size, expected = FOLDS[args.fold]
    if args.checkpoint.stat().st_size != size or sha256_file(args.checkpoint) != expected:
        raise SystemExit("checkpoint size or SHA-256 mismatch")
    if args.input.stat().st_size != 8 * 512 * 512 * 4:
        raise SystemExit("input tensor must be 8x512x512 float32")

    import numpy as np
    import torch

    sys.path.insert(0, str(UPSTREAM))
    from hierarchiretina.stage3.corn import lgdrg_predict
    from hierarchiretina.stage3.engine import LGDRGConfig
    from hierarchiretina.stage3.model import LGDRG

    torch.set_num_threads(4)
    checkpoint = torch.load(args.checkpoint, map_location="cpu", weights_only=True)
    state = checkpoint["ema"]
    if not all(isinstance(k, str) and isinstance(v, torch.Tensor) for k, v in state.items()):
        raise SystemExit("EMA weights are not a tensor-only state dictionary")
    model = LGDRG(LGDRGConfig(), pretrained=False)
    model.load_state_dict(state, strict=True)
    model.eval()
    if args.randomize != "none":
        torch.manual_seed(0)
        targets = [model.sev_head, model.grad_head] if args.randomize == "heads" else [model.backbone]
        for module in targets:
            for sub in module.modules():
                if hasattr(sub, "reset_parameters"):
                    sub.reset_parameters()

    x = torch.from_numpy(np.fromfile(args.input, dtype="<f4").reshape(1, 8, 512, 512).copy())
    if not torch.isfinite(x).all():
        raise SystemExit("input tensor is not finite")

    captured = {}

    def hook(_module, _inputs, output):
        output.retain_grad()
        captured["a"] = output

    handle = model.norm.register_forward_hook(hook)
    sev, grad = model(x)  # grad mode on: the gradient path must reach `norm`
    handle.remove()
    _, full = lgdrg_predict(sev, grad)
    score = full[0, grade - 1] if kind == "severity" else 1.0 - full[0, 4]
    model.zero_grad(set_to_none=True)
    score.backward()
    a = captured["a"]
    if a.grad is None:
        raise SystemExit("no gradient reached the feature layer")
    activations = a.detach()[0]            # (1536, 16, 16)
    gradients = a.grad.detach()[0]
    alpha = gradients.mean(dim=(1, 2))     # (1536,)
    cam = torch.relu((alpha[:, None, None] * activations).sum(0))
    signed = (alpha[:, None, None] * activations).sum(0)  # pre-ReLU map, kept for the signed view
    if not (torch.isfinite(cam).all() and torch.isfinite(signed).all()):
        raise SystemExit("non-finite Grad-CAM")
    json.dump({
        "fold": args.fold,
        "checkpoint_sha256": expected,
        "target": args.target,
        "score": float(score.detach()),
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
