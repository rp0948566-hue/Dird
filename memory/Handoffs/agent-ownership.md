---
title: Repository and agent ownership
type: ownership
domain: project
status: VERIFIED
verified_on: 2026-10-28
source_refs:
  - "../../AGENTS.md"
  - "../../native/Cargo.toml"
  - "../../GUI/Hackthon dezine/rust/Cargo.toml"
owner: Codex backend; Antigravity GUI
tags: [ownership, repositories]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/agent-ownership.md (sha256 1ecd55a9cad3); verify current state in [[active-phase]] -->
# Ownership and repositories

| Workstream | Repository / branch | Owner | Guardrail |
|---|---|---|---|
| Backend, domain, persistence, reports, AI bridge, Python worker | /home/claude/Desktop/Dird, main | Codex/Claude | Work only in task-scoped backend files. |
| Desktop GUI | /home/claude/Desktop/Dird/GUI/Hackthon dezine (nested repo), dezine | Antigravity (integration adapters by Claude, user-approved 2026-10-28) | Keep the design as is; visual redesign is a separate phase. |
| Legacy DIRD+ reference | Dird src/, src-tauri/ | Existing project | Preserve until migration is explicitly verified. |
| Obsidian vault | /home/claude/Desktop/Dird/memory (ONE canonical vault; old copies in Archive/ and on disk as backups) | Claude | Edit only here. See [[00-index]]. |

Verified paths: GUI rust/Cargo.toml uses `../../../native/crates/...` path dependencies (ai-bridge, app, persistence, reports, domain). Both repositories have existing
uncommitted work. See [[Architecture/source-map]], [[Architecture/architecture]], [[Decisions/risks-and-blockers]].
