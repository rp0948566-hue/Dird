---
title: RetinaCare project overview
type: overview
domain: project
status: PARTIAL
verified_on: 2026-10-28
source_refs: ["../../../../README.md", "../../../../docs/architecture.md", "../../../../AGENTS.md"]
owner: project team
tags: [scope, prototype]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Backend/project-overview.md (sha256 f8e47b7af021); verify current state in [[active-phase]] -->
# Project overview

RetinaCare is a local research prototype migrating DIRD+ toward a native Rust
desktop workflow for diabetic-retinopathy screening. It is not a certified
medical device and has no clinical validation. Model execution does not
establish diagnostic safety, accuracy or clinical utility.

Architecture and ownership span two Git repositories; see [[Archive/gui-vault-backend-mirrors/Architecture/architecture]]
and [[Archive/gui-vault-backend-mirrors/Handoffs/agent-ownership]]. Original DIRD+ React/Tauri source remains in src/ and
src-tauri/ as migration reference. Current AI cascade and factual boundaries
are in [[Archive/gui-vault-backend-mirrors/Models/model-pipeline]].

GUI chat and plugin surfaces are engineering features, not extra HierarchiRetina
AI stages. The retinal model is a vision pipeline, not an LLM.
