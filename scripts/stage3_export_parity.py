#!/usr/bin/env python3
"""Development-only LG-DRG fold PyTorch -> ONNX export and CPU parity check.

The fold checkpoint is size- and SHA-256-verified, then loaded with ``weights_only=True``. The
deployed weights are the checkpoint's ``ema`` entry (upstream ``load_lgdrg(key="ema")``,
``strict=True``). The ONNX graph has ONE output ``logits`` of shape [1, 4]: the three CORN severity
logits followed by the gradability logit, so the Rust session adapter (single output) is reused.
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
# Upstream release assets (scripts/weights_manifest.json at the pinned commit): (bytes, SHA-256).
FOLDS = {
    0: (795_798_428, "c1593eacca1148213c89424f19c47b4997d30eaf62cdff971ed147e529ccf57c"),
    1: (795_798_428, "9efb5c932ef7142e36f4e6b0b1f170936fdf771df659dbd53a0b82fa889714eb"),
    2: (795_798_428, "2336a1a56cb82c2f6eb47eb981e57974d9f47b066382e2c2b1b9d923b6f91e0f"),
    3: (795_798_428, "67341f8973c759bcf3dcb72e1dc75058d37a4b573e511fd7e180e663a9acdb99"),
    4: (795_798_428, "239107b0c4f30cdf3aa92dd16b1bb7df564e4add5edac831ca59b85ca1ee888e"),
}
INPUT_SHAPE = (1, 8, 512, 512)
LOGIT_TOLERANCE = 1e-3


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(16 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fold", type=int, required=True, choices=sorted(FOLDS))
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--real-input", type=Path, help="stage3_input.f32 from stage3_reference_fixture.py")
    args = parser.parse_args()

    size, expected_sha = FOLDS[args.fold]
    if args.checkpoint.stat().st_size != size:
        raise SystemExit(f"checkpoint size mismatch: expected {size}")
    checkpoint_hash = sha256_file(args.checkpoint)
    if checkpoint_hash != expected_sha:
        raise SystemExit(f"checkpoint SHA-256 mismatch: got {checkpoint_hash}")
    if not UPSTREAM.is_dir():
        raise SystemExit(f"pinned upstream checkout missing: {UPSTREAM}")

    import numpy as np
    import onnx
    import onnxruntime as ort
    import torch
    from torch import nn

    sys.path.insert(0, str(UPSTREAM))
    from hierarchiretina.stage3.corn import PAPER_T, PAPER_TG, decode_full, lgdrg_predict
    from hierarchiretina.stage3.engine import LGDRGConfig
    from hierarchiretina.stage3.model import LGDRG

    torch.set_num_threads(max(1, min(8, os.cpu_count() or 1)))
    checkpoint = torch.load(args.checkpoint, map_location="cpu", weights_only=True)
    if not isinstance(checkpoint, dict) or "ema" not in checkpoint:
        raise SystemExit("verified fold checkpoint has no 'ema' weights")
    state = checkpoint["ema"]
    if not all(isinstance(k, str) and isinstance(v, torch.Tensor) for k, v in state.items()):
        raise SystemExit("EMA weights are not a tensor-only state dictionary")
    cfg = LGDRGConfig()
    saved = checkpoint.get("cfg", {})
    for key in ("backbone", "img_size", "n_masks", "drop_path", "drop_rate"):
        if saved.get(key) != getattr(cfg, key):
            raise SystemExit(f"checkpoint cfg {key}={saved.get(key)!r} differs from upstream default")

    model = LGDRG(cfg, pretrained=False)
    model.load_state_dict(state, strict=True)
    model.eval()

    class Logits(nn.Module):
        """[1,8,512,512] -> [1,4]: three CORN logits then the gradability logit."""

        def __init__(self, source):
            super().__init__()
            self.source = source

        def forward(self, x):
            severity, gradability = self.source(x)
            return torch.cat([severity, gradability.unsqueeze(1)], dim=1)

    wrapped = Logits(model).eval()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    os.chmod(args.output_dir, 0o700)
    onnx_path = args.output_dir / f"stage3_lgdrg_fold{args.fold}.onnx"

    generator = torch.Generator().manual_seed(0)
    random_input = torch.randn(*INPUT_SHAPE, generator=generator, dtype=torch.float32)
    random_input[:, 3:] = (random_input[:, 3:] > 1.0).float()
    inputs = {"seeded synthetic (randn RGB, random binary masks)": random_input}
    if args.real_input:
        raw = np.fromfile(args.real_input, dtype="<f4").reshape(INPUT_SHAPE)
        inputs["demo image eight-channel tensor (technical only)"] = torch.from_numpy(raw.copy())

    with torch.inference_mode():
        torch.onnx.export(
            wrapped,
            random_input,
            str(onnx_path),
            input_names=["image"],
            output_names=["logits"],
            opset_version=17,
            do_constant_folding=True,
            dynamo=False,
        )
    onnx.checker.check_model(onnx.load(str(onnx_path), load_external_data=True))
    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])

    cases = []
    for name, tensor in inputs.items():
        with torch.inference_mode():
            reference = wrapped(tensor).numpy()
        produced = session.run(None, {"image": tensor.numpy()})[0]
        diff = np.abs(reference - produced)
        sev_r, grad_r = torch.from_numpy(reference[:, :3]), torch.from_numpy(reference[:, 3])
        sev_o, grad_o = torch.from_numpy(produced[:, :3]), torch.from_numpy(produced[:, 3])
        _, full_r = lgdrg_predict(sev_r, grad_r)
        _, full_o = lgdrg_predict(sev_o, grad_o)
        cases.append({
            "input": name,
            "pytorch_logits": reference[0].tolist(),
            "onnx_logits": produced[0].tolist(),
            "max_abs_logit_diff": float(diff.max()),
            "max_abs_full_vector_diff": float((full_r - full_o).abs().max()),
            "single_fold_grade_pytorch": int(decode_full(full_r.numpy())[0]),
            "single_fold_grade_onnx": int(decode_full(full_o.numpy())[0]),
        })
    worst = max(case["max_abs_logit_diff"] for case in cases)
    if worst > LOGIT_TOLERANCE:
        raise SystemExit(f"parity failed: {cases}")
    if any(c["single_fold_grade_pytorch"] != c["single_fold_grade_onnx"] for c in cases):
        raise SystemExit(f"decision mismatch: {cases}")

    report = {
        "fold": args.fold,
        "weights": "ema",
        "checkpoint_epoch": checkpoint.get("epoch"),
        "checkpoint_best_macro_auc": checkpoint.get("best"),
        "gate_gamma_ema": float(state["gate.gamma"]),
        "tolerance": LOGIT_TOLERANCE,
        "provider": "CPUExecutionProvider",
        "opset": 17,
        "torch": torch.__version__,
        "onnxruntime": ort.__version__,
        "cases": cases,
        "onnx_sha256": sha256_file(onnx_path),
        "onnx_size_bytes": onnx_path.stat().st_size,
    }
    manifest = {
        "schema_version": 1,
        "model_id": f"hierarchiretina-stage3-lgdrg-fold{args.fold}",
        "version": "1.0.0",
        "task": "lg_drg_grading",
        "architecture": "LG-DRG-ConvNeXtV2-L",
        "checkpoint": onnx_path.name,
        "size_bytes": report["onnx_size_bytes"],
        "sha256": report["onnx_sha256"],
        "license": "CC-BY-NC-4.0 weights; MIT code",
        "source_url": f"https://github.com/asifuddin01/HierarchiRetina/releases/download/v1.0/stage3_lgdrg_best_fold{args.fold}.pt",
        "input": {"name": "image", "data_type": "float32", "shape": list(INPUT_SHAPE), "color_space": "RGB+5 masks",
                  "normalization": "ImageNet RGB after fundus crop, CLAHE, 512 letterbox; masks binary 0/1 in order MA HE EX CWS VESSEL"},
        "outputs": {"schema": "lgdrg_logits", "name": "logits", "data_type": "float32", "shape": [1, 4]},
        "validation": {
            "stage": "development_conversion",
            "source_commit": UPSTREAM_COMMIT,
            "pytorch_vs_onnxruntime_cpu": report,
            "checkpoint_sha256": expected_sha,
            "fold": args.fold,
            "corn_thresholds": list(PAPER_T),
            "gradability_threshold": PAPER_TG,
            "single_fold_technical_test": True,
        },
    }
    (args.output_dir / f"manifest_fold{args.fold}.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "cases"}, indent=2))
    for case in cases:
        print(case["input"], "max_abs_logit_diff", case["max_abs_logit_diff"],
              "grade", case["single_fold_grade_pytorch"], case["single_fold_grade_onnx"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
