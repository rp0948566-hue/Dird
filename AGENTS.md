# Repository rules

- Keep Ponytail full active for code work; follow `ponytail/skills/ponytail/SKILL.md`.
- Use the other `ponytail/skills/*/SKILL.md` files when their documented triggers match; do not run one-shot report skills on unrelated tasks.
- Superpowers is vendored at `skills/vendor/superpowers` (commit `2ce8aa15d05bd57213e6f746f953aafb7ded7a8b`, git-ignored so it never enters a commit). Read only the task-relevant skill file; never run its `hooks/` or `scripts/`.
- Keep `src/` and `src-tauri/` intact as the legacy DIRD+ reference until migration is tested.
- Native application lives under `native/`; Python worker lives under `ai_engine/`.
- Do not persist patient data in browser storage or expose generic SQL execution through the UI.
- Do not invent AI results, model availability, clinical outcomes, or performance figures.
- Keep weights, datasets, secrets, and patient imagery out of Git; follow dataset licenses and approval rules.
- Preserve AGPL-3.0 notices for reused DIRD+ code and document other asset licenses.
- Run relevant Rust/Python checks; report unavailable platform dependencies and untested hardware plainly.
- Read `docs/index.md` to load only task-relevant documentation.
