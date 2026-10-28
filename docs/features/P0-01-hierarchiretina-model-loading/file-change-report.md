# P0.1 file change report

Baseline recorded before editing (`git status --short --branch` on
`main...origin/main`): 10 tracked files already modified
(`.gitignore`, `README.md`, `src-tauri/src/db.rs`, `src/components/auth/AppGate.tsx`,
`src/i18n/locales/en.json`, `src/i18n/locales/es.json`, `src/lib/crypto/index.ts`,
`src/lib/db-sql/client.ts`, `src/lib/export/dird-importer.ts`,
`src/stores/auth-store.ts`) plus untracked `AGENTS.md`, `CLAUDE.md`,
`ai_engine/`, `docs/*`, `memory/`, `models/`, `native/`, `ponytail/`,
`scripts/*`, `src/lib/tauri-runtime*`. Those pre-existing changes belong to
whoever made them; **nothing was staged, committed, stashed, reverted or
pushed**, and every pre-existing hunk is intact.

| File | Created or modified | Exact behavior changed | Why it was necessary | Tests executed | Verification status |
|---|---|---|---|---|---|
| `native/crates/retinacare-ai-bridge/src/model_manifest.rs` | Created | Adds `verify_manifest(path) -> Result<VerifiedModel, ManifestError>`: parses schema v1, rejects placeholder identity fields, requires `outputs` and `validation` evidence, requires 64-hex sha256, requires an NCHW batch-1 input shape, resolves the checkpoint relative to the manifest, streams it through SHA-256 and compares, else typed error | `docs/model-integration.md` mandates manifest + hash + tensor-shape validation *before* inference; nothing in Rust implemented it, so any file in `models/` could reach a future load path unauthenticated | `cargo test -p retinacare-ai-bridge` (8 new tests: happy path, tampered checkpoint, missing checkpoint, the placeholder example manifest, missing required field, unsupported schema_version, malformed sha256, wrong-rank shape) | VERIFIED — tests pass |
| `native/crates/retinacare-ai-bridge/src/lib.rs` | Modified (2 added lines) | Declares `pub mod model_manifest;` and `pub mod onnx_session;` | Exposes both halves of the P0.1 path | `cargo test --workspace` | VERIFIED |
| `native/Cargo.toml` | Modified (`[workspace.dependencies]`) | Adds `hex = "0.4"`, `sha2 = "0.10"`, and `ort = "=2.0.0-rc.15"` with `default-features = false` and features `std, ndarray, tracing, download-binaries, tls-rustls, copy-dylibs, api-31` | SHA-256 verification cannot be hand-rolled; `ort` supplies the native ONNX Runtime. Default features were dropped because `tls-native` needs system OpenSSL headers this host does not have (`openssl-sys` build failure) — `tls-rustls` downloads the same binaries without them. Approval for the dependency and its binary download was given on 2026-10-27 | `cargo build -p retinacare-ai-bridge`, `cargo test --workspace`, clippy, release build | VERIFIED — ONNX Runtime 1.31 fetched via rustls and statically linked |
| `native/crates/retinacare-ai-bridge/Cargo.toml` | Modified (3 lines) | The crate consumes `hex`, `sha2` and `ort` workspace dependencies | Workspace convention (`serde.workspace = true` style) | same as above | VERIFIED |
| `native/Cargo.lock` | Modified | Locks the `ort`/`ort-sys` tree (plus rustls/ureq download tooling) | Reproducible builds | `cargo test`, `cargo build --release` | VERIFIED |
| `native/crates/retinacare-ai-bridge/src/onnx_session.rs` | Created | `VerifiedSession::open(manifest)` → `verify_manifest` → `ort` session build → graph metadata check (single float32 input matching the declared NCHW shape, single output) → `run(&[f32])` returning the extracted output. Typed errors: `Manifest`, `Runtime`, `Build`, `InputMismatch`, `Run`. One `Environment` per process via `OnceLock` | Requirement: connect the manifest gate to a real ONNX session-loading path with meaningful errors | 5 tests: fixture load+run with exact numerical check, invalid manifest, missing artifact, unsupported model file, wrong-sized input | VERIFIED |
| `native/crates/retinacare-ai-bridge/src/lib.rs` | Modified (2 added lines) | Declares `pub mod model_manifest;` and `pub mod onnx_session;` | Exposes both halves of the P0.1 path | `cargo test --workspace` | VERIFIED |
| `native/crates/retinacare-ai-bridge/fixtures/tiny_mul.onnx` | Created (186 bytes, generated) | Synthetic graph `result = image * 2.0`, float32 `[1,3,4,4]`, IR v8, opset 13, sha256 `e3e8d2ee1c6c61369250ddd6a7675dd790ba964dcff7d5444f617d3f459c867b` | Needed a real, loadable ONNX file to prove loading; deliberately non-medical and unlabeled as clinical | `onnx.checker.check_model` (Python, dev-only) + the 5 Rust session tests | VERIFIED |
| `native/crates/retinacare-ai-bridge/fixtures/make_tiny_mul.py` | Created | Development-time export of the fixture | Python is permitted here for export only; keeps the fixture reproducible | Run once under `ai_engine/.venv` | VERIFIED |
| `native/crates/retinacare-ai-bridge/fixtures/README.md` | Created | Labels the fixture synthetic/non-medical and states what it does **not** prove | Prevents a fixture pass from being read as clinical validation | n/a | VERIFIED |
| `ai_engine/.venv/` | Created (git-ignored) | Dev-only virtualenv holding `onnx 1.23.2` | Model-export tooling must stay out of the production runtime; `ai_engine/.venv/` is already in `.gitignore` | `python -c "import onnx"` → `onnx 1.23.2` | VERIFIED |
| `docs/features/P0-01-hierarchiretina-model-loading/agent-recovery-audit.md` | Created | Repository identity, crate/IPC diagram, VERIFIED/IMPLEMENTED/PARTIAL/BLOCKED table with evidence paths, Python-dependency list, model inventory, missing pieces, reuse/do-not-change lists, skills state | Required initial report of the bootstrap brief; the directory did not exist before this session | Facts cross-checked against executed commands (`cargo test`, `cargo fmt --check`, grep for `HierarchiRetina`/`P0.1` → 0 matches) | VERIFIED (no speculative percentages) |
| `docs/features/P0-01-hierarchiretina-model-loading/file-change-report.md` | Created | This table | Required change-tracking artifact | n/a | VERIFIED |
| `memory/active-phase.md` | Created | Phase, goal, implemented, blockers, next action, source files, tests run, working commands, next-phase prerequisites | Required handoff note with the mandated fields | n/a | VERIFIED |
| `memory/architecture.md` | Created | Crate + IPC map, trust boundary, IPC constants, path discovery | Required memory file; links to `docs/architecture` instead of duplicating it | n/a | VERIFIED |
| `memory/retinacare-current-state.md` | Created | Verified working vs not-started vs never-tested snapshot | Required memory file; keeps verified facts separate from assumptions | n/a | VERIFIED |
| `memory/decisions.md` | Created | Six settled decisions: slice choice, no `ort` yet, no downloads, manifest schema seed, skills vendoring, no caller yet | Required memory file | n/a | VERIFIED |
| `memory/00-index.md` | Modified (4 lines appended) | Adds links to the four new notes | Index is the memory entry point | n/a | VERIFIED |
| `AGENTS.md` | Modified (1 bullet added) | Records the Superpowers vendor path, pinned commit and "never run its `hooks/`/`scripts/`" rule | Installation rule of the bootstrap brief; existing rules preserved unchanged | n/a | VERIFIED |
| `.gitignore` | Modified (3 lines appended) | Ignores `skills/vendor/` | Documented vendor strategy so a nested Git repository can never enter an ordinary commit. **The user's own uncommitted `.gitignore` edits above these lines are untouched** | `git diff --check` | VERIFIED |
| `docs/index.md` | Modified (1 line appended) | Points at the P0.1 feature folder | `docs/index.md` is the documented map for loading task-relevant docs | n/a | VERIFIED |
| `skills/vendor/superpowers/` | Created (git clone, now ignored) | Full clone at commit `2ce8aa15d05bd57213e6f746f953aafb7ded7a8b` | Bootstrap brief requires it when the vendor path is absent | Clone verified; `hooks/` and `scripts/` deliberately not executed, no dependency added to the production runtime | VERIFIED |

