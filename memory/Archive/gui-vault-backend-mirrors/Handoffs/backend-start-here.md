---
title: Claude read-first handoff
type: onboarding
domain: agent-handoff
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../../../../AGENTS.md", "../../../../native/Cargo.toml"]
owner: Claude
tags: [handoff, read-first]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Handoffs/backend-start-here.md (sha256 abc393eebcf8); verify current state in [[active-phase]] -->
# Claude: start here

## Two-minute path

Read `AGENTS.md`, [[00-index]], this note, [[active-phase]], [[Archive/gui-vault-backend-mirrors/Handoffs/next-actions]],
and [[Archive/gui-vault-backend-mirrors/Handoffs/agent-ownership]]. For the assigned task, add [[Archive/gui-vault-backend-mirrors/Architecture/architecture]],
[[Archive/gui-vault-backend-mirrors/Models/model-pipeline]], [[Archive/gui-vault-backend-mirrors/Architecture/source-map]], [[Archive/gui-vault-backend-mirrors/Decisions/risks-and-blockers]] and one phase note.

## Ten-minute architecture path

Read [[Archive/gui-vault-backend-mirrors/Architecture/architecture]], [[Archive/gui-vault-backend-mirrors/Architecture/source-map]], [[Archive/gui-vault-backend-mirrors/Handoffs/agent-ownership]], [[Archive/gui-vault-backend-mirrors/Models/model-pipeline]],
[[Archive/gui-vault-backend-mirrors/Backend/data-contracts]], [[Archive/gui-vault-backend-mirrors/Backend/security]] and [[Archive/gui-vault-backend-mirrors/Decisions/risks-and-blockers]]. Then inspect only
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
Update [[active-phase]], [[Archive/gui-vault-backend-mirrors/Handoffs/next-actions]], [[Archive/gui-vault-backend-mirrors/Handoffs/changelog-and-handoffs]] and the
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
