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
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/01-claude-start-here.md (sha256 55f9f8e40894); verify current state in [[active-phase]] -->
# Claude: start here

## Two-minute path

Read `AGENTS.md`, [[00-index]], this note, [[active-phase]], [[Handoffs/next-actions]],
and [[Handoffs/agent-ownership]]. For the assigned task, add [[Architecture/architecture]],
[[Models/model-pipeline]], [[Architecture/source-map]], [[Decisions/risks-and-blockers]] and one phase note.

## Ten-minute architecture path

Read [[Architecture/architecture]], [[Architecture/source-map]], [[Handoffs/agent-ownership]], [[Models/model-pipeline]],
[[Backend/data-contracts]], [[Backend/security]] and [[Decisions/risks-and-blockers]]. Then inspect only
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
Update [[active-phase]], [[Handoffs/next-actions]], [[Handoffs/changelog-and-handoffs]] and the
relevant domain note. Record limits and file paths. Never claim diagnosis,
clinical validation, GPU execution or GUI test results without evidence.

## Copy-ready startup message

Use /home/claude/Desktop/Hackthon dezine/Hackthon Backend as the canonical backend vault. Read
README, 00-index, 01-claude-start-here, active-phase, next-actions and agent-ownership.
Backend Git repo: /home/claude/Desktop/Dird (branch main). Check git status before edits.
Preserve all existing changes. Antigravity owns the GUI repo and its vault (Hackthon dezine/memory).
Implement only the assigned task, verify it, and update memory before handoff.
