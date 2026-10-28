# P3 five-fold LG-DRG ensemble - evidence (2026-10-28)

Research prototype. Technical parity only; no clinical validity. Weights CC BY-NC 4.0, kept out of Git. Demo image `OD-1.png` has unknown provenance (technical use only).

## Status
| Item | Status |
|---|---|
| Checkpoints, folds 0-4 | VERIFIED: size 795,798,428 B and full SHA-256 equal the pinned `weights_manifest.json` (below) |
| ONNX export and PyTorch parity, folds 0-4 | VERIFIED on CPU (seeded and demo-image tensors) |
| Rust per-fold inference vs PyTorch, folds 0-4 | VERIFIED (demo image) |
| Exact ensemble and CORN decoder (`grade_five_fold_ensemble`, `combine_five_folds`) | IMPLEMENTED, unit-verified, equals the PyTorch-reference ensemble on the demo image |
| Cascade integration (sequential folds, cancel between folds, no partial grade) | IMPLEMENTED, unit-verified, real CPU run VERIFIED |
| GUI scope labels, persistence, restore | IMPLEMENTED, desktop-verified on a synthetic exam |
| Licensed labelled evaluation, CUDA, Windows, clinical validation | BLOCKED / NOT DONE |
Engineering scope of Phase 3: COMPLETE for the CPU technical path. Research-only; the ensemble grade is not clinically validated.

## Decisions and design
- Official math, from the pinned upstream (`corn.py`): per-fold 5-way vector, elementwise mean of five vectors, `corn_from_full` inversion of the MEAN, strict `>` thresholds (0.38, 0.60, 0.19) and `p_gradable < 0.10` for grade 5. Never a logit mean or a vote (test `ensemble_decodes_the_mean_of_probabilities_...`).
- `grade_five_fold_ensemble` needs exactly folds 0-4 once each; missing, duplicated, out-of-range or non-finite folds reject the whole ensemble (no partial grade).
- `combine_five_folds` also checks each result's pinned checkpoint hash, examination, image hash and provider, and keeps all five checkpoint and model hashes in the result.
- Cascade: `CascadeModelPaths.stage3_other_folds` is empty (single-fold technical test, fold 0) or exactly four manifests (ensemble); any other count fails as model-unavailable. The GUI enables the ensemble only if all four extra manifests exist. Folds load, run and unload one at a time; cancellation is checked before every fold (an in-flight ONNX call cannot be interrupted). A failed or cancelled run publishes no grade.
- The GUI banner, headline, report status and limitations follow the actual result scope. A stored fold-0 result keeps its single-fold label (test `a_historic_single_fold_result_keeps_...`).

## Per-fold evidence (demo image `OD-1.png`, technical only)
| Fold | Checkpoint SHA-256 (verified) | ONNX SHA-256 | gamma (EMA) | ONNX vs PyTorch max abs logit diff (seeded / demo) | Rust vs PyTorch (demo) | Rust inference | Fold grade |
|---|---|---|---|---|---|---|---|
| 0 | c1593eac...f57c | 31f20d62...129cb7 | 0.0102 | 2.7e-6 / 2.9e-6 | 5.7e-6 | 1.14 s | 3 |
| 1 | 9efb5c93...9714eb | 3f0deb84...087092620 | 0.0112 | 2.0e-6 / 8.6e-6 | 7.6e-6 | 1.11 s | 3 |
| 2 | 2336a1a5...b91e0f | a55301c7...0df3a3ab | -0.0099 | 1.9e-6 / 4.1e-6 | 3.8e-6 | 1.14 s | 3 |
| 3 | 67341f89...acdb99 | ee7c1661...6fcfe | 0.0088 | 2.3e-6 / 6.7e-6 | 4.8e-6 | 1.10 s | 3 |
| 4 | 239107b0...ee888e | 659e65fb...876238 | -0.0107 | 2.4e-6 / 7.6e-6 | 8.6e-6 | 1.04 s | 2 |
Full hashes are in `models/stage3_lgdrg/manifest_fold{0..4}.json` (gitignored folder; not committed). All ONNX files are 795,936,525 B. EMA weights, opset 17, torch 2.14.1+cpu, onnxruntime 1.31.1. Rust inference time is the session run plus preprocessing, load excluded. Fold 4 disagrees with the other four on this image (grade 2 vs 3), so the mean, not a vote, decides.

