---
title: Build and test evidence
type: operations
domain: verification
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../docs/migration-progress.md"
  - "../../docs/features/P0-02-stage2-segmentation/07-test-results.md"
  - "../../GUI/Hackthon dezine/docs/features/GUI-P05-clinical-report-export/09-test-results.md"
owner: both workstream owners
tags: [build, tests, evidence]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/build-and-test.md (sha256 1cc8a2457a69); verify current state in [[active-phase]] -->
# Build and test evidence

## Backend (Dird/native)

Commands used in prior implementation session:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo build --release

Latest recorded Stage II run after HE: 59 passed, 0 failed, 7 ignored;
Python worker/model-integrity suite: 8 passed; release and diff checks passed.
See [detailed record](../../docs/features/P0-02-stage2-segmentation/07-test-results.md).
This memory-only audit did not rerun builds/tests.

Linux native OpenCV build requires OpenCV 4.x, pkg-config, clang and libclang
development files. Exact install/check commands are in the P0.1 feature doc.

## GUI (separate repository)

Its task reports record cumulative suites of 40, 53, 69, 81 and 94 passed for
Tasks 01–05. These are REPORTED results; current task did not rerun them.
Task 05 report names Cargo 1.84.1; current GUI toolchain was not compared here.
GUI build depends on current sibling path layout.

## Desktop and release

Dird docs report Ubuntu release/package and Windows GNU-target PE cross-build;
Windows execution remains unverified. GUI screenshot files exist in its docs,
but this audit did not inspect screenshots or launch the GUI. See
[[Testing/cuda-and-packaging]], [[Integration/gui-source-findings]], [[Decisions/risks-and-blockers]].

## Test run 2026-10-28 (after M5A cascade)

`cargo test --workspace`: 90 passed, 0 failed, 17 ignored. fmt, strict Clippy, release build and `git diff --check` pass. Heavy tests run individually with env vars and all passed: ai-bridge MA, HE, EX, CWS, VESSEL, Stage III fold 0 and eight-channel parity (7); app-service MA, HE, EX, CWS, VESSEL and the sequential five-mask run (6); full real cascade `local_full_cascade_technical_trace_matches_references` (1, env `RETINACARE_CASCADE_*`). The Stage I local test needs `RETINACARE_STAGE1_IMAGE` and was not run.

## Latest verified run 2026-10-28 (integration task)
Backend (`Dird/native`): fmt, clippy `-D warnings`, release build pass; `cargo test --workspace` **97 passed, 0 failed, 17 ignored**.
GUI (`Dird/GUI/Hackthon dezine/rust`): fmt, clippy `-D warnings`, release build pass; `cargo test` **166 passed, 0 failed, 2 ignored**; `cargo test -- --ignored` (real Stage I, real 7-model cascade through the GUI adapter) 2 passed. `git diff --check` clean in both repos.
Desktop: release binary driven under Xvfb with real clicks/typing; screenshots at 1280x820, 1024x768, 700x650; restart persistence; PDF opened by an independent reader. Evidence folder: `Dird/docs/features/GUI-BACKEND-INTEGRATION/` (42 checks + 30 edge cases).
Commits (local, not pushed): parent `45f4feb`, GUI `574f8f9`.
Gotchas: run the heavy tests alone (CPU/RAM); under Xvfb unset `WAYLAND_DISPLAY` and set `WINIT_UNIX_BACKEND=x11`, give the window X input focus before typing; never `pkill` by a loose pattern.

## 2026-10-28 - P2 final gates
Backend `cargo test --workspace` 120 passed, 0 failed, 18 ignored. GUI `cargo test` 170 passed, 0 failed, 2 ignored. fmt, clippy `-D warnings`, release builds and `git diff --check` pass in both. Heavy real-model GUI test passed earlier (63.7 s, 2826 MiB). Logs: `docs/features/P2-mask-persistence/evidence/`.

## 2026-10-28 - Phase 3 gates
Backend 134 passed / 0 failed / 18 ignored; GUI 172 / 0 / 2. Heavy: per-fold Rust parity (folds 0-4), five-fold cascade (54.9 s, 2.15 GiB), GUI-adapter cascade (55.4 s). fmt, check, clippy -D warnings, release builds pass. Details: `docs/features/P3-five-fold-ensemble/README.md`.
