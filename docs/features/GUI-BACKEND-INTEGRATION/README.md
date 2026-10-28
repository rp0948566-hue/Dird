# GUI + backend integration - acceptance evidence (2026-10-28)

One binary: `GUI/Hackthon dezine/rust/target/release/retinacare` (Antigravity eframe/Glow app, in-process Rust path dependencies on the backend crates). Not committed or pushed. Verdict **A**: integrated and demonstrably working; partial items listed below.

Reproduce: `cd "GUI/Hackthon dezine/rust" && cargo build --release && ./target/release/retinacare`. Screenshots/PDF in this folder come from the real release binary run under Xvfb (isolated X11, throwaway DB and password, demo image `dist/demo-images/session-1-final/OD-1.png`, provenance unknown = technical use only).

## Gates (final run)
| Gate | Result |
|---|---|
| backend `cargo fmt --check` / clippy `-D warnings` | pass / pass |
| backend `cargo test --workspace` | 97 passed, 0 failed, 17 ignored (heavy, env-gated) |
| backend `cargo build --release` | pass |
| GUI `cargo fmt --check` / clippy `-D warnings` | pass / pass |
| GUI `cargo test` | 166 passed, 0 failed, 2 ignored |
| GUI `cargo test -- --ignored` (real Stage I; real 7-model cascade through the GUI adapter) | 2 passed (debug build: cascade 224 s, peak RSS 3.78 GB) |
| GUI `cargo build --release` | pass (binary 60.9 MB) |
| `git diff --cached --name-only` (both repos) | empty; models gitignored; `/GUI/` excluded from the parent; no commits |

## Measured (release GUI, CPU, 640x640 demo image)
Full cascade 73.5 s and 67.3 s wall (two GUI runs); per stage (run 1): screening 3.8 s, MA 5.2, HE 4.7, EX 13.8, CWS 14.1, vessel 5.5, grading 1.8 (+ model loads). Peak RSS 2951 MiB (backend report). Idle GUI RSS 1.38 GB (Stage I preloaded). Close during a run: process exits in 6.1 s (cooperative cancel; was 57 s before the fix). UI stayed interactive during runs (page navigation while running). CUDA not run: all providers report CPU.

