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

## Added 2026-10-29 (GPU)
- New models get no GPU until added to `GPU_VERIFIED_TASKS` after a CPU/GPU parity test. Strict `RETINACARE_PROVIDER=cuda` refuses unverified tasks by design.
- CUDA libs must be on `LD_LIBRARY_PATH`; missing libs in auto = CPU plus a `GPU unavailable for ...` log line, not an error.
- `pkill -f retinacare` killed the shell once; use `kill $(pgrep -x retinacare)`.
- `git push` only to `test` / `test-gpu`; project CLAUDE.md rule says `dezine`, user authorized these branches explicitly.

## Added 2026-10-29 (demo run)
- Screenshot automation: GUI must run under Xvfb (`env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11`), focus the window with XSetInputFocus, XTest clicks need a motion event before the press. Real-desktop XTest clicks can miss when another window covers the app.
- File dialogs go through xdg-desktop-portal on the real desktop, not Xvfb: use `RETINACARE_AUTOPICK_IMAGE=<path>` for attach and `RETINACARE_PDF_DEST=<file>` for PDF export. `RETINACARE_THEME=light` starts light.
- `pkill -f "^./target/release/retinacare"` also kills the user's own desktop instance: relaunch it afterwards.
- Chat needs a working Ollama: without `llama-server` Ollama answers HTTP 500 with the reason in the body; the GUI now shows it.
