#!/usr/bin/env python3
"""Development-only Python/OpenCV and PyTorch oracle for a local Stage I image.

Write outputs only to a private temporary directory. The tensor is derived from
retinal imagery and must not be committed, published, or shared.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

import cv2
import numpy as np
import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "ai_engine/.venv/hierarchiretina-src"))
from hierarchiretina.stage1.config import GATE_THRESHOLD, IMAGENET_MEAN, IMAGENET_STD, get_preset  # noqa: E402
from hierarchiretina.stage1.model import build_model, load_eval_weights  # noqa: E402
from hierarchiretina.stage1.preprocessing import (  # noqa: E402
    apply_clahe, ben_graham, detect_retinal_bounds, green_channel_boost,
    mild_sharpen, preprocess_gate, square_retinal_crop,
)

CHECKPOINT = ROOT / "models/stage1_convnextv2l_768_best_auc_model.pth"
EXPECTED_CHECKPOINT_SHA256 = "89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path)
    parser.add_argument("--tensor-out", type=Path, required=True)
    parser.add_argument("--json-out", type=Path, required=True)
    parser.add_argument("--steps-dir", type=Path)
    args = parser.parse_args()
    encoded = args.image.read_bytes()
    if sha256_file(CHECKPOINT) != EXPECTED_CHECKPOINT_SHA256:
        raise SystemExit("local Stage I checkpoint SHA-256 does not match the pinned source")
    bgr = cv2.imdecode(np.frombuffer(encoded, dtype=np.uint8), cv2.IMREAD_COLOR)
    if bgr is None:
        raise SystemExit("OpenCV could not decode the supplied image")
    bounds = detect_retinal_bounds(bgr)
    crop = square_retinal_crop(bgr)
    interpolation = cv2.INTER_AREA if max(crop.shape[:2]) > 768 else cv2.INTER_LANCZOS4
    resized = cv2.resize(crop, (768, 768), interpolation=interpolation)
    green = green_channel_boost(resized, strength=0.25)
    ben = ben_graham(green)
    lab = apply_clahe(ben, clip=2.2, grid=8)
    processed = mild_sharpen(lab, strength=0.30)
    assert np.array_equal(processed, preprocess_gate(bgr, target_size=768))
    if args.steps_dir:
        args.steps_dir.mkdir(parents=True, exist_ok=True)
        for name, stage in (("source", bgr), ("crop", crop), ("resize", resized),
                            ("green", green), ("ben", ben), ("lab", lab), ("sharp", processed)):
            if not cv2.imwrite(str(args.steps_dir / f"{name}.png"), stage):
                raise SystemExit(f"could not write temporary {name} step")
    rgb = processed[:, :, ::-1].astype(np.float32) / np.float32(255.0)
    mean = np.asarray(IMAGENET_MEAN, dtype=np.float32)
    std = np.asarray(IMAGENET_STD, dtype=np.float32)
    tensor = np.ascontiguousarray(((rgb - mean) / std).transpose(2, 0, 1)[None], dtype="<f4")

    torch.set_num_threads(2)
    model = build_model(get_preset("convnextv2_768"), pretrained=False)
    load_eval_weights(model, str(CHECKPOINT), strict=True)
    model.eval()
    with torch.no_grad():
        logit = float(model(torch.from_numpy(tensor.copy())).numpy()[0, 0])
    score = 1.0 / (1.0 + np.exp(-logit))
    result = {
        "image_sha256": sha256(encoded),
        "width": int(bgr.shape[1]),
        "height": int(bgr.shape[0]),
        "tensor_shape": [1, 3, 768, 768],
        "opencv_version": cv2.__version__,
        "torch_version": torch.__version__,
        "checkpoint_sha256": EXPECTED_CHECKPOINT_SHA256,
        "torch_logit": logit,
        "torch_sigmoid": float(score),
        "threshold": GATE_THRESHOLD,
        "continue_cascade": bool(score >= GATE_THRESHOLD),
        "retinal_bounds": [int(value) for value in bounds] if bounds else None,
        "clinical_interpretation": "none; technical parity oracle only",
    }
    args.tensor_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    tensor.tofile(args.tensor_out)
    args.json_out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
