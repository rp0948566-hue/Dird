---
title: HierarchiRetina model pipeline
type: model-pipeline
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../docs/features/P0-02-stage2-segmentation/01-upstream-research.md"
owner: Codex/Claude backend
tags: [AI, pipeline]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/model-pipeline.md (sha256 993482d3fba5); verify current state in [[active-phase]] -->
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

Current implementation and assets: [[Models/stage-i-screening]],
[[Models/stage-ii-segmentation]], [[Models/stage-iii-grading]], [[Models/model-inventory]].
Threshold routes are research outputs, not diagnoses. See [[Models/licensing-and-data]].

## Update 2026-10-28
The pipeline is wired end to end in the app: one request -> Stage I gate -> (continue) MA, HE, EX, CWS, VESSEL -> fold 0 grade -> one terminal report. See [[Integration/gui-backend-integration]] for the event contract and timings.
