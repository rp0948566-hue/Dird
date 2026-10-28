---
title: Stage I screening status
type: model-stage
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../../models/stage1_convnextv2l_768/manifest.json"
owner: Codex/Claude backend
tags: [stage-i, inference]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/stage-i-screening.md (sha256 adb2c638cf33); verify current state in [[active-phase]] -->
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
unverified. See [implementation and evidence](../../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md),
[[Archive/dird-memory-pre-consolidation/model-inventory]], [[Archive/dird-memory-pre-consolidation/cuda-and-packaging]], [[Archive/dird-memory-pre-consolidation/licensing-and-data]].