## Checks actually executed (2026-10-27, from `native/`)

| Check | Result |
|---|---|
| `cargo test --workspace` | PASS — 12 test targets ok, **25 tests** (12 baseline + 8 manifest gate + 5 session path) |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo build --release` | PASS (55.20 s; links the statically downloaded ONNX Runtime) |
| `git diff --check` | PASS (also a manual trailing-blank scan of every new file, since `native/`, `memory/` and `docs/features/` are untracked and invisible to `git diff`) |
| `python3 -m pytest ai_engine/tests -q` | **BLOCKED** — `No module named pytest`. The 5 Python worker tests were not re-run this session. |
| `cargo test` for legacy `src-tauri/` | **BLOCKED** (pre-existing) — missing `gdk-3.0 >= 3.22` development files |
| Windows execution, CUDA/RTX 5070, GUI smoke, model/parity validation | **NOT RUN** — no hardware, no artifacts, GUI out of scope |

## Not changed

`src/`, `src-tauri/`, `retinacare-ui` (all GUI), SQLCipher behavior, the
worker protocol, `models/README.md` and `models/manifest.example.json`, and
every pre-existing uncommitted hunk. `git status --short` after this session
lists exactly the same 10 pre-modified tracked files as the baseline above.

## Explicitly not claimed

No HierarchiRetina checkpoint was downloaded, converted, hashed against the
upstream release, parity-checked or executed. No CUDA or other execution
provider was enabled or tested — inference verified here is **CPU only**. No
accuracy, grading, segmentation or benchmark figure was produced.

## Stage I continuation — 2026-10-28

The report above is an immutable snapshot of initial model-loading work, not
the current file inventory. Continued Stage I preprocessing changes and their
purpose are recorded in [the current file-change report](09-file-change-report.md).
Current results supersede the earlier test totals: OpenCV Rust preprocessing,
CPU image-to-result inference, exact single-image numerical parity, 53 Rust
workspace tests, 5 Python worker tests, and strict Clippy passed. Formal
licensed-image acceptance and RTX 5070/CUDA execution remain open. No GUI
source was changed.
