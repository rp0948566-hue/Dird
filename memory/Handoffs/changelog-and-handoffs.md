---
title: Changelog and handoffs
type: changelog
domain: agent-handoff
status: VERIFIED
verified_on: 2026-10-28
source_refs:
  - "../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../docs/features/P0-02-stage2-segmentation/07-test-results.md"
  - "../../GUI/Hackthon dezine/docs/features/GUI-P05-clinical-report-export/README.md"
owner: Codex/Claude
tags: [history, handoff]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/changelog-and-handoffs.md (sha256 b6dacde19686); verify current state in [[active-phase]] -->
# Changelog and handoffs

## 2026-10-28 — Codex memory-first audit

Updated Dird/memory in place for Claude handoff. Audited backend and GUI
repository roots, branches, source contracts, model file presence, prior test
reports and blockers. No source code, models, GUI files or Git history changed.
Obsidian desktop UI attempt failed in sandbox; see [[Archive/hackthon-backend-superseded/README]].

## 2026-10-28 — P0.2 HE report

Prior Codex record documents authorized HE checkpoint, conversion and CPU
parity alongside MA. EX, CWS and vessel remain pending. Full record:
[Stage II tests](../../docs/features/P0-02-stage2-segmentation/07-test-results.md).

## GUI handoffs

Antigravity task reports document GUI Tasks 01–05. Task 05 points to Task 06
patient/exam management; this is a proposed next GUI task, not authorization
to edit GUI from Codex. See [[Integration/gui-source-findings]] and [[Handoffs/agent-ownership]].

## File-by-file memory change report

New notes (22):

- README.md — Obsidian opening steps, duplicate vault mapping and UI limits.
- 01-claude-start-here.md — two-minute, ten-minute and task-scoped reading path.
- next-actions.md — ordered gates, owners and approvals.
- agent-ownership.md — separate repos, branches and edit boundaries.
- source-map.md — exact code files and symbols.
- project-overview.md — product scope and research-only limit.
- architecture-overview.md — crate dataflow.
- model-pipeline.md — official three-stage AI flow.
- stage-i-screening.md — Stage I artifacts, preprocessing and evidence.
- model-inventory.md — local asset paths, hashes and license notes.
- stage-ii-segmentation.md — MA/HE verified path and remaining three models.
- stage-iii-grading.md — absent model, contract and prerequisites.
- gui-implementation.md — Tasks 01–05 with reported versus inspected status.
- data-contracts.md — Stage I/II and GUI boundary types.
- build-and-test.md — previous test/build evidence and current audit limits.
- cuda-and-packaging.md — CUDA, Ubuntu/Windows package status.
- licensing-and-data.md — model rights and image provenance.
- risks-and-blockers.md — verified mismatch and implementation risks.
- changelog-and-handoffs.md — dated events and this change inventory.
- docs-index.md — memory and repository documentation map.
- 90-claude-session-template.md — future memory update protocol.
- graph-overview.md — Mermaid map and measured graph audit.

Existing notes updated (10):

- 00-index.md — replaced stale DIRD+ links with the RetinaCare graph hub.
- active-phase.md — current branch, model statuses, tests and vault ambiguity.
- architecture.md — preserved as code-level map; clarified two GUI apps and model paths.
- decisions.md — current model approval and ownership rules.
- eye-ok.md — retained user note; added minimal Obsidian metadata.
- gotchas.md — replaced stale status with current reproducible limitations.
- retinacare-current-state.md — retained filename as redirect to active-phase.
- retinacare-phase-1.md — preserved initial objective, corrected stale current state.
- security.md — scoped native SQLCipher versus legacy browser storage and GUI export gaps.
- working-agreement.md — memory-first workflow metadata and start link.

No application source, Cargo files, checkpoints, external GUI files or Git history
were changed. The two repositories' prior uncommitted work remains intact.

## 2026-10-28 — Backend vault cleanup

- Owner set Hackthon Backend as the canonical backend vault; GUI vault stays with Antigravity.
- Dird HEAD reconciled: `9f13b5f` (local, unpushed) on top of `7286cf7`. The commit contains the whole native workspace and memory folder.
- MA/HE checkpoints and ONNX graphs re-hashed and copied to gitignored `Dird/models/stage2_hsmoe_aunet/`. Originals kept in /tmp.
- Vault README, 00-index, 01-claude-start-here, active-phase, next-actions, risks, agent-ownership, model-inventory, stage-ii, gotchas, architecture and source-map updated.
- No source code changed. No tests rerun.

