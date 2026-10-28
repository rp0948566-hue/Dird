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

Expected outputs: separate MA, HE, EX, CWS lesion masks and one vessel mask,
registered to the exact source image/examination. Rust typed contract and runtime support all five: MA, HE (circle stretch), EX, CWS (rect letterbox, flip TTA; CWS closing and component filter) and VESSEL (FOV crop, CLAHE, green blend, 4-view TTA, opening and component filter). A sequential five-mask ModelService test checks fixed order MA, HE, EX, CWS, VESSEL on one image. M1 verification (2026-10-28): see [[Models/model-inventory]] and `Dird/docs/features/P0-02-stage2-segmentation/11-m1-ma-he-verification.md`. Both have user-authorized weights,
verified local conversion manifests, CPU parity records, full-frame mask
restore and nonblocking app-service test records. Persisted gitignored copies live in `Dird/models/stage2_hsmoe_aunet/`; originals in /tmp/retinacare-models/. Neither is packaged.

EX, CWS and vessel weights are absent and not authorized. Do not run the
upstream bulk downloader. Whole Stage II acceptance remains open until all five
paths, registration and licensed-image integration pass. Evidence:
[checkpoint inventory](../../docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md),
[test results](../../docs/features/P0-02-stage2-segmentation/07-test-results.md),
[[Models/model-inventory]], [[Models/stage-iii-grading]], [[Backend/data-contracts]].
