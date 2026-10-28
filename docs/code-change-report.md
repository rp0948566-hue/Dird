# RetinaCare repository change report

Date: 2026-10-27

This report maps every source/configuration/documentation file added or changed for the RetinaCare Phase 1 work to its purpose. DIRD+ source remains in place; this work adds the native implementation alongside it and applies two safety fixes to the retained app.

## Existing application changes

| File | Change and reason |
|---|---|
| `src/lib/tauri-runtime.ts` | Added a Tauri runtime check and guarded IPC helper. Prevents browser mode from calling an unavailable desktop bridge. |
| `src/stores/auth-store.ts` | Added an `unavailable` state before database boot checks when running outside Tauri. Stops browser unlock from reaching database commands. |
| `src/components/auth/AppGate.tsx` | Displays a clear desktop-runtime-required message for that state. |
| `src/i18n/locales/en.json` | Adds English copy for the desktop-only message. |
| `src/i18n/locales/es.json` | Adds the Spanish equivalent. |
| `src/lib/db-sql/client.ts` | Routes database IPC through the runtime guard so missing Tauri cannot cause an undefined `invoke` error. Existing legacy SQL commands remain in the old app; new native UI has no generic SQL command. |
| `src/lib/crypto/index.ts` | Routes crypto IPC through the same runtime guard. |
| `src-tauri/src/db.rs` | Stages the new salt before SQLCipher rekey and retains recovery candidates if salt metadata commit fails. Adds recovery test for an interrupted post-rekey salt update. |
| `src/lib/export/dird-importer.ts` | Wraps patient/session/full archive writes in a Dexie read/write transaction. Converts returned import failures into transaction failures so Dexie rolls back partial changes. |
| `src/lib/export/dird-importer.test.ts` | Checks successful transaction commit and rollback when import reports failure. |
| `src/lib/tauri-runtime.test.ts` | Checks desktop detection, safe refusal in browser mode, and native boot behavior. |

## New native Rust workspace

| File | Purpose |
|---|---|
| `native/Cargo.toml`, `native/Cargo.lock` | Define and lock the independent Rust workspace and crate dependencies. |
| `native/README.md` | Gives native build and run entry points. |
| `native/crates/retinacare-domain/Cargo.toml`, `src/lib.rs` | Typed patient, examination, clinical classification, and annotation domain models, with serialization tests. |
| `native/crates/retinacare-persistence/Cargo.toml`, `src/lib.rs` | SQLCipher database and safe patient/examination repositories; Argon2id key derivation, zeroized key material, atomic rotation recovery, and encrypted-storage tests. |
| `native/crates/retinacare-reports/Cargo.toml`, `src/lib.rs` | Typed report data structures and clinician-review state. |
| `native/crates/retinacare-ai-bridge/Cargo.toml`, `src/lib.rs` | Persistent bounded JSONL IPC, protocol validation, timeout cancellation, worker shutdown/reaping, and stderr logging, with regression tests. |
| `native/crates/retinacare-app/Cargo.toml`, `src/lib.rs`, `src/database.rs`, `src/resources.rs` | Coordinates nonblocking worker/database controllers and discovers packaged resources plus writable per-user data paths. |
| `native/crates/retinacare-ui/Cargo.toml`, `build.rs`, `src/main.rs` | Native seven-screen `eframe` application, background image loading, zoom/pan, patient/examination UI, smoke modes, and embedded Windows/window icon. |

## New Python worker

| File | Purpose |
|---|---|
| `ai_engine/pyproject.toml` | Defines the local Python package and test configuration. |
| `ai_engine/README.md` | Documents worker startup and protocol use. |
| `ai_engine/src/retinacare_ai/__init__.py` | Marks the worker package. |
| `ai_engine/src/retinacare_ai/worker.py` | Persistent stdin/stdout JSONL protocol, health/model/analysis commands, bounded requests, and explicit missing-model errors. No HTTP server. |
| `ai_engine/src/retinacare_ai/devices.py` | Reports available CPU/PyTorch/CUDA device information without inventing GPU telemetry. |
| `ai_engine/src/retinacare_ai/preprocessing.py` | Validates local image inputs and bounds file size, dimensions, and Pillow decompression behavior. |
| `ai_engine/tests/test_worker_protocol.py` | Verifies health/shutdown, invalid envelopes, oversized-message recovery, unknown commands, and truthful missing-model behavior. |

