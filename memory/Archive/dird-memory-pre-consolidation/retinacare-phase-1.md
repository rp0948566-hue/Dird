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
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/retinacare-phase-1.md (sha256 c441f412f4a3); verify current state in [[active-phase]] -->
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
[migration progress](../../../docs/migration-progress.md), [[Archive/dird-memory-pre-consolidation/active-phase]],
[[Archive/dird-memory-pre-consolidation/architecture]], [[Archive/dird-memory-pre-consolidation/security]] and [[Archive/dird-memory-pre-consolidation/next-actions]]. This initial brief is
historical; current results are not limited to the early 12-test baseline.
