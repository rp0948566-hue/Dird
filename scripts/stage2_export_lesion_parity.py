#!/usr/bin/env python3
"""Development-only MA/HE/EX PyTorch -> ONNX export and CPU parity check.

The checkpoint is integrity-checked before torch deserialization. Outputs default
to /tmp so neither weights nor converted graphs enter Git or a distributable.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
UPSTREAM = REPO / "ai_engine/.venv/hierarchiretina-src"
UPSTREAM_COMMIT = "bf84b1d3c7409d2575dff84c7575c67577493a30"
MODEL_SPECS = {
    "MA": {
        "size": 874_071_456,
        "sha256": "42c776e9b1bc04f6fb267a3e325a0188a1fad54097293961f3e6be1200b092f8",
        "threshold": 0.01,
        "task": "microaneurysm_segmentation",
    },
    "HE": {
        "size": 762_747_290,
        "sha256": "c8fc44d5b14f54cb26c8418d6e40c61c283bf7daae7a1b3fd2ff31923371d94a",
        "threshold": 0.20,
        "task": "hemorrhage_segmentation",
    },
    "EX": {
        "size": 751_909_002,
        "sha256": "6a4bd1b40749475dbb36a1812e08d5caf3a4fce5d2c90892c0f53ab55e64453e",
        "threshold": 0.325,
        "task": "hard_exudate_segmentation",
    },
    "CWS": {
        "size": 771_587_386,
        "sha256": "c42c9b7754c71c474d9f58587d3fbe63e17deb5611a3f5656e76f0415bd5bba2",
        "threshold": 0.40,
        "task": "cotton_wool_spot_segmentation",
    },
}
CHECKPOINT_BASE_URL = "https://github.com/asifuddin01/HierarchiRetina/releases/download/v1.0"
INPUT_SHAPE = (1, 3, 1024, 1024)
LOGIT_TOLERANCE = 1e-3


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def checkpoint_name(model_kind: str) -> str:
    return f"stage2_hsmoe_aunet_{model_kind}_best_model.pth"


def verify_checkpoint(path: Path, model_kind: str = "MA") -> str:
    spec = MODEL_SPECS[model_kind]
    size = path.stat().st_size
    if size != spec["size"]:
        raise ValueError(f"checkpoint size mismatch: expected {spec['size']}, got {size}")
    actual = sha256_file(path)
    if actual != spec["sha256"]:
        raise ValueError(f"checkpoint SHA-256 mismatch: expected {spec['sha256']}, got {actual}")
    return actual


def deterministic_input():
    import numpy as np

    rng = np.random.default_rng(20261028)
    return np.ascontiguousarray(rng.normal(0.0, 1.0, INPUT_SHAPE).astype(np.float32))


def image_input(path: Path):
    import cv2
    import numpy as np

    image = cv2.imread(str(path), cv2.IMREAD_COLOR)
    if image is None:
        raise ValueError(f"OpenCV could not decode image: {path}")
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    _, mask = cv2.threshold(gray, 20, 255, cv2.THRESH_BINARY)
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (15, 15))
    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, kernel, iterations=3)
    mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, kernel, iterations=2)
    contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    height, width = image.shape[:2]
    x1, y1, x2, y2 = 0, 0, width, height
    if contours:
        (cx, cy), radius = cv2.minEnclosingCircle(max(contours, key=cv2.contourArea))
        cx, cy, radius = int(cx), int(cy), int(radius)
        if radius > min(height, width) * 0.1:
            x1, y1 = max(0, cx - radius), max(0, cy - radius)
            x2, y2 = min(width, cx + radius), min(height, cy + radius)
    crop = image[y1:y2, x1:x2]
    rgb = cv2.cvtColor(crop, cv2.COLOR_BGR2RGB)
    rgb = cv2.resize(rgb, (1024, 1024), interpolation=cv2.INTER_LINEAR)
    normalized = rgb.astype(np.float32) / np.float32(255.0)
    normalized = (normalized - np.array([0.485, 0.456, 0.406], dtype=np.float32))
    normalized = normalized / np.array([0.229, 0.224, 0.225], dtype=np.float32)
    tensor = np.ascontiguousarray(normalized.transpose(2, 0, 1)[None], dtype=np.float32)
    return tensor, (x1, y1, x2, y2), (width, height)


def write_manifest(out_dir: Path, onnx_path: Path, report: dict, model_kind: str) -> None:
    spec = MODEL_SPECS[model_kind]
    model_id = f"hierarchiretina-stage2-{model_kind.lower()}-hsmoe-aunet"
    manifest = {
        "schema_version": 1,
        "model_id": model_id,
        "version": "1.0.0",
        "task": spec["task"],
        "architecture": f"HSMoE-AUNet-{model_kind}",
        "checkpoint": onnx_path.name,
        "size_bytes": onnx_path.stat().st_size,
        "sha256": sha256_file(onnx_path),
        "license": "CC-BY-NC-4.0 weights; MIT code",
        "source_url": f"{CHECKPOINT_BASE_URL}/{checkpoint_name(model_kind)}",
        "input": {
            "name": "image",
            "data_type": "float32",
            "shape": list(INPUT_SHAPE),
            "color_space": "RGB",
            "normalization": f"ImageNet mean/std after HierarchiRetina {model_kind} circle crop and 1024 resize",
        },
        "outputs": {
            "schema": "main_logits",
            "name": "logits",
            "data_type": "float32",
            "shape": [1, 1, 1024, 1024],
        },
        "validation": {
            "stage": "development_conversion",
            "source_commit": UPSTREAM_COMMIT,
            "pytorch_vs_onnxruntime_cpu": report,
            "checkpoint_sha256": spec["sha256"],
            "deployed_probability_threshold": spec["threshold"],
            "threshold_operator": "greater_than",
        },
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-kind", choices=tuple(MODEL_SPECS), default="MA")
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--image", type=Path, help="optional local technical input image")
    parser.add_argument("--reuse-onnx", action="store_true", help="reuse graph only if provenance matches")
    args = parser.parse_args()
    model_kind = args.model_kind
    spec = MODEL_SPECS[model_kind]
    threshold = float(spec["threshold"])
    if args.output_dir is None:
        args.output_dir = Path("/tmp/retinacare-models") / model_kind.lower()

    checkpoint_hash = verify_checkpoint(args.checkpoint, model_kind)
    if not UPSTREAM.is_dir():
        raise SystemExit(f"pinned upstream checkout missing: {UPSTREAM}")

    import numpy as np
    import onnx
    import onnxruntime as ort
    import torch
    from torch import nn

    sys.path.insert(0, str(UPSTREAM))
    from hierarchiretina.stage2.hsmoe_aunet import build_hsmoe_aunet

    torch.set_num_threads(max(1, min(8, os.cpu_count() or 1)))
    # weights_only=True is mandatory: checkpoint is verified above and remains data-only.
    state = torch.load(args.checkpoint, map_location="cpu", weights_only=True)
    if not isinstance(state, dict) or "model_state" not in state:
        raise ValueError(f"verified {model_kind} checkpoint has unexpected top-level structure")
    if set(state) - {"model_state", "epoch", "val_dice"}:
        raise ValueError(f"verified {model_kind} checkpoint has unexpected fields")
    state_dict = state["model_state"]
    if not isinstance(state_dict, dict) or not all(
        isinstance(key, str) and isinstance(value, torch.Tensor)
        for key, value in state_dict.items()
    ):
        raise ValueError(f"{model_kind} model_state is not a tensor-only state dictionary")

    model = build_hsmoe_aunet(model_kind, pretrained=False)
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
    onnx_path = args.output_dir / f"stage2_{model_kind.lower()}.onnx"
    if args.reuse_onnx:
        graph = onnx.load(str(onnx_path), load_external_data=False)
        onnx.checker.check_model(graph)
        properties = {item.key: item.value for item in graph.metadata_props}
        if properties.get("source_commit") != UPSTREAM_COMMIT or properties.get(
            "checkpoint_sha256"
        ) != checkpoint_hash:
            raise SystemExit(
                f"existing ONNX graph provenance does not match verified {model_kind} checkpoint"
            )
    else:
        with tempfile.NamedTemporaryFile(
            prefix=f"stage2_{model_kind.lower()}_", suffix=".onnx", dir=args.output_dir, delete=False
        ) as temp:
            temp_path = Path(temp.name)
        try:
            dummy = torch.from_numpy(deterministic_input())
            with torch.inference_mode():
                torch.onnx.export(
                    wrapped,
                    dummy,
                    str(temp_path),
                    input_names=["image"],
                    output_names=["logits"],
                    opset_version=17,
                    do_constant_folding=True,
                    dynamo=False,
                )
            graph = onnx.load(str(temp_path), load_external_data=False)
            onnx.checker.check_model(graph)
            onnx.helper.set_model_props(
                graph,
                {
                    "source_commit": UPSTREAM_COMMIT,
                    "source_model": f"HSMoE-AUNet-{model_kind}",
                    "checkpoint_sha256": checkpoint_hash,
                    "weights_license": "CC BY-NC 4.0",
                    "code_license": "MIT",
                },
            )
            onnx.save_model(graph, str(temp_path))
            temp_path.replace(onnx_path)
        finally:
            temp_path.unlink(missing_ok=True)
        os.chmod(onnx_path, 0o600)

    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    if args.image:
        sample, crop_box, original_size = image_input(args.image)
    else:
        sample, crop_box, original_size = deterministic_input(), None, None
    with torch.inference_mode():
        pytorch_logits = wrapped(torch.from_numpy(sample)).numpy()
        pytorch_repeat = wrapped(torch.from_numpy(sample)).numpy()
    onnx_logits = session.run(["logits"], {"image": sample})[0]
    onnx_repeat = session.run(["logits"], {"image": sample})[0]
    if pytorch_logits.shape != (1, 1, 1024, 1024) or onnx_logits.shape != pytorch_logits.shape:
        raise SystemExit(f"unexpected output shape: torch={pytorch_logits.shape} onnx={onnx_logits.shape}")
    if not np.isfinite(pytorch_logits).all() or not np.isfinite(onnx_logits).all():
        raise SystemExit("non-finite output from PyTorch or ONNX Runtime")
    abs_diff = np.abs(pytorch_logits.astype(np.float64) - onnx_logits.astype(np.float64))
    rmse = float(np.sqrt(np.mean(abs_diff**2)))
    max_abs = float(abs_diff.max())
    if max_abs > LOGIT_TOLERANCE:
        raise SystemExit(f"logit parity failed: max abs {max_abs} > {LOGIT_TOLERANCE}")
    torch_prob = 1.0 / (1.0 + np.exp(-pytorch_logits.astype(np.float64)))
    onnx_prob = 1.0 / (1.0 + np.exp(-onnx_logits.astype(np.float64)))
    torch_mask = torch_prob > threshold
    onnx_mask = onnx_prob > threshold
    agreement = float(np.mean(torch_mask == onnx_mask))
    if not np.array_equal(torch_mask, onnx_mask):
        raise SystemExit(f"PyTorch and ONNX {model_kind} thresholded masks disagree")
    report = {
        "input_kind": "local_image" if args.image else "deterministic_tensor",
        "input_image_sha256": sha256_file(args.image) if args.image else None,
        "input_shape": list(sample.shape),
        "source_crop_xyxy": list(crop_box) if crop_box else None,
        "original_image_width_height": list(original_size) if original_size else None,
        "output_shape": list(onnx_logits.shape),
        "max_abs_logit_diff": max_abs,
        "rmse_logit_diff": rmse,
        "max_abs_probability_diff": float(np.abs(torch_prob - onnx_prob).max()),
        "threshold_mask_agreement": agreement,
        "torch_repeat_max_abs": float(np.abs(pytorch_logits - pytorch_repeat).max()),
        "onnx_repeat_max_abs": float(np.abs(onnx_logits - onnx_repeat).max()),
        "tolerance": LOGIT_TOLERANCE,
        "provider": "CPUExecutionProvider",
    }
    if args.image:
        stem = model_kind.lower()
        sample.astype("<f4", copy=False).tofile(args.output_dir / f"{stem}_input.f32")
        pytorch_logits.astype("<f4", copy=False).tofile(args.output_dir / f"{stem}_pytorch_logits.f32")
        import cv2

        x1, y1, x2, y2 = crop_box
        crop_prob = 1.0 / (1.0 + np.exp(-pytorch_logits[0, 0].astype(np.float64)))
        crop_prob = cv2.resize(
            crop_prob.astype(np.float32),
            (x2 - x1, y2 - y1),
            interpolation=cv2.INTER_LINEAR,
        )
        source_width, source_height = original_size
        mask = np.zeros((source_height, source_width), dtype=np.uint8)
        mask[y1:y2, x1:x2] = (crop_prob > threshold).astype(np.uint8) * 255
        mask.tofile(args.output_dir / f"{stem}_pytorch_mask.u8")
        for fixture in (
            f"{stem}_input.f32",
            f"{stem}_pytorch_logits.f32",
            f"{stem}_pytorch_mask.u8",
        ):
            os.chmod(args.output_dir / fixture, 0o600)
    write_manifest(args.output_dir, onnx_path, report, model_kind)
    os.chmod(args.output_dir / "manifest.json", 0o600)
    print(json.dumps({
        **report,
        "checkpoint_size_bytes": args.checkpoint.stat().st_size,
        "checkpoint_sha256": checkpoint_hash,
        "onnx_path": str(onnx_path),
        "onnx_size_bytes": onnx_path.stat().st_size,
        "onnx_sha256": sha256_file(onnx_path),
        "manifest_path": str(args.output_dir / "manifest.json"),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