## Repository setup and workflow

| File | Change and reason |
|---|---|
| `.gitignore` | Ignores Rust build output, Python virtualenv/cache, local databases, datasets, and model-weight files to avoid committing generated or sensitive data. |
| `README.md` | Adds the RetinaCare migration entry and states current limitations while preserving DIRD+ as the existing implementation. |
| `AGENTS.md` | Adds repository-specific agent instructions for the migration. |
| `CLAUDE.md` | Adds project context and links to on-demand documentation. |
| `Ghost.md` | Adds project AI rules. |
| `.github/workflows/native.yml` | Adds CI checks for the new Rust and Python components. |
| `.github/workflows/native-release.yml` | Builds and checks Ubuntu and Windows portable artifacts, including GUI smoke jobs. |
| `scripts/run-native-dev.ps1` | Provides a Windows development launcher for the native UI and worker. |
| `scripts/test-native.ps1` | Provides Windows commands to run native and worker tests. |
| `scripts/package-ubuntu.sh` | Builds a self-contained Ubuntu package with pinned Python and Pillow artifacts plus checksums. |
| `scripts/package-windows.ps1` | Builds and smoke-tests the native Windows portable distribution on Windows. |
| `scripts/package-windows-cross.sh` | Cross-builds the real Windows PE package on Ubuntu without claiming Windows execution. |
| `scripts/test-windows.ps1` | Performs post-reboot packaged worker, SQLCipher, GUI navigation, exit, and orphan-process smoke checks. |
| `icon/RetinaCare.ico` | Multi-size executable/window icon generated from the supplied source image. |

## Documentation and Obsidian memory

| File | Purpose |
|---|---|
| `docs/index.md` | Documentation map and read guidance. |
| `docs/code-change-report.md` | This file-by-file report, added to explain the scope and reason for each change. |
| `docs/architecture.md` | Explains Rust crate boundaries and worker IPC. |
| `docs/migration-audit.md` | Captures the pre-implementation repository audit and baseline; findings here describe the state before the current changes. |
| `docs/migration-progress.md` | Records verified implementation, test results, blockers, and next milestone. |
| `docs/model-integration.md` | Defines model discovery, checkpoint requirements, and missing-weight behavior. |
| `docs/datasets.md` | Records dataset acquisition and licensing requirements. |
| `docs/windows-build.md` | Lists Windows build requirements and observed Linux GTK/display blockers. |
| `docs/security.md` | Documents storage, IPC, input validation, and privacy boundaries. |
| `docs/browser-unlock-fix-plan.md` | Records screenshot bug root cause and the fix path. |
| `docs/code-quality-audit.md` | Records confirmed production findings, fixes, RustSec result, and remaining gaps. |
| `docs/exe-verification.md` | Records executable formats, package contents, display diagnosis, and prohibited claims. |
| `memory/00-index.md` | Obsidian vault index for project context and notes. |
| `memory/eye-ok.md` | Preserves the previously requested short memory note. |
| `memory/gotchas.md` | Captures implementation gotchas for future sessions. |
| `memory/security.md` | Keeps security-specific project memory. |
| `memory/retinacare-phase-1.md` | Preserves the user’s phase objective and current progress in the vault. |
| `models/README.md` | Explains where verified model files belong and how they are handled. |
| `models/manifest.example.json` | Provides a metadata-only example manifest; contains no trained weights. |

## Verification state

Passed: 59 legacy frontend tests from Phase 1, TypeScript check, frontend production build, 5 Python tests, 12 native Rust tests, Ubuntu and Windows release builds, packaged Ubuntu smoke, native formatting, native Clippy with warnings denied, package checksums, Python `pip check`, and RustSec with no known vulnerabilities. `git diff --check` passes.

Blocked or not validated: existing Tauri Rust tests stop before compilation because this host lacks `gdk-3.0 >= 3.22`; Xvfb cannot bind because `/tmp/.X11-unix` has the wrong owner, and the bounded Wayland GUI smoke produced no report. Windows execution, CUDA/RTX 5070 behavior, and inference with trained weights were not validated. Details live in `migration-progress.md`.

## Repository publication

These changes are present in the current local Git working tree. No commit, remote repository creation, or push was performed.
