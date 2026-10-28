# P0.1 agent recovery audit

Audited: 2026-10-27, by a fresh agent with no prior chat context. Evidence is
file paths plus commands actually executed; no completion percentages are
guessed.

## What this repository is

RetinaCare is the migration of DIRD+ (Diabetic Retinopathy Detection Platform)
toward a native Rust desktop application for AI Manthan PS14. Two trees coexist:

- `src/` + `src-tauri/` — the legacy DIRD+ React 18 / TypeScript / Vite 6 /
  Tauri v2 application. **Frozen reference; must stay intact.**
- `native/` + `ai_engine/` — the new Rust workspace and its persistent Python
  JSON Lines worker. This is where RetinaCare development happens.

The problem it solves: diabetic retinopathy screening from retinal fundus
images with all inference, storage and reporting kept on the local machine
(no cloud, no per-screening fee), under SQLCipher-encrypted patient storage.

## How the Rust crates communicate

```
retinacare-ui (eframe, 7 screens)
      │  state / commands
      ▼
retinacare-app ──┬──► retinacare-persistence  (SQLCipher, .dird import)
  WorkerSupervisor│──► retinacare-domain       (typed clinical entities)
                  └──► retinacare-reports      (report structures)
                        │
                        │  WorkerClient: bounded, versioned JSONL over
                        │  stdin/stdout, one request per line,
                        ▼  one response per line
                 retinacare-ai-bridge
                        │
                        ▼
                 ai_engine/retinacare_ai/worker.py  (persistent process)
                        │  health / device probe / image inspection /
                        │  model discovery (no deserialization)
                        ▼
                 models/  (only README + manifest.example.json today)
```

Protocol facts (verified in `native/crates/retinacare-ai-bridge/src/lib.rs`):
`PROTOCOL_VERSION = 1`, requests ≤ 4096 bytes, responses ≤ 1 MiB, timeout
kills and reaps the child, response envelope must echo `request_id` and
protocol version. Images travel as local file paths, never bytes.

## Verified status table

| Capability | Classification | Evidence |
|---|---|---|
| Rust workspace, 6 crates, edition 2024 | VERIFIED_IMPLEMENTED | `native/Cargo.toml` |
| Baseline gates: `cargo test --workspace` (12 tests) and `cargo fmt --all --check` | VERIFIED_IMPLEMENTED | executed 2026-10-27, both pass |
| Bounded JSONL worker client, cancellation, reaping, size limits | VERIFIED_IMPLEMENTED | `retinacare-ai-bridge/src/lib.rs` + 4 tests |
| Persistent Python worker, health/device/image commands | VERIFIED_IMPLEMENTED | `ai_engine/src/retinacare_ai/worker.py`, 5 tests in `ai_engine/tests/` |
| SQLCipher patient/examination flow, atomic `.dird` import | VERIFIED_IMPLEMENTED | `retinacare-persistence/src/lib.rs`, `retinacare-app/src/database.rs` |
| Model *discovery* by the worker (lists `.pt`/`.pth`, never loads them) | VERIFIED_IMPLEMENTED | `worker.py::discover_models` |
| P0.1 documentation root `docs/features/P0-01-hierarchiretina-model-loading/` | NOT_IMPLEMENTED (created by this session) | directory did not exist before |
| Any reference to "HierarchiRetina" or "P0.1" anywhere in the tree | NOT_IMPLEMENTED | `grep -ri "hierarchiretina\|P0\.1"` → 0 matches |
| Rust-side model manifest validation (the documented precondition) | VERIFIED_IMPLEMENTED | `native/crates/retinacare-ai-bridge/src/model_manifest.rs` + 8 tests (updated 2026-10-27) |
| Manifest gate → real ONNX session (load, metadata check, inference) | VERIFIED_IMPLEMENTED (fixture only) | `native/crates/retinacare-ai-bridge/src/onnx_session.rs` + 5 tests; a 186-byte synthetic graph loads and runs (updated 2026-10-27) |
| Native ONNX Runtime in Rust (`ort`) | VERIFIED_IMPLEMENTED, CPU only | `ort = 2.0.0-rc.15` pinned in `native/Cargo.toml`; ONNX Runtime 1.31 downloaded and statically linked (rustls, no system OpenSSL). No CUDA/EP registered or tested. ROADMAP.md line 81 stays unchecked: two-pass tiled inference is not implemented |
| HierarchiRetina checkpoint on disk | BLOCKED | `models/` holds only `README.md` + `manifest.example.json` |
| PyTorch → ONNX conversion of HierarchiRetina | BLOCKED | requires dev-only Python (`onnx`, `torch` not installed: `ModuleNotFoundError: No module named 'onnx'`) and the ~10 GB checkpoint set, which needs authorization |
| Numerical parity vs. PyTorch reference | BLOCKED | no converted artifact and no reference output exist |
| CUDA / RTX 5070 validation, clinical output, accuracy figures | NOT_TESTED | no hardware or model evidence in this repository |

