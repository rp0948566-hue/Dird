---
title: Stage I screening status
type: model-stage
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../models/stage1_convnextv2l_768/manifest.json"
owner: Codex/Claude backend
tags: [stage-i, inference]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/stage-i-screening.md (sha256 ff5335046c3f); verify current state in [[active-phase]] -->
# Stage I screening

Rust OpenCV preprocessing and authentic ONNX CPU inference exist in the
AI bridge; retinacare-app ModelService exposes a nonblocking image request/event
path. Model files are present under models/stage1_convnextv2l_768/. Manifest
records 768×768 RGB ImageNet input, single-logit output, sigmoid route threshold
0.2391715943813324, ONNX SHA
9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac, and
CC BY-NC 4.0 weights.

The previous run record reports exact Python/Rust tensor and inference parity
on one temporary 640×640 repository demo image. Its provenance and license
are unknown; this is not licensed-data or clinical acceptance. CUDA remains
unverified. See [implementation and evidence](../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md),
[[Models/model-inventory]], [[Testing/cuda-and-packaging]], [[Models/licensing-and-data]].

## Update 2026-10-28
Stage I is the first step of the live cascade. GUI demo run: score 0.9983 (threshold 0.2391716), raw logit +6.386, 3.8 s on CPU after preload. The GUI preloads the Stage I model at startup (idle GUI RSS ~1.4 GB). The score is not a calibrated probability. Real negative-route behaviour has not been seen on a real image (none available). See [[Integration/gui-backend-integration]].

## Update 2026-10-28 (P1 hardening)
Evidence: `docs/features/P1-stage1-hardening/README.md`. ONNX and .pth SHA-256 recomputed and equal to the manifest. Routing boundary proven with adjacent-float and 512-step logit sweep tests (0 boundary errors); all 9 Stage I failure kinds end `Failed`, never grade 0. CPU profile (OD-1.png): load 2.1-5.1 s, inference median 4.45 s (n=9), HWM 2.52 GB, logits bit-identical. Finding: the gate does not reject non-fundus synthetic images (black/gray/disc/noise all score 0.72-0.81 and continue); a real negative-route image has still not been seen. Licensed-image acceptance BLOCKED; CUDA BLOCKED, see [[Testing/cuda-and-packaging]].
