---
title: Licensing and data provenance
type: safety
domain: licensing
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/datasets.md"
  - "../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md"
owner: project team
tags: [license, provenance, privacy]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/licensing-and-data.md (sha256 d3477372351b); verify current state in [[active-phase]] -->
# Licensing and data

HierarchiRetina release model weights are documented as CC BY-NC 4.0; use
is noncommercial and requires attribution. Upstream source-code license is
separately documented as MIT in the Stage II audit. Code license does not
grant model-weight rights. Preserve Dird AGPL notices for reused legacy code;
check asset-specific terms before distribution.

The repository demo image public/demo-images/session-1-final/OD-1.png was
approved for temporary local technical tests only. Its dataset, provenance
and usage license are not documented. It is not an authorized clinical-data
validation image. Do not publish, redistribute or put it in regression fixtures.

No patient images, names, health records, keys or checkpoint binaries belong
in this memory graph. Keep large checkpoints outside Git. See [[Backend/security]],
[[Models/model-inventory]], [[Decisions/risks-and-blockers]] and [dataset notes](../../docs/datasets.md).

## Update 2026-10-28
The app surfaces these limits itself: report banner, PDF "Limitations" section (research-only, fold 0, CPU only, CC BY-NC weights, no images embedded). The demo image `dist/demo-images/session-1-final/OD-1.png` stays "provenance unknown, technical use only". Test patients in the e2e run were synthetic.
