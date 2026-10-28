---
title: Durable project decisions
type: decisions
domain: engineering
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../../AGENTS.md", "../../native/Cargo.toml"]
owner: project team
tags: [decisions, ADR]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/decisions.md (sha256 f50032bf75e1); verify current state in [[active-phase]] -->
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
- **Memory is source-on-demand.** Use [[00-index]], focused notes and
  [[Architecture/source-map]]; avoid repeating large source/docs excerpts.

Evidence and blockers: [[active-phase]], [[Handoffs/agent-ownership]],
[[Decisions/risks-and-blockers]].

## Decisions 2026-10-28 (integration)
- **One app, separate repos.** The GUI crate links backend crates by path; nested GUI `.git` stays; parent excludes `/GUI/` locally. Commits are local (parent `45f4feb`, GUI `574f8f9`), not pushed.
- **Never fake.** Missing capability = explicit unavailable state (chats, explainability, CUDA). Failed/cancelled runs publish no grade. A grade needs a verified, completed cascade run.
- **Results are stored as a summary only** (no pixels) until a mask-artifact design is approved.
- **Scratch exams** (id >= 1e9) are never written to the database.
- **One heavy cascade at a time**; a replaced/cancelled job blocks new runs until its terminal event.
- **MA/HE manifests copied into `Dird/models`** (gitignored) so the cascade finds all seven manifests.
