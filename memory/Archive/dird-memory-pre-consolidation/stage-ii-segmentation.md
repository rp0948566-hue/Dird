---
title: Stage II segmentation status
type: model-stage
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../docs/features/P0-02-stage2-segmentation/README.md"
  - "../../../docs/features/P0-02-stage2-segmentation/07-test-results.md"
owner: Codex/Claude backend
tags: [stage-ii, segmentation]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/stage-ii-segmentation.md (sha256 69715a572e19); verify current state in [[active-phase]] -->
# Stage II segmentation

Expected outputs: separate MA, HE, EX, CWS lesion masks and one vessel mask,
registered to the exact source image/examination. Rust typed contract and
runtime currently support MA and HE only. Both have user-authorized weights,
verified local conversion manifests, CPU parity records, full-frame mask
restore and nonblocking app-service test records. Assets live under
/tmp/retinacare-models/, not in Git or packaged models/.

EX, CWS and vessel weights are absent and not authorized. Do not run the
upstream bulk downloader. Whole Stage II acceptance remains open until all five
paths, registration and licensed-image integration pass. Evidence:
[checkpoint inventory](../../../docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md),
[test results](../../../docs/features/P0-02-stage2-segmentation/07-test-results.md),
[[Archive/dird-memory-pre-consolidation/model-inventory]], [[Archive/dird-memory-pre-consolidation/stage-iii-grading]], [[Archive/dird-memory-pre-consolidation/data-contracts]].
