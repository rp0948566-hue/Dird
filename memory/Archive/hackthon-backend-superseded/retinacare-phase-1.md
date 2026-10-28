---
title: RetinaCare Phase 1 migration brief
type: project-brief
domain: project
status: PARTIAL
verified_on: 2026-10-28
source_refs: ["../../../docs/migration-progress.md", "../../../AGENTS.md"]
owner: project team
tags: [migration, phase-1]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/retinacare-phase-1.md (sha256 0557f65d5f8f); verify current state in [[active-phase]] -->
# RetinaCare Phase 1 migration brief

Original objective: progressively migrate DIRD+ to a native Rust egui/eframe
desktop application while preserving legacy source. Architecture target is
Rust UI/backend, local Python tooling/AI bridge, ONNX inference, SQLCipher and
native Windows packaging. Browser UI, React, Tauri and HTTP server are not the
final target architecture.

Core safeguards: failure-safe key rotation, atomic imports, no generic SQL
execution in UI, no patient data in browser storage, explicit missing-model
states, preserve AGPL notices, and do not fabricate clinical output.

Current implementation evidence and open platform gates are in
[migration progress](../../../docs/migration-progress.md), [[active-phase]],
[[Architecture/architecture]], [[Backend/security]] and [[Handoffs/next-actions]]. This initial brief is
historical; current results are not limited to the early 12-test baseline.
