# Repository audit and native migration baseline

Audit date: 2026-10-26. Scope: tracked source, app/build configuration, security and validation docs, test inventory, and available local verification. No migration implementation performed.

## Current system

DIRD+ is a React 18 + TypeScript + Vite 6 application packaged as a Tauri v2 desktop app. The frontend includes patient/session workflows, retinal image review and annotation, ONNX inference, configurable clinical guideline classification, PDF reports, and `.dird` import/export. `src-tauri/` is an active Rust backend, not a native egui application: it provides SQLCipher storage, Argon2id/AES-GCM commands, model file management, and optional llama.cpp text generation. There is no `native/` workspace, Python `ai_engine/`, or persistent JSONL worker.

## Feature parity baseline

| Capability | Current implementation and evidence | Native migration status |
|---|---|---|
| Patient and exam workflows | `src/components/patients/`, `src/components/upload/`; patient, session, image and report schemas in `src/lib/db/schema.ts` | Existing Tauri UI only; no egui port |
| Fundus viewer and annotation | `src/components/canvas/ImageAnalyzer.tsx`, `src/components/canvas/layers/`, `src/hooks/` | Existing webview implementation only |
| Detection inference | `src/lib/ai/onnx-manager.ts`, `src/lib/ai/inference-service.ts`; external ONNX files via model registry/downloader | Native Python pipeline not implemented; actual detector weights live outside this repository |
| Segmentation inference | UI/schema scaffolding exists; README and format reference mark model as planned/not released | Blocked on compatible model and validated pipeline |
| Clinical rules | `src/lib/clinical-guidelines/`, `public/clinical-guidelines/`; ICDR/MINSAL JSON-driven classification | Rust clinical crate not implemented |
| Encrypted desktop storage | `src-tauri/src/db.rs`, `src-tauri/src/crypto.rs`, `src-tauri/migrations/0001_init.sql`; frontend SQLCipher facade in `src/lib/db-sql/` | Existing Tauri path; no independent native storage crate |
| Browser storage | Dexie schema retained in `src/lib/db/schema.ts` for legacy migration | Browser mode stores data in IndexedDB, not SQLCipher |
| Data import/export | `src/lib/export/dird-exporter.ts`, `src/lib/export/dird-importer.ts`, `src/lib/export/dird-container.ts` | Native importer/exporter not implemented |
| Reports | `src/lib/pdf/`, `src/components/reports/` | Native PDF crate not implemented |
| Local language model | `src-tauri/src/llm.rs`, `src/lib/ai/llm-client.ts`; optional llama.cpp via Rust | Python worker not implemented |
| Dataset evaluation | `validation/experiment-1-idrid/` through `experiment-5-fairness/`; reports and scripts are present | Reproducible native training/eval system not implemented |
| Dataset/model acquisition docs | `validation/README.md`, experiment reports, `REFERENCES.md`; no root `docs/datasets.md` or `models/README.md` | Incomplete for proposed migration contract |
| App and release CI | `.github/workflows/release.yml`; `scripts/` contains version and model-card utilities | No separate Rust/Python CI or Windows native packaging scripts |

## Findings to address before expanding the desktop backend

1. **High — password rotation can strand the encrypted database.** `src-tauri/src/db.rs::db_change_password_with_handle` runs SQLCipher `PRAGMA rekey` and then writes the new salt directly to `dird.salt`. If that write fails after rekey succeeds, the DB uses the new key while the stored salt still derives the old key. The code comment says the salt write is atomic, but `std::fs::write` is not an atomic replace. Make rotation recoverable (staged salt/backup or a transaction protocol) before relying on this path.
2. **High — archive import can partially overwrite patient data.** `src/lib/export/dird-importer.ts` deletes an existing session/patient's related records before importing replacements, and import spans multiple awaited writes without wrapping them in a database transaction. A malformed archive or storage failure can leave data deleted or a partial import. Validate archive contents first and perform replacement in a single transaction with rollback.
3. **Medium — broad renderer-to-SQL command surface.** `src-tauri/src/lib.rs` registers `db_execute` and `db_query`; `src-tauri/src/db.rs` accepts arbitrary SQL from the renderer. Parameters protect values from SQL injection, but the command surface still permits any renderer code to issue arbitrary reads, DDL, and writes. Replace it with domain repository commands or enforce a narrow SQL allowlist before widening the UI/content trust boundary.
4. **Medium — database confidentiality depends on runtime.** The Tauri database uses SQLCipher; the retained Dexie implementation is IndexedDB. Product/security docs should state that browser/web mode does not get SQLCipher-at-rest protection and should not make an unconditional encrypted-storage promise.
5. **Clinical limitation — image quality is not gated.** `validation/README.md` explicitly says ungradable images are scored as if gradable. It also reports reduced mild-DR sensitivity on external datasets. A native port must preserve these limits, add/validate an image-quality gate, and keep clinician review explicit; a missing model must remain an unavailable result, never a fabricated grade.
6. **Migration decision — there is no detached native base today.** The existing Rust code is coupled to Tauri commands and the frontend calls them through `invoke`; moving to egui/Python is a new application architecture, not a straightforward UI swap. Keep the existing `src/` and `src-tauri/` as a working reference until parity and import/export compatibility are tested.

## Quality and local verification

- `npm test -- --reporter=dot`: passed, 7 test files and 52 tests.
- `npm run type-check`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: could not reach project compilation because host dependency `gdk-3.0` is unavailable (`pkg-config` could not find `gdk-3.0.pc`). This is an environment prerequisite, not evidence of a Rust test failure.
- `npm run build`: passed. Vite reports the main JS chunk at 1.32 MB and the ONNX WebAssembly asset at 23.8 MB; review startup/download cost and code splitting before distribution.
- No Tauri packaging, Windows build, GPU inference, or dataset experiment was run for this audit.

## Highest-priority next work

Fix and regression-test password rotation and archive-import rollback in the current app before beginning native migration. Then document dataset/model acquisition constraints in `docs/datasets.md` and `models/README.md`, including original URLs, approval/license terms, checksums, local layout, and reproducible evaluation commands. Only after that should a minimal Rust-native vertical slice and versioned worker protocol be implemented.