## 2026-10-28 — Phase 2 M0 and M1 (MA/HE)

- M0: upstream v1.0 re-read from pinned commit; `LESION_CONFIGS` thresholds confirmed (MA 0.01, HE 0.20).
- M1: PyTorch vs ONNX parity on synthetic tensor passes for MA and HE. Rust ORT CPU heavy tests pass on local demo image (technical only).
- Added `Dird/scripts/stage2_lesion_parity.py` (dev-only parity tool, untracked until committed) and `Dird/docs/features/P0-02-stage2-segmentation/11-m1-ma-he-verification.md`.
- Dev venv `~/.venvs/retinacare-stage2` (Python 3.12, torch 2.14.1 CPU, timm 1.0.30, onnxruntime 1.31.1). Outside repo.
- No EX, CWS or VESSEL download. No source change to Rust crates. No GUI change.

## 2026-10-28 — Phase 2 M2 EX download and parity

- EX checkpoint approved by owner, downloaded from official v1.0 release, SHA-256 verified, stored in gitignored `Dird/models/stage2_hsmoe_aunet/ex/`.
- `scripts/stage2_export_lesion_parity.py` gained an EX spec. EX ONNX exported; PyTorch vs ONNX parity pass on deterministic tensor.
- Added `Dird/docs/features/P0-02-stage2-segmentation/12-m2-ex-export-parity.md`. Added `models/stage2_hsmoe_aunet/` to `.gitignore`.
- Not done: Rust EX runtime, image-level EX run, CWS, vessel.

## 2026-10-28 — Phase 2 M2 EX Rust runtime

- EX preprocessing (rect detection, letterbox), probability-based restore and flip TTA added to `retinacare-ai-bridge`.
- Heavy EX test matches upstream PyTorch reference mask on demo image (0 px disagreement).
- Gates: fmt, Clippy, workspace tests (62 pass, 8 ignored), release build, diff check pass.
- Added `Dird/docs/features/P0-02-stage2-segmentation/13-m2-ex-rust-runtime.md` and `scripts/stage2_ex_reference_fixture.py`.
- Not done: CWS, vessel, EX app-service test, licensed-image acceptance.

## 2026-10-28 — Phase 2 CWS

- CWS checkpoint authorized by owner, downloaded (no prior copy), SHA-256 verified, ONNX exported, parity pass.
- Rust CWS: rectangle letterbox, flip TTA, closing and component filtering, threshold 0.40. Heavy test and ModelService test match upstream reference (0 px disagreement).
- Gates pass; workspace 63 passed, 10 ignored.
- Added `Dird/docs/features/P0-02-stage2-segmentation/14-cws-plan-and-runtime.md`. Renamed fixture script to `scripts/stage2_rect_reference_fixture.py`.
- VESSEL not downloaded. Stage II not complete.

## 2026-10-28 — Phase 2 CWS

- CWS checkpoint approved and downloaded (SHA verified), exported to ONNX, parity pass.
- Rust CWS runtime (TTA + closing + component filter) matches upstream reference mask; nonblocking service tests for EX and CWS added.
- Gates pass; workspace 63 passed, 11 ignored. Evidence: `Dird/docs/features/P0-02-stage2-segmentation/14-cws-plan-and-runtime.md`.
- Not done: VESSEL, M4 five-mask service, licensed-image acceptance.

## 2026-10-28 — Phase 2 VESSEL and five-mask

- Vessel checkpoint authorized by owner, downloaded (no prior copy), SHA-256 verified. EMA weights exported to ONNX; parity pass.
- New `stage2_vessel.rs` (FOV, CLAHE, green blend, crop box, 4-view TTA, restore, opening, component filter). Heavy test matches upstream reference (0 px disagreement).
- Five-mask sequential ModelService test passes. Gates pass; workspace 67 passed, 14 ignored.
- Docs: `Dird/docs/features/P0-02-stage2-segmentation/15-vessel-plan-and-runtime.md`. Scripts: `stage2_vessel_export_parity.py`, `stage2_vessel_reference_fixture.py`.
- Still PENDING: licensed-image acceptance, CUDA, mask persistence. Stage II NOT complete. Nothing committed.
- Committed locally as `8fb2fee` on `main` (2 ahead of origin, not pushed). Model weights, ONNX and manifests stay gitignored.