## Acceptance checks (PASS / PARTIAL / BLOCKED / NOT RUN)
| ID | Result | Evidence |
|---|---|---|
| A01 | PASS | launched `target/release/retinacare`; one bin in the crate |
| A02 | PASS | `Cargo.toml` path deps; live cascade ran in-process |
| A03 | PASS | real clicks: Home, Report/Quad, Patients, Previous Chats, Timeline, Plugins (`screenshots/`) |
| A04 | PASS | Glow retained; 95 s idle then click switched view (`f1024` run) |
| A05 | PARTIAL | request carries patient/exam/eye/sha256/size (unit + live). Native file dialog cannot run in Xvfb: image loaded from a saved exam and `RETINACARE_LOAD_IMAGE` |
| A06 | PASS | navigated Timeline/Report during a 70 s run |
| A07 | PASS | demo image: score 0.9983 >= 0.2392 -> Stage II x5 -> Fold 0 |
| A08 | PARTIAL | negative route via scripted events + backend tests; no real negative image (synthetic non-retinal image still routed positive) |
| A09 | PASS | MA/HE/EX/CWS/vessel overlays rendered (`result-quad.png`) |
| A10 | PASS | banner "SINGLE-FOLD TECHNICAL TEST (LG-DRG fold 0) - RESEARCH ONLY" |
| A11 | PARTIAL | ungradable distinct in tests; no real ungradable image |
| A12 | PASS | panel 4 "Explainability unavailable", no heatmap |
| A13 | PASS | event trace below; Started, stages, one Finished |
| A14 | PARTIAL | QueueFull unit-tested; duplicates are blocked in UI so real queue-full is unreachable |
| A15 | PASS | Run disabled while active; unit test |
| A16 | PASS | real UI cancel -> "Cancelled - no grade"; follow-up run enabled (`cancelling/cancelled.png`) |
| A17 | PASS | unit tests; real UI: switching patient clears image/results |
| A18 | PARTIAL | failed run publishes no grade (scripted); no real model failure induced |
| A19 | PASS | identity verified in the live runs (sha256 32d756fc..., 640x640); mismatch tests |
| A20 | PASS | worker thread + channel; UI responsive during runs |
| A21 | PASS | restart, unlock, patient + exam present (`restart-unlocked.png`) |
| A22 | PASS | saved exam + result summary after restart (`restart-report.png`); timeline from records |
| A23 | PASS | Previous Chats: "EPHEMERAL - PERSISTENCE NOT CONNECTED", no fake sessions; reopening never runs models |
| A24 | PARTIAL | timeline renders real records (one exam); filters not exercised on multi-record data |
| A25 | PASS | real export by click; PDF opened by an independent reader (`sample-report-OD-1.pdf`) |
| A26 | PASS | audit rows survive restart (`audit-ledger.png`); locked DB -> ledger unavailable |
| A27 | PARTIAL | plaintext-export screen shows the warning and scope; export action not exercised in the final desktop run |
| A28 | PASS | tests: v1->v2 migration keeps data + `.pre-v2.bak`; failed migration rolls back |
| A29 | PARTIAL | fixtures only with `RETINACARE_DEV_FIXTURES`, own mock audit; not screenshotted this session |
| A30 | PASS | nothing staged; models ignored |
| A31-A34 | PASS | gates table |
| A35 | PASS | `idle-1280x820.png`, `idle-1024x768.png`, `idle-700-fixed.png`, `f1024-result.png` |
| A36 | PARTIAL | corrupt/oversized/unreadable image + PDF write failure covered; missing-model/bad-checksum covered by backend tests and the GUI "unavailable" path, not induced in the desktop |
| A37 | PARTIAL | idle, running cancel, follow-up request covered (real + unit); queued cancel only in backend tests |
| A38 | PASS | real timings above |
| A39 | PASS | banners, report limitations, PDF limitations |
| A40 | PASS | vault links resolve (714 wikilinks, 0 broken) |
| A41 | PASS | heads `8fb2fee` (parent) and `82db168` (GUI); no push/reset/delete |
| A42 | PASS | patient -> exam -> image -> analysis -> viewer -> report -> PDF -> audit -> restart |

## Edge cases
E01 PASS (no Run without image; message) | E02 PASS (corrupt/oversized rejected before any model; real corrupt-file screenshot) | E03 PARTIAL | E04 PARTIAL (scripted) | E05 PARTIAL | E06 PASS | E07 PASS | E08 PARTIAL | E09 PARTIAL (unit) | E10 PARTIAL | E11 PARTIAL | E12 PASS (real: stops at stage boundary) | E13 PASS | E14 PASS (no ETA, measured clock) | E15 PASS (6.1 s) | E16 PASS | E17 PASS (eye switch drops results) | E18 PASS (tests; wrong password refused) | E19 PASS (rollback test) | E20 PASS | E21 PARTIAL | E22 PASS | E23 PASS | E24 PARTIAL (toolbar scrolls; other pages at 700 px not all checked; Previous Chats banner overlaps its controls - pre-existing, design phase) | E25 NOT RUN (CPU only, CUDA unverified, nothing claimed) | E26 PASS (technical-only labels) | E27 PASS | E28 PASS | E29 PASS | E30 PASS.
PARTIAL = covered by unit/scripted tests or backend tests but not reproduced on the desktop with real models.

## Sample analysis event trace (GUI adapter, real models, debug build)
Started 26.7 s -> Screening 58.0 -> MA 94.4 -> HE 125.7 -> EX 164.2 -> CWS 204.1 -> Vessel 223.7 -> Grading 251.1 -> Finished Graded (grade 3, research only). 26.7 s before Started is the worker finishing the Stage I preload.
