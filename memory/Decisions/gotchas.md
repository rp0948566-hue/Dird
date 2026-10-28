---
title: Verified project gotchas
type: gotchas
domain: engineering
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../docs/migration-progress.md"
  - "../../GUI/Hackthon dezine/rust/src/main.rs"
owner: all project agents
tags: [gotchas, troubleshooting]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/gotchas.md (sha256 249726c02736); verify current state in [[active-phase]] -->
# Verified gotchas

- Rust native image preprocessing requires OpenCV 4.x, pkg-config, clang and
  libclang development headers. Python parity tooling is not production inference.
- Stage I export script can rewrite local graph/manifest/golden files; use its
  verify-existing mode for non-mutating identity checks.
- CUDA provider compilation is not hardware inference evidence.
- Stage II originals live in /tmp/retinacare-models (copies also in Dird/models/stage2_hsmoe_aunet/), while normal
  application resource paths point at models/. They are not automatically
  included in a package.
- GUI Cargo path dependencies break if Dird and Hackthon dezine are moved
  independently.
- GUI mock switches can return deterministic synthetic medical-looking data.
  Keep them away from real patient workflows and release configurations.
- GUI report constructor currently contains a stale checkpoint hash/runtime
  label. Check [[Decisions/risks-and-blockers]] before using provenance display.
- Obsidian launcher invocation in this sandbox failed with Chromium sandbox
  permission errors. UI/Graph View appearance remains unverified.
- GUI `backend.rs` matches `ModelEvent` and `ModelSubmitError` exhaustively. Never add variants to them; cascade events use the separate `AnalysisEvent` channel and `AnalysisSubmitError`.
- Each cascade stage re-verifies (SHA-256) and loads its ~0.8 GB model, so a full run is slow (232 s here, dominated by loading).
- Legacy npm build rewrites public/version.json. See repository docs before
  running legacy scripts.

See [[Testing/build-and-test]], [[Testing/cuda-and-packaging]], [[Integration/gui-source-findings]].

## Added 2026-10-28
- Cascade timings (release GUI, CPU): ~67-74 s; the first event can lag ~25 s while the worker finishes the Stage I preload.
- Closing the window used to wait for the whole run; `Drop for RetinaCareApp` now cancels the job (exit ~6 s).
- `RetinaCareApp` implements `Drop`, so struct-update syntax (`..Default::default()`) cannot be used on it in tests.
- `pkill -f` with a loose pattern can kill other sessions' processes (it hit a pre-existing GUI instance once). Use `^./target/release/retinacare`-style anchors or PIDs.
- Under Xvfb: `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11`, give the window X focus before typing, wheel over an image zooms it.
- The GUI decodes a metadata-scrubbed copy for display but the backend hashes/decodes the original file: an EXIF-rotated JPEG can fail the frame-size identity check (safe failure).
- `cargo fmt --all` formats the whole GUI crate (Antigravity's files included).
