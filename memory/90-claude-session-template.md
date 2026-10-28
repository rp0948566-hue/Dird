---
title: Claude session update template
type: template
domain: agent-handoff
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../AGENTS.md"]
owner: Claude
tags: [template, handoff]
---
# Session closeout template

Update only after work:

1. Record exact branch/HEAD and initial/final worktree status; preserve preexisting changes.
2. State task owner, files changed, tests actually run and outputs.
3. Record model/image hashes and license only when verified; no patient data or secrets.
4. Update [[active-phase]], [[next-actions]], relevant domain note and one
   [[changelog-and-handoffs]] entry.
5. Check every new wikilink, external path and evidence reference.
6. List unverified items, user approvals needed and next acceptance gate.
7. Do not claim completion if a required gate is pending.
