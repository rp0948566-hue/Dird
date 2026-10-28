#!/usr/bin/env python3
"""Development-only SwinHRUNet++ reference fixture from the pinned upstream vessel pipeline.

Writes vessel_input.f32 (1x3x512x512 float32 LE), vessel_pytorch_prob.f32 (4-view TTA probabilities
512x512), vessel_pytorch_mask.u8 (source-frame 0/255 vessel mask at the deployed threshold 0.46).
Usage: stage2_vessel_reference_fixture.py <image> <checkpoint> <upstream_root> <out_dir>
"""
import sys
from pathlib import Path

import cv2
import numpy as np
import torch

IMAGE, CKPT, UPSTREAM, OUT = sys.argv[1:5]
sys.path.insert(0, UPSTREAM)
from hierarchiretina.stage2.swinhrunetpp import build_swinhrunetpp  # noqa: E402
from hierarchiretina.stage2.vessel_inference import preprocess_for_inference, to_original_resolution  # noqa: E402
from hierarchiretina.stage2.vessel_metrics import (  # noqa: E402
    MIN_COMPONENT_AREA,
    MORPH_OPEN_RADIUS,
    postprocess_mask,
    predict_tta_batch,
)
from hierarchiretina.stage2.vessel_data import generate_fov_mask  # noqa: E402

DEPLOYED_THRESHOLD = 0.46
torch.set_num_threads(4)
img = cv2.imread(IMAGE)
assert img is not None, "image unreadable"
tensor, meta = preprocess_for_inference(img, 512)

checkpoint = torch.load(CKPT, map_location="cpu", weights_only=True)
model = build_swinhrunetpp(pretrained=False, img_size=512)
model.load_state_dict(checkpoint["ema"], strict=True)
model.eval()

with torch.no_grad():
    prob = predict_tta_batch(model, tensor, True)[0, 0].float().cpu().numpy()
full_prob = to_original_resolution(prob, meta)
full_fov = generate_fov_mask(img, is_rgb=True).astype(np.float32)
mask = postprocess_mask(full_prob, DEPLOYED_THRESHOLD, MORPH_OPEN_RADIUS, MIN_COMPONENT_AREA, fov=full_fov)

out = Path(OUT)
out.mkdir(parents=True, exist_ok=True)
tensor.numpy().astype("<f4").tofile(out / "vessel_input.f32")
prob.astype("<f4").tofile(out / "vessel_pytorch_prob.f32")
(mask * 255).astype(np.uint8).tofile(out / "vessel_pytorch_mask.u8")
print("crop box", (meta["x1"], meta["y1"], meta["x2"], meta["y2"]), "positive px", int(mask.sum()), "of", mask.size)
