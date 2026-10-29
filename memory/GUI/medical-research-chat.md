---
title: Medical research chat (streaming, PDF papers, source chips)
type: feature
status: READY-DEMO
verified_on: 2026-10-29
---
# Medical research chat (GUI repo `rust/`, uncommitted)
Doctor-style, not a doctor. Local Ollama `llama3.2:3b` via `RETINACARE_OLLAMA_URL`, true NDJSON token streaming.

## Code
- `rust/src/engine/papers.rs` (new): PDF -> pages -> 1200-char chunks, lexical top-4, `SourceRef` chips from retrieval (never from model text), `unverified_citations` check `[label p.N]`, doc id = sha256[..12].
- `engine/ollama.rs`: safety system prompt, `keep_alive` (default 30 s, `RETINACARE_OLLAMA_KEEP_ALIVE`), no-match path skips the model (fixed reply, no invented DOI), metrics line on stderr.
- `domain.rs`: `ChatEvent::Sources`, `SourceRef`; `chat_app.rs`: PDFs go to papers, images refused; `main.rs`: chips, excerpt window, retry clears sources.
- Env hooks: `RETINACARE_AUTOPICK_PDF`, `RETINACARE_AUTOPICK_IMAGE`, `RETINACARE_THEME=light`.
- Skill: `GUI/Hackthon dezine/.claude/skills/retinacare-medical-research/`.

## Verified (release GUI 1280x820, dark+light)
Streamed answer, chip `paper.pdf · p. 2`, click opens page excerpt, no-match refusal, scan-only/corrupt/injection PDFs, safety suite 10 prompts, fundus Stage I-III after VRAM freed. fmt/clippy clean, 248 tests pass.
TTFT: cold 2978 ms, warm 22-51 ms, 115-135 tok/s (single runs).

## Not done / risks
1600x900 NOT_RUN; GUI cold TTFT; oversized/encrypted PDF; skill discovery BLOCKED (org disabled `claude -p`); chips not persisted across restart; retry drops history; excerpt viewer is text, not rendered page; 3B model weak in Hinglish, sometimes "MA"=macroaneurysm.
GPU: Ollama + retinal CUDA models share 8 GB; analysis within 30 s of a chat reply can fail Stage II ("model inference failed"). Unload Ollama first.
Verdict: demo-usable research prototype, NOT declared READY.

## 2026-10-29 follow-up pass (uncommitted on top of local commit 925881d)
- VRAM: `ollama::release_vram()` (api/ps -> keep_alive 0 -> poll) runs in `start_analysis` before the cascade. Verified with Ollama resident (6.2 GB used, keep_alive 5m): analysis 38.0 s, no Stage II failure.
- No-match with a PDF attached: labelled general answer (`papers::GENERAL_LABEL`) instead of refusal; label sent after Ollama answers.
- Sources + note persist in `card_json` of assistant text rows (local encrypted store only; Supabase sends `card_json: None`). Retry reuses history + results context; restored message with a lost paper asks to re-attach.
- Size limit: `validate_attachment_path` stats before any read (50 MB); regression test in `security.rs`.
- Checks on replies: `ungrounded_numbers`, `health_overclaim`, fixed text-only reply when asked "did you look at my image".
- Known: 3B model still can say odd things (Hinglish quality, loose general knowledge); notes only catch invented numbers, health overclaims, bad citations.
- GUI verified: 1280x820 dark+light, 1600x900 dark+light; fmt, clippy -D warnings, 251 tests, native workspace tests pass.
- Results message: after analysis the chat shows `results_message` (fixed list), the 3B model only answers follow-up questions.
