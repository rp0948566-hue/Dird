---
title: HierarchiRetina model pipeline
type: model-pipeline
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../../../docs/features/P0-02-stage2-segmentation/01-upstream-research.md"
owner: Codex/Claude backend
tags: [AI, pipeline]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Models/model-pipeline.md (sha256 9604cc01030c); verify current state in [[active-phase]] -->
# Model pipeline

HierarchiRetina has three AI stages: (I) binary screening gate, (II) four
lesion masks plus vessel mask, (III) LG-DRG five-class grading using RGB and
the five masks. These are model stages; GUI, attribution, chat and report
export are not extra AI stages. No Stage IV is documented by the pinned model
pipeline.

```mermaid
flowchart LR
  IMG[Fundus RGB] --> S1[Stage I screening gate]
  S1 -->|positive route| S2[Stage II MA HE EX CWS plus vessel masks]
  S2 --> S3[Stage III LG-DRG grading]
  S1 -->|negative route| R[Stage I gate result]
```

Current implementation and assets: [[Archive/gui-vault-backend-mirrors/Models/stage-i-screening]],
[[Archive/gui-vault-backend-mirrors/Models/stage-ii-segmentation]], [[Archive/gui-vault-backend-mirrors/Models/stage-iii-grading]], [[Archive/gui-vault-backend-mirrors/Models/model-inventory]].
Threshold routes are research outputs, not diagnoses. See [[Archive/gui-vault-backend-mirrors/Models/licensing-and-data]].
