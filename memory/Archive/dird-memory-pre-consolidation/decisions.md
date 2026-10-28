---
title: Durable project decisions
type: decisions
domain: engineering
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../../../AGENTS.md", "../../../native/Cargo.toml"]
owner: project team
tags: [decisions, ADR]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/decisions.md (sha256 d42b9cd9b4e3); verify current state in [[active-phase]] -->
# Decisions

- **Rust owns application/runtime inference.** Python worker is a local utility
  and conversion/parity environment, not a production model runtime. ONNX
  manifests are verified before runtime use.
- **CPU evidence is separate from CUDA claims.** CUDA provider support does
  not prove execution on RTX 5070; no hardware run is verified.
- **Model downloads require separate user authorization.** MA and HE are the
  only approved Stage II weights. EX, CWS and vessel remain pending approval.
- **Two repositories remain separate.** GUI path dependencies currently use
  the local sibling layout; do not move, merge or rewrite without coordination.
- **Antigravity owns the GUI.** Backend agent must not edit GUI source paths.
- **Clinical claims require evidence.** Never invent results, masks, diagnoses,
  heatmaps, model availability, benchmarks or certification.
- **Memory is source-on-demand.** Use [[Archive/dird-memory-pre-consolidation/00-index]], focused notes and
  [[Archive/dird-memory-pre-consolidation/source-map]]; avoid repeating large source/docs excerpts.

Evidence and blockers: [[Archive/dird-memory-pre-consolidation/active-phase]], [[Archive/dird-memory-pre-consolidation/agent-ownership]],
[[Archive/dird-memory-pre-consolidation/risks-and-blockers]].