## Which pieces remain Python-dependent

`ai_engine/` owns device probing, image inspection and preprocessing
validation. It deliberately reports `no_trained_weights` /
`inference_adapter_unavailable` instead of producing any clinical result.
Per repository rules Python stays development-only except for this worker
protocol role: **no Python production inference runtime may be added.**

## Which models are available locally

None. `.pt`, `.pth`, `.onnx`, `.gguf` are git-ignored by design
(`models/README.md`); nothing has been placed there. HierarchiRetina v1.0
itself publishes 12 PyTorch checkpoints (~10 GB, SHA-256 listed on the
release page) under claimed MIT (code) / CC BY-NC 4.0 (weights) terms, with
**no ONNX export**.

Source: https://github.com/asifuddin01/HierarchiRetina ,
https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0 ,
https://arxiv.org/abs/2610.17933

## What is missing for Rust-only AI inference

Ordered smallest-first:

1. ~~**Manifest verification gate**~~ — done 2026-10-27: `model_manifest::verify_manifest`
   enforces schema v1, placeholder rejection, required output/validation
   evidence, 64-hex sha256, NCHW batch-1 shape and a streamed file hash.
2. ~~**A native inference runtime**~~ — done 2026-10-27 (CPU): `ort`
   2.0.0-rc.15 / ONNX Runtime 1.31, wired through `onnx_session::VerifiedSession`,
   which refuses any checkpoint whose graph metadata disagrees with the
   manifest. Stage-specific adapters for the four HierarchiRetina graphs are
   **not** written yet.
3. **Authentic converted artifacts** — needs a 10 GB authorized download and
   development-only `torch`/`onnx` conversion, then a parity harness.
   `ort`'s model editor could write ONNX from PyTorch weights only after that
   download; nothing has been fetched.
4. **CUDA validation** on the RTX 5070 (never run in this repository).

## Milestone 2 update (2026-10-27, second session)

`cargo test --workspace` = 25 tests pass. A **synthetic, non-medical** fixture
(`native/crates/retinacare-ai-bridge/fixtures/tiny_mul.onnx`, 186 bytes,
sha256 `e3e8d2ee…c867b`, IR v8/opset 13) is verified by hash, loaded into a
real ONNX Runtime session, its tensor metadata (`image [1,3,4,4]` float32 →
`result [1,3,4,4]`) is read from the graph and compared with the manifest, and
its output is checked element-wise (`result = image * 2`).

**This proves the loading machinery only.** HierarchiRetina is not integrated:
no authentic checkpoint was downloaded, converted, hashed against upstream,
parity-checked or run on retinal data.

## Why the architecture works this way

The renderer never gets raw SQL or browser persistence; the worker validates
request shape and local paths; clinical output is only allowed behind a
verified task-compatible checkpoint. The manifest gate is the load-time half
of that trust boundary.

## What should be reused

`WorkerClient` (bounded JSONL, cancellation), `ResourcePaths` (models
directory discovery, dev override `RETINACARE_DEV_ROOT`), the
`WorkerSupervisor` health loop, `models/manifest.example.json` as the schema
seed, and the existing `WorkerError`/`thiserror` error style.

## What must not be changed

`src/`, `src-tauri/`, `retinacare-ui` (GUI is out of scope unless explicitly
requested), SQLCipher protections, AGPL-3.0 notices, the
`no_trained_weights` honest-error behavior, and the no-download /
no-fabricated-results policy.

## Skills installed

| Skill | Location | State |
|---|---|---|
| ponytail | `ponytail/` (already present, plain directory, not a nested repo) | reused, not duplicated; `ponytail/skills/ponytail/SKILL.md` followed for all code work |
| superpowers | `skills/vendor/superpowers` | cloned this session at commit `2ce8aa15d05bd57213e6f746f953aafb7ded7a8b`; its `hooks/` and `scripts/` were **not** executed or installed |

No JavaScript framework or other dependency was added to the production
runtime for either skill.