# Phase 5 plan and risk map (2026-10-29)

Research prototype. Not a diagnostic device. Spec: user PDF + master prompts (not in repo).

## M0 baseline (verified 2026-10-29)
- Parent `main` HEAD `8581790` (11 ahead of `Debaq/Dird`, push 403). GUI `dezine` HEAD `0f14417` (3 ahead of origin). Both clean except 2 untracked Antigravity PDFs in GUI (left alone).
- Tests rerun: backend 157 passed / 0 failed / 21 ignored; GUI 188 passed / 0 failed / 2 ignored.
- Supabase `ebflochdtajcflmlhgqa`: one migration `chat_persistence`; 5 tables, RLS on, 0 rows. No Rust client yet.
- DB path already stable: `$RETINACARE_DB_PATH` else `$XDG_DATA_HOME|~/.local/share/RetinaCare/database/retinacare.sqlite` (`engine/store.rs`).
- Home composer: `UnavailableChatService`, label "HierarchiRetina (Awaiting Backend)", chats in memory only.
- Timeline: reads stored records; badge shows only when the DB is locked.
- Gnome keyring (Secret Service) running; Ollama installed, no model; Stage I `.pth` checkpoint present.

## User decisions (2026-10-29)
- Mode C: local Ollama (`llama3.2:3b`, ~2 GB download approved). First pull was stopped at 5% by the session; retry needs user go-ahead.
- Supabase: build Stage B/C; live two-user test BLOCKED until the user signs in with two test accounts.
- New crates approved: `ureq` (HTTP), `keyring` (Secret Service).

## Order of work (Mode R first)
1. Backend: per-fold Stage III progress event (`FoldCompleted`). Chat store in SQLCipher (schema v5: conversations, messages, outbox).
2. Home: mode selector (Research Analysis default / Chat Assistant). Mode R runs the existing verified cascade; chat cards follow its real events; links to pages.
3. Previous Chats on the encrypted store; Timeline adds attribution events.
4. Three pages: Lesion Analysis, Vessel Analysis, Research Findings (stored data only).
5. Explainability: append sections below the existing ones.
6. P4 carryover: provenance rejection tests, five-fold Grad-CAM (sum of per-fold signed maps of the mean target), Stage I attribution feasibility, occlusion benchmark.
7. PDF dossier with masks, vessels, heatmap.
8. Supabase auth + keyring + outbox sync (general/synthetic chats only, opt-in).
9. Screenshots at 1600/1280/1024/700, full gates, vault, report.

## Risks
- main.rs is 4.4k lines: new pages go in their own modules (`impl RetinaCareApp` in child modules, as `explain_app.rs`).
- LLM text is untrusted: rendered as plain text, never executed, never written to analysis records.
- Patient data never syncs: conversations with analysis cards or attachments are local-only by rule, not by setting.
- Heavy runs (cascade ~55 s, five-fold occlusion ~5 min, Grad-CAM 5.1 GiB/fold) stay sequential on one worker.
