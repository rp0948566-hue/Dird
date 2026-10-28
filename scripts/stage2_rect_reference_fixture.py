"""Dev-only EX reference fixture from the pinned upstream deployment pipeline.

Rectangle-crop lesion models (EX, CWS). Writes <lesion>_input.f32 (1x3x1024x1024 float32 LE),
<lesion>_pytorch_prob.f32 (TTA probabilities 1024x1024) and <lesion>_pytorch_mask.u8 (source-frame
0/255 mask at the deployed threshold, with upstream CWS post-processing where configured).
Usage: stage2_rect_reference_fixture.py <image> <checkpoint> <upstream_root> <out_dir> <EX|CWS>
"""
import sys
from pathlib import Path

import cv2
import numpy as np
import torch

IMAGE, CKPT, UPSTREAM, OUT, LESION = sys.argv[1:6]
LESION = LESION.upper()
sys.path.insert(0, UPSTREAM)
from hierarchiretina.stage2.hsmoe_aunet import build_hsmoe_aunet, get_lesion_config  # noqa: E402
from hierarchiretina.stage2.lesion_inference import (  # noqa: E402
    postprocess_to_full_mask,
    predict_probs,
    preprocess_for_deployment,
)

torch.set_num_threads(4)
cfg = get_lesion_config(LESION)
img = cv2.imread(IMAGE)
assert img is not None, "image unreadable"
tensor, meta = preprocess_for_deployment(img, cfg)

# Same integrity-friendly load as scripts/stage2_export_lesion_parity.py.
state = torch.load(CKPT, map_location="cpu", weights_only=True)["model_state"]
model = build_hsmoe_aunet(LESION, pretrained=False)
model.load_state_dict(state, strict=True)
model.eval()

probs = predict_probs(model, tensor, tta=True, use_amp=False)
prob = probs[0, 0].cpu().float().numpy()
mask = postprocess_to_full_mask(prob, meta, cfg, cfg.threshold)

out = Path(OUT)
out.mkdir(parents=True, exist_ok=True)
prefix = LESION.lower()
tensor.numpy().astype("<f4").tofile(out / f"{prefix}_input.f32")
prob.astype("<f4").tofile(out / f"{prefix}_pytorch_prob.f32")
mask.astype(np.uint8).tofile(out / f"{prefix}_pytorch_mask.u8")
print("letterbox", meta["letterbox"], "crop_hw", meta["crop_hw"], "box", meta["box"])
print("positive mask px", int((mask == 255).sum()), "of", mask.size, "threshold", cfg.threshold)
