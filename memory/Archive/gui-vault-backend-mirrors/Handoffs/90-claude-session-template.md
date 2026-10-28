---
title: Claude session update template
type: template
domain: agent-handoff
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../../../../AGENTS.md"]
owner: Claude
tags: [template, handoff]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Handoffs/90-claude-session-template.md (sha256 831328ed6131); verify current state in [[active-phase]] -->
# Session closeout template

Update only after work:

1. Record exact branch/HEAD and initial/final worktree status; preserve preexisting changes.
2. State task owner, files changed, tests actually run and outputs.
3. Record model/image hashes and license only when verified; no patient data or secrets.
4. Update [[active-phase]], [[Archive/gui-vault-backend-mirrors/Handoffs/next-actions]], relevant domain note and one
   [[Archive/gui-vault-backend-mirrors/Handoffs/changelog-and-handoffs]] entry.
5. Check every new wikilink, external path and evidence reference.
6. List unverified items, user approvals needed and next acceptance gate.
7. Do not claim completion if a required gate is pending.
