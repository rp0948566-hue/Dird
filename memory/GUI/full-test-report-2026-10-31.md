# Full test report, 2026-10-31

Scope: test only, no feature code changed in this pass. Weekly usage was 78% at start (target cap 80%), so the run was cut to cheap, high-value checks. Items marked NOT RUN were not tested.

## Results
| Area | Result | Evidence |
|---|---|---|
| GUI crate (`GUI/Hackthon dezine/rust`) `cargo test` | PASS | 277 passed, 0 failed, 9 ignored |
| Backend workspace `cargo test --workspace --manifest-path native/Cargo.toml` | PASS | 109 + 25 + 2 + 36 + 8 passed, 0 failed, 23 ignored |
| Backend persistence: `delete_all_conversations` | PASS | unit test |
| GUI clippy (earlier this session) | PASS | clean |
| Release build, GPU-only launcher (`RETINACARE_PROVIDER=cuda`) | PASS | built and running earlier in session |
| Test images (3 WhatsApp files, 1179x1179, 361x390, 1179x1205) | PARTIAL | decode OK, sizes recorded; NOT run through the full GUI pipeline |
| Doctor/RBAC role-play, screenshot GUI sweep, broken/huge/corrupt image fuzzing | NOT RUN | weekly budget exhausted |
| "libpng error: PNG input buffer is incomplete" in backend tests | INFO | printed by a negative test (truncated PNG); tests still pass |

## Edge cases to run next (not yet done)
- 361x390 low-resolution image: expect a quality warning, not a grade.
- Non-fundus image (photo, screenshot): expect rejection or a low-confidence flag.
- Two images at once, 0-byte file, truncated PNG, wrong extension.
- Restart after analysis: results, masks, chat and report identical.
- Delete all chats: patients, exams, reports remain.
- Role checks: doctor vs. technician vs. viewer on export, delete, reassign.
- Ollama down: chat shows "Chat assistant unavailable" and the app keeps working.

## Known limits
- GUI repo `main` has diverged from `origin/main` (not force-pushed).
- 3B local chat model still imperfect on some leading prompts; stage-status questions are answered deterministically.
- Folder animation, single-image thumbnail, table wrap, delete-all button, PDF look: not visually verified after the last build.
- Research prototype, not a diagnostic device.
