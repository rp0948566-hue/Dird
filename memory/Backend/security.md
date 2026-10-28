---
title: Persistence and security boundaries
type: security
domain: security
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/security.md"
  - "../../docs/migration-progress.md"
  - "../../native/crates/retinacare-persistence/src/lib.rs"
  - "../../GUI/Hackthon dezine/rust/src/engine/export.rs"
owner: Codex/Claude backend; Antigravity GUI
tags: [security, privacy, SQLCipher]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/security.md (sha256 b3855b2255b2); verify current state in [[active-phase]] -->
# Persistence and security

Native Dird persistence uses SQLCipher and exposes typed patient/examination
operations. Existing project test records report encrypted round trips,
failure-safe password rotation and atomic .dird import. Inspect current
implementation and run focused tests before relying on these claims.

Legacy DIRD+ retains browser code and IndexedDB; do not describe the entire
repository as having eliminated browser persistence. The migration target
forbids patient data in browser storage. See AGENTS.md and migration docs.

GUI Task 05 session export has per-patient/examination filtering logic, but
audit event persistence is unavailable in the live backend. Test mock audit
rows are not a durable audit log. PDF export similarly remains unavailable.
No sensitive image or health data should enter logs, demo fixtures, or this
vault. See [[Backend/data-contracts]], [[Models/licensing-and-data]], [[Decisions/risks-and-blockers]].