## 2026-10-28 — Phase 3 M0 (Stage III LG-DRG research and plan)

- Read pinned upstream Stage III code. Confirmed two-head model, 8-channel contract, CORN consecutive decode with t=(0.38,0.60,0.19), gradability 0.10, fold-mean ensemble, no TTA, Stage I gate 0.2391716.
- No Stage III checkpoint exists locally; nothing downloaded.
- Added `Dird/docs/features/P0-03-lg-drg-grading/` (README, upstream-research, model-inventory, implementation-plan, testing-plan). Updated [[Models/stage-iii-grading]].

## 2026-10-28 — Phase 3 M2 (no checkpoint needed)

- New `stage3_input.rs` (eight-channel assembler, typed Stage II result validation) and `stage3_grading.rs` (CORN algebra, fold mean, consecutive decode). CLAHE helper shared with the vessel path.
- Heavy parity: eight-channel tensor equals upstream exactly (all channels, max abs 0) on the demo image with the five verified Stage II masks. CORN/ensemble vectors match upstream (48 and 24 cases).
- Gates pass; workspace 76 passed, 15 ignored. Docs: `Dird/docs/features/P0-03-lg-drg-grading/m2-eight-channel-and-corn-results.md`. Uncommitted.
- Next: owner approval for ONE fold download (fold 0). No grade can be produced before that. See [[Models/stage-iii-grading]].

## 2026-10-28 — Phase 3 fold 0 (M1 + M3, single fold)

- Owner approved ONLY fold 0. Downloaded `stage3_lgdrg_best_fold0.pt`, SHA-256 verified, gitignored. Loaded with `weights_only=True` (EMA).
- ONNX export (one `[1,4]` output) and PyTorch vs ONNX parity (max logit diff 2.9e-6). New `ModelRuntime::infer_stage3_fold` and typed `Stage3GradeResult` (scope SingleFoldTechnicalTest).
- Heavy Rust test: logits within 5.7e-6 of PyTorch, technical grade 3 on the demo image. Gates pass; workspace 78 passed, 16 ignored. Docs: `Dird/docs/features/P0-03-lg-drg-grading/m1-m3-fold0-export-and-single-fold-results.md`. Uncommitted.

## 2026-10-28 — Phase 3 M5A (Stage I → II → III fold 0 cascade)

- New `retinacare-app/src/cascade.rs` and `ModelService::analyze` / `cancel_analysis` / `try_next_analysis`; existing `ModelEvent` and `ModelSubmitError` untouched (GUI-safe). Sequential load-run-unload; bounded queue (2); duplicate ids rejected; cancel at stage boundaries.
- Real end-to-end trace on the demo image passed (five masks equal references; technical grade 3; CPU; peak RSS 2.2 GB; 232 s). All 13 heavy tests re-run green. Gates pass; workspace 90 passed, 17 ignored. Docs: `Dird/docs/features/P0-03-lg-drg-grading/m5a-source-audit-and-plan.md`, `m5a-cascade-results.md`. Uncommitted. See [[Models/stage-iii-grading]].

## 2026-10-28 — vault + GUI consolidation (Claude)
- Backups verified: `/home/claude/Desktop/retinacare-backups/2026-10-28-pre-consolidation/`.
- GUI relocated to `Dird/GUI/Hackthon dezine` (nested repo kept); parent excludes `/GUI/`. Changed: GUI `rust/Cargo.toml` path deps only.
- Tests: backend 90 passed/17 ignored, fmt+clippy clean; GUI 119 passed, clippy clean, `cargo fmt --check` fails (pre-existing).
- Vault consolidated into `Dird/memory` by `scripts/consolidate-vaults.py`; old vaults untouched as backups; stale copies in `Archive/`.
- Added: [[00-index]], [[01-claude-start-here]], [[active-phase]], [[Handoffs/next-actions]], [[Integration/readiness-report]], [[Integration/repo-topology]]. No commits.

