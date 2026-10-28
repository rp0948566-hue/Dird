---
title: Claude read-first
type: onboarding
status: VERIFIED
verified_on: 2026-10-28
---
# Claude: start here

1. Read [[00-index]], [[active-phase]], [[Handoffs/next-actions]]. Stop; open more only for the task.
2. `git status --short --branch` in `/home/claude/Desktop/Dird` AND `Dird/GUI/Hackthon dezine`. They are separate repos ([[Integration/repo-topology]]).
3. Preserve uncommitted work. Never reset/clean. Never `git add`/`push` in the GUI repo without user OK (its CLAUDE.md). No commit/push in either repo without approval.
4. Backend crates: `Dird/native/crates/` (Codex/Claude). GUI: `Dird/GUI/Hackthon dezine/rust/src` (Antigravity) — coordinate before editing.
5. Do not extend `ModelEvent` / `ModelSubmitError`; the GUI matches them exhaustively. Use `AnalysisEvent` ([[Integration/readiness-report]]).
6. Check [[Decisions/gotchas]] before building. Research-only model limits: [[Models/licensing-and-data]].

## Checks
- Backend: `cargo test --workspace --manifest-path native/Cargo.toml` (+ fmt, clippy -D warnings).
- GUI: in `GUI/Hackthon dezine/rust`: `cargo fmt --all -- --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`; heavy real-model tests: `cargo test -- --ignored`.
- Launch the one app: `cd "GUI/Hackthon dezine/rust" && ./target/release/retinacare` (DB password prompt on Patients; `RETINACARE_DB_PATH` overrides the DB file; `RETINACARE_DEV_FIXTURES=1` = labelled fixtures only).
- Headless screenshots: Xvfb + `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11`; never `pkill` by a loose pattern (it can hit other sessions' processes).
- Python worker: `python -m unittest discover -s ai_engine/tests`.

## After the task
Update [[active-phase]], [[Handoffs/next-actions]], [[Handoffs/changelog-and-handoffs]]. Real results only; never claim clinical validation, CUDA runs or GUI results without evidence.
