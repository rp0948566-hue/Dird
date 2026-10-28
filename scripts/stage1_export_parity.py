#!/usr/bin/env python
"""Development-only: export the authentic HierarchiRetina Stage I gate to ONNX and prove parity.

NOT part of the production runtime (the shipped application is Rust-only). Python is used here
exactly as the phase rules allow: checkpoint conversion and numerical parity verification.

Prerequisites:
  * models/stage1_convnextv2l_768_best_auc_model.pth
    sha256 89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff
    (official GitHub release v1.0; verify before loading)
  * ai_engine/.venv with torch/timm/opencv<5/onnx/onnxruntime (see docs/features/.../stage1)
  * ai_engine/.venv/hierarchiretina-src = upstream repo @ 27f49f5 (MIT code)

Outputs:
  * models/stage1_convnextv2l_768/stage1.onnx        (the loadable artifact)
  * models/stage1_convnextv2l_768/manifest.json      (manifest gate input, with provenance)
  * native/crates/retinacare-ai-bridge/fixtures/stage1_golden.json  (Rust parity golden)

Run: ai_engine/.venv/bin/python scripts/stage1_export_parity.py
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
UPSTREAM_SRC = REPO / "ai_engine/.venv/hierarchiretina-src"
sys.path.insert(0, str(UPSTREAM_SRC))

import numpy as np  # noqa: E402
import onnx  # noqa: E402
import onnxruntime as ort  # noqa: E402
import torch  # noqa: E402

CKPT = REPO / "models/stage1_convnextv2l_768_best_auc_model.pth"
EXPECTED_PTH_SHA256 = "89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff"
OUT_DIR = REPO / "models/stage1_convnextv2l_768"
ONNX_PATH = OUT_DIR / "stage1.onnx"
MANIFEST_PATH = OUT_DIR / "manifest.json"
GOLDEN_PATH = REPO / "native/crates/retinacare-ai-bridge/fixtures/stage1_golden.json"

IMG_SIZE = 768
# Upstream hierarchiretina/stage1/config.py — full float precision.
GATE_THRESHOLD = 0.2391715943813324
IMAGENET_MEAN = (0.485, 0.456, 0.406)
IMAGENET_STD = (0.229, 0.224, 0.225)
# Logit-level agreement gate between PyTorch and ONNX Runtime (fp32 CPU kernels).
PARITY_TOLERANCE = 1e-3

LCG_MUL = 1664525
LCG_ADD = 1013904223


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def lcg_input(seed: int) -> np.ndarray:
    """Deterministic float32 [1,3,768,768] in [-1, 1).

    The exact same u32 LCG runs in the Rust golden test
    (wrapping_mul / wrapping_add, then f32 arithmetic), so both sides build
    bit-identical inputs without shipping a 17 MB tensor file.
    """
    total = 1 * 3 * IMG_SIZE * IMG_SIZE
    states = np.empty(total, dtype=np.uint32)
    state = np.uint32(seed)
    for index in range(total):
        state = np.uint32((int(state) * LCG_MUL + LCG_ADD) & 0xFFFFFFFF)
        states[index] = state
    values = states.astype(np.float32) / np.float32(4294967296.0)
    values = values * np.float32(2.0) - np.float32(1.0)
    return values.reshape(1, 3, IMG_SIZE, IMG_SIZE)


def synthetic_fundus_bgr(size: int = 800) -> np.ndarray:
    """Non-medical stand-in image: bright disc on dark background + deterministic noise.

    It exercises the real `preprocess_gate` code path. It is NOT a retinal photograph
    and says nothing clinical.
    """
    yy, xx = np.mgrid[0:size, 0:size]
    radius = np.sqrt((xx - size // 2) ** 2 + (yy - size // 2) ** 2)
    disc = np.clip(220 - radius * 0.7, 0, 255)
    rng = np.random.default_rng(20261027)
    noise = rng.integers(0, 40, (size, size))
    gray = np.clip(disc + noise, 0, 255).astype(np.uint8)
    bgr = np.stack([gray, (gray * 0.85).astype(np.uint8), (gray * 0.6).astype(np.uint8)], axis=-1)
    return bgr


def preprocess_and_normalize() -> np.ndarray:
    """Upstream preprocess_gate (BGR uint8) -> RGB float32 with ImageNet normalization."""
    from hierarchiretina.stage1.preprocessing import preprocess_gate

    processed = preprocess_gate(synthetic_fundus_bgr(), target_size=IMG_SIZE)
    rgb = processed[:, :, ::-1].astype(np.float32) / 255.0
    mean = np.asarray(IMAGENET_MEAN, dtype=np.float32)
    std = np.asarray(IMAGENET_STD, dtype=np.float32)
    normalized = (rgb - mean) / std
    # HWC -> NCHW, contiguous so ORT/PyTorch see identical memory layout
    return np.ascontiguousarray(normalized.transpose(2, 0, 1)[None])


def build_model_with_ckpt():
    from hierarchiretina.stage1.config import get_preset
    from hierarchiretina.stage1.model import build_model, load_eval_weights

    cfg = get_preset("convnextv2_768")
    model = build_model(cfg, pretrained=False)  # checkpoint supplies all weights
    meta = load_eval_weights(model, str(CKPT), strict=True)
    return model.eval(), meta


def export_onnx(model: torch.nn.Module) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    dummy = torch.zeros(1, 3, IMG_SIZE, IMG_SIZE)
    kwargs = dict(
        input_names=["image"],
        output_names=["logit"],
        opset_version=17,
        do_constant_folding=True,
    )
    try:
        torch.onnx.export(model, (dummy,), str(ONNX_PATH), dynamo=False, **kwargs)
    except TypeError:  # older signature without `dynamo`
        torch.onnx.export(model, (dummy,), str(ONNX_PATH), **kwargs)
    onnx.checker.check_model(str(ONNX_PATH))


def sigmoid(value: float) -> float:
    return float(1.0 / (1.0 + np.exp(-value)))


def run_parity(model: torch.nn.Module, inputs: dict[str, np.ndarray]) -> dict:
    session = ort.InferenceSession(str(ONNX_PATH), providers=["CPUExecutionProvider"])
    assert session.get_providers()[0] == "CPUExecutionProvider", "expected CPU execution provider"
    cases = {}
    for name, array in inputs.items():
        with torch.no_grad():
            torch_logit = model(torch.from_numpy(array)).numpy()[0, 0]
        ort_logit = session.run(["logit"], {"image": array})[0][0, 0]
        diff = abs(float(torch_logit) - float(ort_logit))
        relative_diff = diff / max(abs(float(torch_logit)), 1e-8)
        torch_prob = sigmoid(float(torch_logit))
        ort_prob = sigmoid(float(ort_logit))
        cases[name] = {
            "torch_logit": float(torch_logit),
            "onnxruntime_logit": float(ort_logit),
            "max_abs_logit_diff": diff,
            "max_relative_logit_diff": relative_diff,
            "rmse_logit_diff": diff,  # Scalar output: RMSE equals absolute error.
            "torch_sigmoid": torch_prob,
            "onnxruntime_sigmoid": ort_prob,
            "torch_decision_positive": torch_prob >= GATE_THRESHOLD,
            "onnxruntime_decision_positive": ort_prob >= GATE_THRESHOLD,
        }
        if diff > PARITY_TOLERANCE:
            raise SystemExit(f"PARITY FAILED for {name}: |Δlogit| = {diff} > {PARITY_TOLERANCE}")
        if cases[name]["torch_decision_positive"] != cases[name]["onnxruntime_decision_positive"]:
            raise SystemExit(f"DECISION MISMATCH for {name}")
    return cases


def pick_threshold_straddle_inputs(
    model: torch.nn.Module,
) -> tuple[dict[str, np.ndarray], dict[str, int | None]]:
    """Deterministic parity inputs, plus a search for threshold-straddling seeds."""
    from hierarchiretina.stage1.preprocessing import preprocess_gate
    import cv2 as _cv2

    inputs: dict[str, np.ndarray] = {"preprocessed_synthetic_image": preprocess_and_normalize()}
    seeds: dict[str, int | None] = {"preprocessed_synthetic_image": None}
    # Real (legacy demo) fundus image through the authentic preprocessing:
    # strongest end-to-end evidence available without downloading datasets.
    demo = REPO / "public/demo-images/session-1-final/OD-1.png"
    if demo.is_file():
        image = _cv2.imread(str(demo))
        processed = preprocess_gate(image, target_size=IMG_SIZE)
        rgb = processed[:, :, ::-1].astype(np.float32) / 255.0
        mean = np.asarray(IMAGENET_MEAN, dtype=np.float32)
        std = np.asarray(IMAGENET_STD, dtype=np.float32)
        inputs["demo_fundus_image"] = np.ascontiguousarray(
            ((rgb - mean) / std).transpose(2, 0, 1)[None]
        )
        seeds["demo_fundus_image"] = None
    low_seed = high_seed = None
    with torch.no_grad():
        for seed in range(1, 13):
            array = lcg_input(seed * 7919)
            prob = sigmoid(float(model(torch.from_numpy(array)).numpy()[0, 0]))
            if prob < GATE_THRESHOLD and low_seed is None:
                low_seed = seed * 7919
                inputs["lcg_below_threshold"] = array
                seeds["lcg_below_threshold"] = low_seed
            if prob >= GATE_THRESHOLD and high_seed is None:
                high_seed = seed * 7919
                inputs["lcg_above_threshold"] = array
                seeds["lcg_above_threshold"] = high_seed
            if low_seed is not None and high_seed is not None:
                break
    # Always include a fixed low-logit noise case: bit-reproducible in Rust, and
    # the closest we get to the negative side without a Grade-0 fundus image
    # (every synthesised input lands sigmoid >= 0.69; threshold is 0.2392).
    if "lcg_low_logit" not in inputs:
        inputs["lcg_low_logit"] = lcg_input(197975)
        seeds["lcg_low_logit"] = 197975
    return inputs, seeds


def write_manifest(onnx_sha: str, upstream_commit: str, parity: dict) -> None:
    manifest = {
        "schema_version": 1,
        "model_id": "hierarchiretina-stage1-convnextv2l-768",
        "version": "1.0.0",
        "task": "dr_screening_gate",
        "architecture": "convnextv2_large+gem+attn_pool+mlp_head",
        "checkpoint": "stage1.onnx",
        "size_bytes": ONNX_PATH.stat().st_size,
        "sha256": onnx_sha,
        "license": "CC-BY-NC-4.0 weights; MIT code",
        "source_url": (
            "https://github.com/asifuddin01/HierarchiRetina/releases/download/v1.0/"
            "stage1_convnextv2l_768_best_auc_model.pth"
        ),
        "input": {
            "name": "image",
            "data_type": "float32",
            "shape": [1, 3, IMG_SIZE, IMG_SIZE],
            "color_space": "RGB",
            "normalization": "imagenet mean/std after upstream preprocess_gate (BGR uint8 pipeline)",
        },
        "outputs": {
            "schema": "single_logit",
            "name": "logit",
            "data_type": "float32",
            "shape": [1, 1],
            "activation": "sigmoid applied by caller",
            "gate_threshold": GATE_THRESHOLD,
            "positive_route": "DR grades 1-5 including ungradable; negative route: grade 0",
        },
        "validation": {
            "report": "docs/features/P0-01-hierarchiretina-model-loading/stage1-conversion.md",
            "upstream_checkpoint_sha256": EXPECTED_PTH_SHA256,
            "upstream_commit": upstream_commit,
            "parity_torch_vs_onnxruntime_max_abs_logit": max(
                case["max_abs_logit_diff"] for case in parity.values()
            ),
            "parity_tolerance": PARITY_TOLERANCE,
            "clinical_validation": "none in this repository; weights are CC BY-NC 4.0 (non-commercial)",
        },
    }
    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--verify-existing",
        action="store_true",
        help="compare the verified checkpoint against existing ONNX without rewriting artifacts",
    )
    args = parser.parse_args()
    if not CKPT.is_file():
        raise SystemExit(f"missing checkpoint: {CKPT}")
    actual = sha256_file(CKPT)
    if actual != EXPECTED_PTH_SHA256:
        raise SystemExit(f"checkpoint sha256 mismatch: expected {EXPECTED_PTH_SHA256}, got {actual}")
    upstream_commit = (
        __import__("subprocess")
        .check_output(["git", "-C", str(UPSTREAM_SRC), "rev-parse", "HEAD"], text=True)
        .strip()
    )
    if upstream_commit != "27f49f5ccc1d6297903dd651cef8a660eecc48ce":
        raise SystemExit(f"unexpected upstream source commit: {upstream_commit}")

    print(f"[1/4] loading checkpoint (EMA) from {CKPT.name} ...", flush=True)
    started = time.time()
    model, meta = build_model_with_ckpt()
    print(f"      loaded in {time.time() - started:.1f}s; meta: {meta}", flush=True)

    if args.verify_existing:
        print("[2/4] verifying existing ONNX artifact without rewriting it ...", flush=True)
        if not ONNX_PATH.is_file() or not MANIFEST_PATH.is_file():
            raise SystemExit("existing ONNX artifact or manifest is missing")
        manifest = json.loads(MANIFEST_PATH.read_text())
        onnx_sha = sha256_file(ONNX_PATH)
        if manifest.get("sha256") != onnx_sha:
            raise SystemExit("existing ONNX artifact hash does not match its manifest")
        if manifest.get("validation", {}).get("upstream_checkpoint_sha256") != actual:
            raise SystemExit("existing manifest names a different source checkpoint")
        if manifest.get("validation", {}).get("upstream_commit") != upstream_commit:
            raise SystemExit("existing manifest names a different upstream source revision")
        onnx.checker.check_model(str(ONNX_PATH))
        golden_file = json.loads(GOLDEN_PATH.read_text())
        if golden_file.get("onnx_sha256") != onnx_sha:
            raise SystemExit("existing Rust golden belongs to a different ONNX artifact")
        if golden_file.get("upstream_checkpoint_sha256") != actual:
            raise SystemExit("existing Rust golden belongs to a different source checkpoint")
    else:
        print("[2/4] exporting ONNX (opset 17, static 1x3x768x768) ...", flush=True)
        started = time.time()
        export_onnx(model)
        print(f"      export+checker: {time.time() - started:.1f}s", flush=True)
    onnx_size = ONNX_PATH.stat().st_size
    onnx_sha = sha256_file(ONNX_PATH)
    graph = onnx.load(str(ONNX_PATH), load_external_data=False)
    print(
        f"      verified artifact: {onnx_size} bytes, "
        f"sha256 {onnx_sha}, ir {graph.ir_version}, "
        f"inputs {[i.name for i in graph.graph.input]}, "
        f"outputs {[o.name for o in graph.graph.output]}",
        flush=True,
    )

    print("[3/4] PyTorch vs ONNX Runtime parity on identical inputs ...", flush=True)
    if args.verify_existing:
        inputs = {}
        seeds = {}
        for case in golden_file["cases"]:
            name = case["name"]
            kind = case["input"]["kind"]
            if kind == "lcg":
                seeds[name] = case["input"]["seed"]
                inputs[name] = lcg_input(seeds[name])
            elif kind == "preprocessed_synthetic_image":
                seeds[name] = None
                inputs[name] = preprocess_and_normalize()
            else:
                raise SystemExit(f"unsupported existing golden input kind: {kind}")
    else:
        inputs, seeds = pick_threshold_straddle_inputs(model)
    parity = run_parity(model, inputs)
    for name, case in parity.items():
        print(
            f"      {name}: torch_logit={case['torch_logit']:.9g} "
            f"ort_logit={case['onnxruntime_logit']:.9g} "
            f"|Δ|={case['max_abs_logit_diff']:.3e} "
            f"rel={case['max_relative_logit_diff']:.3e} "
            f"rmse={case['rmse_logit_diff']:.3e} "
            f"prob={case['onnxruntime_sigmoid']:.9g} "
            f"positive={case['onnxruntime_decision_positive']}",
            flush=True,
        )

    if args.verify_existing:
        print("[4/4] read-only verification complete; no artifacts written", flush=True)
        print("PARITY OK", flush=True)
        return 0

    print("[4/4] writing manifest + Rust golden ...", flush=True)
    write_manifest(onnx_sha, upstream_commit, parity)
    cases = []
    golden = {
        "note": (
            "Synthetic non-medical inputs only. Proves graph equivalence and threshold "
            "mechanics, NOT clinical validity. Regenerate inputs with the documented u32 LCG."
        ),
        "model_id": "hierarchiretina-stage1-convnextv2l-768",
        "onnx_sha256": onnx_sha,
        "upstream_checkpoint_sha256": EXPECTED_PTH_SHA256,
        "gate_threshold": GATE_THRESHOLD,
        "parity_tolerance": PARITY_TOLERANCE,
        "lcg": {"mul": LCG_MUL, "add": LCG_ADD, "shape": [1, 3, IMG_SIZE, IMG_SIZE]},
        "cases": cases,
    }
    for name, case in parity.items():
        cases.append(
            {
                "name": name,
                "input": (
                    {"kind": "preprocessed_synthetic_image"}
                    if seeds.get(name) is None and name != "demo_fundus_image"
                    else (
                        {"kind": "demo_fundus_image"}
                        if name == "demo_fundus_image"
                        else {"kind": "lcg", "seed": seeds[name]}
                    )
                ),
                "onnxruntime_logit": case["onnxruntime_logit"],
                "torch_logit": case["torch_logit"],
                "onnxruntime_sigmoid": case["onnxruntime_sigmoid"],
                "decision_positive": case["onnxruntime_decision_positive"],
            }
        )
    GOLDEN_PATH.write_text(json.dumps(golden, indent=2) + "\n")
    print(f"manifest -> {MANIFEST_PATH}", flush=True)
    print(f"golden   -> {GOLDEN_PATH}", flush=True)
    print("PARITY OK", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