## 2026-10-28 — GUI Stage I adapter swap (Claude, approved by user; uncommitted)
- GUI `rust/src/engine/`: `backend.rs` (PipelineStage -> Stage1ScreeningGate/Stage2Segmentation/Stage3Grading; manifest discovery via CARGO_MANIFEST_DIR; ignored live test), `manifest.rs` (Stage I = ONNX, VerifiedActive), `report.rs` (model id/hash/provider from the backend result, not constants), `main.rs` (manifest test).
- Tests: GUI 119 passed + clippy clean; ignored live test loads real Stage I via `LiveCodexBackendService` on demo image: passed (31 s, hash == manifest). Remaining hardcoded hashes are mock/test fixtures only.

## 2026-10-28 — Actual GUI + backend integration (Claude, user-approved, uncommitted)
- Backend: `retinacare-persistence` schema v2 (audit ledger, analysis/image setters, rollback-safe migration + backup), `retinacare-reports` PDF writer. No cascade/ModelService code changed.
- GUI: `engine/cascade.rs`, `engine/store.rs`, `ui/cascade_cards.rs`, live patient/audit/PDF services, Quad/Report/Timeline wiring, intake limits, toolbar + narrow-layout fixes, stale-image clearing on patient/exam change, shutdown cancels the running job. Details: [[Integration/gui-backend-integration]].
- Tests: backend 97 passed/17 ignored; GUI 166 passed/2 ignored (+2 heavy ignored passed on real models); fmt, clippy -D warnings, release builds pass. Evidence folder: `docs/features/GUI-BACKEND-INTEGRATION/`.
- Models: HE/MA `manifest.json` copied from `/tmp/retinacare-models` into `Dird/models/stage2_hsmoe_aunet/{he,ma}` (sha256 verified equal to the ONNX files; gitignored).

## 2026-10-28 - post-compaction state sync (documentation only)
- Verified: git state of both repos, legacy vault pointers (4 copies), sensitive-file scan (clean), vault links (727 ok).
- Rerun: backend fmt/clippy/release pass, tests 97 passed / 0 failed / 17 ignored; GUI fmt/clippy/release pass, tests 166 passed / 0 failed / 2 ignored. Heavy model tests, CUDA, Windows not rerun.
- Pushes this session (user-approved): GUI `dezine` `4814064`; `AI-Manthan-2.0` `main` force-replaced then fast-forwarded to combined `0997866`; backend to `backend-rust` `01f4cb7`. `Debaq/Dird` still 403.
- Files: new `README.md`, `.obsidian/app.json`, `Integration/project-state-sync-2026-10-28.md`; edited `00-index`, `active-phase`, `Handoffs/next-actions`, `Integration/repo-topology`. No source code changed. Uncommitted.

## 2026-10-28 - P1 Stage I hardening
- Added tests: stage1 boundary/sweep/extreme (4), cascade Stage I failure and threshold (2), preprocessing edge cases (5), ONNX graph-vs-manifest mismatch (1), ignored real-model CPU profile and CUDA proof tests. New: `scripts/stage1_cuda_check.sh`, `docs/features/P1-stage1-hardening/README.md`. No GUI code changed.
- Downloads (user-approved): NVIDIA CUDA 12 and 13 runtime wheels from PyPI into `/home/claude/Desktop/retinacare-cuda/` (outside git).
- Results and blockers: [[Models/stage-i-screening]], [[Testing/cuda-and-packaging]].

## 2026-10-28 - P2 five-mask persistence
- New `native/crates/retinacare-persistence/src/masks.rs` (schema v3, atomic save, verified load, RLE); GUI wiring in `engine/{cascade,patient}.rs`, `main.rs`, `ui/cascade_cards.rs`; 11 persistence tests, 4 new GUI flow tests, real 7-model roundtrip in the ignored heavy test.
- Run now: backend 120/0/18 ignored, GUI 170/0/2 ignored, fmt/clippy/release clean; heavy GUI run 63.7 s, 2826 MiB, five real masks reloaded pixel-identical (save 2.9 ms, load 4.6 ms, DB 86 KB).
- Evidence and matrix: `docs/features/P2-mask-persistence/README.md`. Nothing committed or pushed. Blocked: labelled Dice/IoU, CUDA, Windows.

## 2026-10-28 - P2 desktop acceptance and GUI push
- Real release GUI on a private Xvfb display, synthetic DB and patient SYN-P2-001: create DB, register, save exam (demo image OD-1.png, technical use only), Run Full Analysis (67.7 s, CPU, grade 3 research-only, five masks stored), close (exit clean), relaunch, locked state shown, unlock, restore with overlays and "No model was run".
- Evidence: `docs/features/P2-mask-persistence/evidence/` (sanitized screenshots, app logs, test logs).
- GUI commit `1565326` on `dezine`, pushed to origin (AI-Manthan-2.0); remote ref verified. Backend `a3c7b27` local only.
- Tests rerun: backend 120/0/18, GUI 170/0/2; fmt, clippy, release, diff-check clean.

