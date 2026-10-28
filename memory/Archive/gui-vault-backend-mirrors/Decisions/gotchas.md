---
title: Verified project gotchas
type: gotchas
domain: engineering
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../../../docs/migration-progress.md"
  - "../../../../GUI/Hackthon dezine/rust/src/main.rs"
owner: all project agents
tags: [gotchas, troubleshooting]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Decisions/gotchas.md (sha256 17d83f3109df); verify current state in [[active-phase]] -->
# Verified gotchas

- Rust native image preprocessing requires OpenCV 4.x, pkg-config, clang and
  libclang development headers. Python parity tooling is not production inference.
- Stage I export script can rewrite local graph/manifest/golden files; use its
  verify-existing mode for non-mutating identity checks.
- CUDA provider compilation is not hardware inference evidence.
- Stage II runtime files live in /tmp/retinacare-models, while normal
  application resource paths point at models/. They are not automatically
  included in a package.
- GUI Cargo path dependencies break if Dird and Hackthon dezine are moved
  independently.
- GUI mock switches can return deterministic synthetic medical-looking data.
  Keep them away from real patient workflows and release configurations.
- GUI report constructor currently contains a stale checkpoint hash/runtime
  label. Check [[Archive/gui-vault-backend-mirrors/Decisions/risks-and-blockers]] before using provenance display.
- Obsidian launcher invocation in this sandbox failed with Chromium sandbox
  permission errors. UI/Graph View appearance remains unverified.
- Legacy npm build rewrites public/version.json. See repository docs before
  running legacy scripts.

See [[Archive/gui-vault-backend-mirrors/Testing/build-and-test]], [[Archive/gui-vault-backend-mirrors/Testing/cuda-and-packaging]], [[GUI/Frontend/gui-implementation]].
