---
title: Stage III grading status
type: model-stage
domain: AI
status: PENDING
verified_on: 2026-10-28
source_refs:
  - "../../../../docs/features/P0-02-stage2-segmentation/10-stage3-handoff.md"
owner: Codex/Claude backend
tags: [stage-iii, grading]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Models/stage-iii-grading.md (sha256 80e8f3171d81); verify current state in [[active-phase]] -->
# Stage III LG-DRG grading

No Stage III checkpoint, ONNX graph, Rust inference function or real integration
test was found. The documented upstream contract expects fundus RGB plus the
four lesion masks and vessel mask, then a five-class LG-DRG output. This is an
unverified integration target, not an available feature.

Do not fabricate a grade, probabilities, Grad-CAM or heatmap. Do not begin
until P0.2 acceptance and a separately scoped task are complete. See
[[Archive/gui-vault-backend-mirrors/Models/model-pipeline]], [[Archive/gui-vault-backend-mirrors/Models/stage-ii-segmentation]], [[Archive/gui-vault-backend-mirrors/Backend/data-contracts]],
[[Archive/gui-vault-backend-mirrors/Decisions/risks-and-blockers]].