## 2026-10-28 - Patient Directory redesign (GUI, uncommitted)
- Files: GUI `rust/src/ui/patients.rs`, `ui/theme.rs`, `engine/patient.rs`. No backend or AI code changed.
- Verified: fmt, clippy -D warnings, GUI tests 170 passed / 2 ignored, release build; real desktop screenshots at 1600x900, 1280x820, 1024x768, 700x650 plus locked, first-use, empty, no-match and filter states; register, search, filters and refresh driven in the live app. Evidence: `docs/features/GUI-PATIENTS-REDESIGN/`.
- Not committed or pushed. Not redesigned: profile, exam detail, Previous Chats, Timeline, Report.

## 2026-10-28 - Supabase Stage A (design, nothing applied)
- Added `supabase/migrations/20261028120000_chat_persistence.sql` (draft), `supabase/design/patient_cloud_design.sql` (design only), `docs/features/SUPABASE-INTEGRATION/README.md`, [[Backend/supabase-architecture]]. Project reachable (empty public schema); keyring available. No cloud change, no Rust change. Awaiting SQL approval.
- 2026-10-28 (later): migration `chat_persistence` applied to `ebflochdtajcflmlhgqa` and verified (5 tables, forced RLS, 3 storage policies, private bucket, no anon grants, security advisors 0 findings, performance INFO only). Next: Stage B.

## 2026-10-28 - Phase 3 five-fold ensemble (committed locally: parent `1937ed9`, GUI `7097319`; not pushed)
- Approved downloads of folds 1-4 (official v1.0, SHA-256 verified; fold 4 needed one retry). ONNX export and PyTorch/Rust parity for all folds; installed pandas, scikit-learn, albumentations into `ai_engine/.venv` (dev only).
- Code: `stage3_grading.rs` (`grade_five_fold_ensemble`, `combine_five_folds`), `retinacare-app/src/cascade.rs` (`stage3_other_folds`, `grade_cancellable`, `collect_folds`), `model_runtime.rs` (heavy test reads the fold), GUI `engine/{cascade,report,manifest}.rs`, `ui/{cascade_cards,viewer}.rs`, `main.rs` test text. 12 new backend tests, 2 new GUI tests.
- Verified: backend 134/0/18 ignored, GUI 172/0/2 ignored, fmt, check, clippy, release; heavy five-fold cascade and GUI-adapter run; live desktop save, restart, restore. Evidence: `docs/features/P3-five-fold-ensemble/`.

## 2026-10-28 - Phase 4 explainability (uncommitted)
- New: `attribution.rs`, `stage3_explain.rs` (ai-bridge), `attributions.rs` (persistence, schema v4 with backup), `scripts/stage3_gradcam.py`, reports picture pages, GUI Explainability page (`ui/explain.rs`, `engine/explain.rs`, `explain_app.rs`), Quad fourth panel + link, PDF section.
- Verified: backend 157/0/21 ignored, GUI 188/0/2 ignored, fmt, clippy -D warnings, release builds; heavy: occlusion fold 0 and five-fold, deletion/insertion, Grad-CAM parity, full five-fold cascade regression; live desktop analysis, explanation, cancel, PDF, restart restore, 1280/1024/700 screenshots. Evidence: `docs/features/P4-explainability/README.md`, [[Models/explainability]].
- Not done: Stage I explanation, five-fold Grad-CAM, CUDA, Windows, labelled evaluation. Nothing committed or pushed; Phase 5 not started.


## 2026-10-29: Phase 4 and Phase 5 (committed locally, not pushed)
- Phase 4 commits: parent `8581790`, GUI `0f14417`. Backend 157 pass / 21 ignored; GUI 188 pass / 2 ignored at that time.
- Phase 5 commits: parent `4a936ec`, GUI `c3e4665`. Backend 172 pass / 21 ignored; GUI 225 pass / 3 ignored.
- Built and run on desktop: Mode R Home with live cards, Lesion/Vessel/Research Findings pages, Stage I and five-fold Grad-CAM, encrypted chats v5, PDF evidence grid, stable DB location.
- Blocked: Mode C (no `llama-server`), live Supabase two-user test, mask ablation, Windows.
- Evidence: `docs/features/P5-product-completion/` (README, plan, 27 screenshots). Ledger: [[Handoffs/p5-progress]].

