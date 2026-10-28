---
title: Stage II segmentation status
type: model-stage
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/features/P0-02-stage2-segmentation/README.md"
  - "../../docs/features/P0-02-stage2-segmentation/07-test-results.md"
owner: Codex/Claude backend
tags: [stage-ii, segmentation]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/stage-ii-segmentation.md (sha256 b2ffdcc7a1db); verify current state in [[active-phase]] -->
# Stage II segmentation

**Status 2026-10-28: 5 of 5 models run in Rust ONNX (CPU), technical only.** MA, HE (circle stretch), EX, CWS (rect letterbox, flip TTA, closing + component filter) and VESSEL (SwinHRUNet++: FOV crop, CLAHE, green blend, 4-view TTA) each match upstream-derived reference masks on the demo image. Masks are binary 0/255, restored to the source frame (`mask size == image size`, enforced by `Stage2MaskResult::validate`).

Manifests: `Dird/models/stage2_hsmoe_aunet/{ma,he,ex,cws}/manifest.json`, `stage2_swinhrunetpp/manifest.json` (MA/HE manifests were copied from `/tmp/retinacare-models` on 2026-10-28 after sha256 equality with the ONNX files; all gitignored, never packaged).

Live in the app: the cascade runs the five models one at a time and the GUI draws them as overlays after identity verification ([[Integration/gui-backend-integration]]). Real GUI timings (release, CPU, 640x640): MA 5.2 s, HE 4.7 s, EX 13.8 s, CWS 14.1 s, vessel 5.5 s (+ model load).

Still open: licensed-image acceptance, CUDA, mask persistence (masks are not stored with exams), Windows. Do not run the upstream bulk downloader. Evidence: [checkpoint inventory](../../docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md), [test results](../../docs/features/P0-02-stage2-segmentation/07-test-results.md), [[Models/model-inventory]], [[Models/stage-iii-grading]], [[Backend/data-contracts]].
