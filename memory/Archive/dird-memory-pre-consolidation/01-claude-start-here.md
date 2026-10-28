---
title: Claude read-first handoff
type: onboarding
domain: agent-handoff
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../../../AGENTS.md", "../../../native/Cargo.toml"]
owner: Claude
tags: [handoff, read-first]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/01-claude-start-here.md (sha256 bd014e16b986); verify current state in [[active-phase]] -->
# Claude: start here

## Two-minute path

Read `AGENTS.md`, [[Archive/dird-memory-pre-consolidation/00-index]], this note, [[Archive/dird-memory-pre-consolidation/active-phase]], [[Archive/dird-memory-pre-consolidation/next-actions]],
and [[Archive/dird-memory-pre-consolidation/agent-ownership]]. For the assigned task, add [[Archive/dird-memory-pre-consolidation/architecture]],
[[Archive/dird-memory-pre-consolidation/model-pipeline]], [[Archive/dird-memory-pre-consolidation/source-map]], [[Archive/dird-memory-pre-consolidation/risks-and-blockers]] and one phase note.

## Ten-minute architecture path

Read [[Archive/dird-memory-pre-consolidation/architecture]], [[Archive/dird-memory-pre-consolidation/source-map]], [[Archive/dird-memory-pre-consolidation/agent-ownership]], [[Archive/dird-memory-pre-consolidation/model-pipeline]],
[[Archive/dird-memory-pre-consolidation/data-contracts]], [[Archive/dird-memory-pre-consolidation/security]] and [[Archive/dird-memory-pre-consolidation/risks-and-blockers]]. Then inspect only
the source symbols needed for the task.

## Before editing

- Check `git status --short --branch` in every affected repository.
- Confirm owner, exact paths, current phase gate, acceptance criteria and any
  uncommitted work. Preserve changes; never reset or clean.
- Read target code and callers. Treat reports as leads; cite actual code or
  recorded test output. Do not run large model jobs for documentation work.
- Antigravity owns `/home/claude/Desktop/Hackthon dezine/rust/src/` GUI.
  Codex/Claude owns Dird `native/crates/` backend. Coordinate before crossing.

## After editing

Run focused tests and applicable full gates; report only commands actually run.
Update [[Archive/dird-memory-pre-consolidation/active-phase]], [[Archive/dird-memory-pre-consolidation/next-actions]], [[Archive/dird-memory-pre-consolidation/changelog-and-handoffs]] and the
relevant domain note. Record limits and file paths. Never claim diagnosis,
clinical validation, GPU execution or GUI test results without evidence.

## Copy-ready startup message

Use /home/claude/Desktop/Dird/memory as this repo-local project context. Read
AGENTS.md, memory/00-index.md, memory/01-claude-start-here.md,
memory/active-phase.md, memory/next-actions.md and memory/agent-ownership.md.
Then read architecture, source-map, risks and only the assigned phase note.
Check Git status before edits. Preserve all existing changes. Codex/Claude owns
the Dird backend; Antigravity owns the separate GUI repo. Do not copy or merge
memory into the configured sibling Obsidian vault until the user chooses the
canonical folder. Implement only the assigned task, verify it, and update
memory before handoff.