## 2026-10-29: GPU provider switch (committed and pushed, no force, nothing on main)
- Dird `test-gpu` `7c7144c`: `ai-bridge/{device,model_runtime,stage3_explain}.rs`, `app/cascade.rs` (`GpuNotVerified` -> `ModelUnavailable`), `reports/lib.rs` (`preview_pages`, needed by the GUI build).
- GUI repo `test` `5ce9ac9` (was `7f94938`): `rust/Cargo.toml` (bridge `features=["cuda"]`), `engine/backend.rs` (real provider labels), `main.rs` (sidebar status from readiness), `chat_app.rs` (image prompt -> analysis, text -> assistant), `ui/explain.rs` (`ellipsize`), `ui/patients.rs` (clipped labels, themed chips).
- Verified: fmt and clippy clean (native workspace, bridge `--features cuda`, GUI); native tests bridge 109/15 ignored, app 25/8, domain 2, persistence 34, reports 4, 0 failed; GUI 241 passed/3 ignored/0 failed; parity + provider matrix in [[Testing/cuda-and-packaging]].
- Not committed on purpose: `CLAUDE.md`, `memory/GUI/figma-design-system.md`, Figma-brief PDFs, CUDA libs, models, DB.
- Not checked: visual pass of Patients profile, Explainability header, Vessel panel fixes.

## 2026-10-29 (later): final demo verification and fixes
- Ran the real release GUI on Xvfb (`RETINACARE_UI=v2`): real image upload, Stage I (neg 0.0650 / pos 0.9983), Stage II five masks on CUDA, Stage III grade 3 on CPU, saved-exam re-run, 3-page report, real PDF (pdfinfo/pdftotext), five-fold Grad-CAM (110.7 s, pytorch-cpu, stored), chat via Ollama, Previous Chats persistence, Patients/Timeline in light and dark, 1600x900 and 1280x820.
- Fixes: hardcoded "on the CPU" text now lists real devices; Ollama 500 body shown and wrapped in the error banner; double-click on a Patients row no longer opens the legacy profile; sidebar Patients always lands on the directory; Previous Chats colours theme-aware (light mode readable); long image filename ellipsized in Retinal Review.
- Files (GUI repo): `rust/src/{chat_app,main}.rs`, `engine/ollama.rs`, `ui/{explain,history}.rs`, `ui2/{patients_page,review_page}.rs`.
- Tests: GUI 241 passed/3 ignored; native fmt+clippy clean, bridge 109/15 ignored, app 25/8, domain 2, persistence 34, reports 4; parity and provider matrix pass.
- Ollama: system install broken (`llama-server` missing, needs sudo). Official v0.40.13 extracted to `/home/claude/ollama-user` (outside git); run it on port 11435 and start the GUI with `RETINACARE_OLLAMA_URL=http://127.0.0.1:11435`.

## 2026-10-29 medical research chat
Changed (GUI `rust/src`): `engine/papers.rs` (new), `engine/ollama.rs`, `engine/mod.rs`, `domain.rs`, `chat_app.rs`, `main.rs`; skill dir `.claude/skills/retinacare-medical-research/`. Tests: fmt+clippy clean, 248 pass/6 ignored (live Ollama). Open: 1600x900, oversized/encrypted PDF, chip persistence, retry history, branch policy (`dezine` vs `test`). Note: [[GUI/medical-research-chat]].

## 2026-10-30 analysis results list, long file names, push to main
- Post-analysis chat reply is now a fixed headed bullet list (`chat_app::results_message`), not model prose. Stage I/II/III, run time, research-only line.
- Long attachment names cut to 40 chars (`explain::ellipsize`) in chip, card, bubble. File on disk keeps its name.
- Explainability empty = scratch run not saved to a patient exam. Not a bug. Auto-save to a patient is NOT built (needs a decision).
- Pushed to `main`: GUI repo AI-Manthan-2.0 (`GUI/Hackthon dezine/` in main layout) and Dird root `memory/`. Tests 251 pass/6 ignored.
