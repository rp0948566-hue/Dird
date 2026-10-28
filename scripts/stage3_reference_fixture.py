#!/usr/bin/env python3
"""Development-only Stage III references from the pinned upstream HierarchiRetina code.

1. ``corn``: seeded synthetic logits -> single-fold and five-fold-mean decisions (small JSON; no patient data).
2. ``input``: the eight-channel LG-DRG input tensor for one image plus five raw 0/255 Stage II masks
   (``<mask_dir>/{ma,he,ex,cws,vessel}_pytorch_mask.u8``), via upstream ``load_image_and_masks`` and
   ``build_aug(train=False)``. Writes ``stage3_input.f32`` (8x512x512 float32 LE).

Usage:
  stage3_reference_fixture.py corn  <upstream_root> <out.json>
  stage3_reference_fixture.py input <upstream_root> <image> <mask_dir> <out_dir>
"""
import json
import sys
import tempfile
from pathlib import Path

import cv2
import numpy as np
import torch

mode, upstream = sys.argv[1], sys.argv[2]
sys.path.insert(0, upstream)
from hierarchiretina.stage3.corn import (  # noqa: E402
    PAPER_T,
    PAPER_TG,
    corn_from_full,
    decode_consecutive,
    lgdrg_predict,
)
from hierarchiretina.stage3.preprocessing import build_aug, load_image_and_masks  # noqa: E402

LESIONS = ["ma", "he", "ex", "cws", "vessel"]

if mode == "corn":
    rng = np.random.RandomState(0)

    def decode(full):
        return int(decode_consecutive(*corn_from_full(full), PAPER_T, PAPER_TG)[0])

    single, ensemble = [], []
    for _ in range(48):
        sev = rng.normal(0.0, 2.0, 3).astype(np.float32)
        grad = np.float32(rng.normal(1.0, 2.5))
        _, full = lgdrg_predict(torch.from_numpy(sev)[None], torch.tensor([float(grad)]))
        full = full[0].numpy()
        p1, p2, p3, pg = (float(v[0]) for v in corn_from_full(full))
        single.append({"sev": sev.tolist(), "grad": float(grad), "full": full.tolist(),
                       "p": [p1, p2, p3, pg], "grade": decode(full)})
    for _ in range(24):
        folds, fulls = [], []
        for _ in range(5):
            sev = rng.normal(0.0, 2.0, 3).astype(np.float32)
            grad = np.float32(rng.normal(1.0, 2.5))
            _, full = lgdrg_predict(torch.from_numpy(sev)[None], torch.tensor([float(grad)]))
            folds.append({"sev": sev.tolist(), "grad": float(grad)})
            fulls.append(full[0])
        mean = (sum(fulls[1:], fulls[0]) / len(fulls)).numpy()
        ensemble.append({"folds": folds, "mean_full": mean.tolist(), "grade": decode(mean)})
    Path(sys.argv[3]).write_text(json.dumps(
        {"thresholds": list(PAPER_T), "gradability_threshold": PAPER_TG,
         "single": single, "ensemble": ensemble}, indent=1) + "\n")
    print("wrote", sys.argv[3], len(single), len(ensemble))
else:
    image, mask_dir, out_dir = sys.argv[3], Path(sys.argv[4]), Path(sys.argv[5])
    with tempfile.TemporaryDirectory() as tmp:
        paths = []
        for lesion in LESIONS:
            raw = np.fromfile(mask_dir / f"{lesion}_pytorch_mask.u8", dtype=np.uint8)
            side = int(round(raw.size ** 0.5))
            assert side * side == raw.size, "mask fixture is not square"
            path = Path(tmp) / f"{lesion}.png"
            cv2.imwrite(str(path), raw.reshape(side, side))
            paths.append(str(path))
        img, masks = load_image_and_masks(image, paths)
        out = build_aug(512, 5, train=False)(image=img, **{f"mask{i}": m for i, m in enumerate(masks)})
        x = torch.cat([out["image"], torch.stack([out[f"mask{i}"] for i in range(5)], 0)], 0)
    out_dir.mkdir(parents=True, exist_ok=True)
    x.numpy().astype("<f4").tofile(out_dir / "stage3_input.f32")
    print("shape", tuple(x.shape), "mask sums", [int(x[3 + i].sum()) for i in range(5)])
