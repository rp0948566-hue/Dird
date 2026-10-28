Before any code, run `/ponytail` first.
Never copy or restate text that exists in another file or in the code. Link to it.
Never write a doc, tool or system twice. Reuse what exists, edit it in place.
Never paste whole files into chat or notes. Quote the lines you need.
Short answers, no recaps. Keep this file under 80 lines.
Ponytail full applies to code work; use matching skills from `ponytail/skills/` when their triggers apply.
Use `memory/00-index.md` and relevant linked notes for continuity; search/read source only for task-required details, never reread the whole repo by default.
Stack: legacy React 18/TypeScript/Vite/Tauri; new native Rust/eframe and Python worker.
Native workspace: `native/`; AI package: `ai_engine/`; migration status: `docs/migration-progress.md`.
Run legacy checks with `npm test`, `npm run type-check`, and `npm run build`.
Run native checks with `cargo test --workspace --manifest-path native/Cargo.toml`.
Run worker checks with `python -m unittest discover -s ai_engine/tests`.
Protect patient data, model weights, datasets, and secrets from Git.
Load `docs/index.md` for the document map; read memory index before other vault notes.