## Ensemble result (demo image)
Mean five-way vector `[8.27e-5, 0.2377, 0.7621, 1.38e-5, 8.1e-6]` (G1..G4, ungradable). Rust grade 3, PyTorch-reference grade 3, max abs difference of the mean vector 2.98e-8.

## Full CPU cascade (real models, release, ModelService)
Heavy test `local_full_cascade_technical_trace_matches_references` with all four extra fold manifests: Stage I 5.99 s, MA 6.64, HE 4.93, EX 10.52, CWS 10.93, vessel 3.93, Stage III five folds 11.93 s (load plus run, sequential); total 54.9 s wall, peak RSS 2.15 GiB. All five Stage II masks equal the upstream reference masks pixel for pixel. Heavy GUI adapter test `live_cascade_runs_through_the_gui_adapter` (release): five-fold grade 3, 55.4 s, 2795 MiB process RSS. Logs: `evidence/five-fold-cascade-heavy-test.log`, `evidence/rust-fold*.log`, `evidence/export-fold*.log`.

## Desktop acceptance (live native app, private display, synthetic exam, CPU)
1. Reopen the existing fold-0 exam: "Single-fold technical severity grade 3", stored masks restored, no model run (`01-...png`). Historic single-fold labelling preserved.
2. Run Full Analysis: "Five-fold ensemble severity grade 3 (research only)", 55.3 s, result and five masks saved (`03-...png`); Report page states the ensemble banner (`04-...png`).
3. Close (clean exit), relaunch, unlock, reopen: summary and masks restored, "No model was run" (`05-...png`). Post-restore process CPU was transient UI load, no inference.
4. Report subtitle fixed to follow the scope (`06-...png`). PDF export content is covered by the unit test; a PDF was not re-exported in this session.

## Tests (run now)
Backend 134 passed, 0 failed, 18 ignored. GUI 172 passed, 0 failed, 2 ignored. fmt, check, strict clippy, release builds and `git diff --check` pass in both repos. Heavy tests run: single-fold parity for folds 0-4, full five-fold cascade, GUI-adapter cascade. Not rerun this session: the individual Stage I and Stage II `local_*` parity tests (covered end to end by the cascade test's Stage I route and pixel-equal mask checks).

## Edge cases (matrix)
Covered by tests: absent or partial fold set (E01, `a_partial_fold_configuration...`, `missing_fold_manifests...`), substituted checkpoint (E02, `combining_rejects...`), nonfinite fold (E14), fold failure without partial grade (E15), cancel before next fold (E16, E17 semantics: only between folds), Stage I negative route and failure routing (E18, E19, existing P1 tests), ungradable override and exact thresholds (E11-E13), stale masks and wrong identity (E06-E08, existing cascade and Phase 2 tests), historic fold-0 label (E21). Not tested: E26 restart mid-fold, E28 wrong ONNX runtime version, E23 CUDA (not run, never reported as CUDA).

## Remaining blockers
Licensed, labelled evaluation (no QWK or accuracy claim), CUDA on the RTX 5070 (GPU in error state), Windows, the demo image has unknown provenance, a real power-loss test, and Phase 4 explainability. Nothing was committed or pushed.

## Python environment note
The existing `ai_engine/.venv` lacked three imports of upstream `engine.py`; installed (dev-only, in the venv): `pandas 3.1.0`, `scikit-learn 1.9.1`, `albumentations 2.0.8`. torch 2.14.1+cpu and onnxruntime 1.31.1 are unchanged. The export script does not use albumentations preprocessing, so the version difference from upstream's 1.4.24 does not affect export or parity.
