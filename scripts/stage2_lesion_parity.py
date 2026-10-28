"""Dev-only parity: upstream PyTorch HSMoE-AUNet vs exported ONNX on identical tensors.

Usage: parity_lesion.py <MA|HE> <checkpoint.pth> <graph.onnx> <upstream_repo_root> <out.json>
Synthetic seeded input (technical parity only, no image).
"""
import json
import sys

import numpy as np
import onnxruntime as ort
import torch

LESION, CKPT, ONNX, UPSTREAM, OUT = sys.argv[1:6]
sys.path.insert(0, UPSTREAM)
from hierarchiretina.stage2.hsmoe_aunet import get_lesion_config, load_hsmoe_aunet  # noqa: E402

torch.set_num_threads(4)
cfg = get_lesion_config(LESION)
thr = float(cfg.threshold)

model = load_hsmoe_aunet(LESION, CKPT, map_location="cpu", strict=True).eval()

gen = torch.Generator().manual_seed(0)
x = torch.randn(1, 3, 1024, 1024, generator=gen, dtype=torch.float32)

with torch.no_grad():
    ref = model(x)[0].float().numpy()

sess = ort.InferenceSession(ONNX, providers=["CPUExecutionProvider"])
got = sess.run(None, {"image": x.numpy()})[0]

diff = np.abs(ref - got)
near_zero = np.abs(ref) < 1e-3
rel = diff[~near_zero] / np.abs(ref[~near_zero])

sig_ref = 1.0 / (1.0 + np.exp(-ref))
sig_got = 1.0 / (1.0 + np.exp(-got))
m_ref = sig_ref > thr
m_got = sig_got > thr

result = {
    "lesion": LESION,
    "threshold": thr,
    "input": "seeded randn(1,3,1024,1024) float32, seed 0 (synthetic, technical parity only)",
    "logit_shape": list(ref.shape),
    "max_abs_error": float(diff.max()),
    "rmse": float(np.sqrt(np.mean(diff ** 2))),
    "relative_error_max_over_|ref|>=1e-3": float(rel.max()) if rel.size else None,
    "near_zero_excluded_pixels": int(near_zero.sum()),
    "prob_max_abs_error": float(np.abs(sig_ref - sig_got).max()),
    "mask_positive_pytorch": int(m_ref.sum()),
    "mask_positive_onnx": int(m_got.sum()),
    "mask_disagreement_pixels": int((m_ref != m_got).sum()),
    "mask_pixels_total": int(m_ref.size),
}
with open(OUT, "w") as f:
    json.dump(result, f, indent=2)
print(json.dumps(result, indent=2))
