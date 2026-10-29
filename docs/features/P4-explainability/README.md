# Phase 4: authentic explainability and the Explainability workspace (2026-10-28)

Research prototype. Technical evidence only; no clinical validation. Test images are the unlicensed demo `OD-1.png` (provenance unknown, technical use only) and synthetic patient records. Attribution is influence evidence for one named model score. It is not a lesion segmentation, not a localisation and not a diagnosis. This is a RetinaCare feature, not an official HierarchiRetina stage.

Status: engineering PARTIAL-COMPLETE. Real attribution (occlusion through the pinned ONNX sessions, Grad-CAM through the pinned PyTorch EMA weights), encrypted storage, restore after restart, PDF section with the real overlay, the sidebar page and the Quad fourth panel are implemented and verified on CPU. NOT done: Stage I explanation, Grad-CAM of the five-fold mean, CUDA, Windows, labelled evaluation (no licensed set), mask-channel ablation. Nothing was committed or pushed.

## Targets, scopes, algorithms (definitions in code: `native/crates/retinacare-ai-bridge/src/attribution.rs`)
| Item | Definition |
|---|---|
| Target score | A probability from the real 5-way vector of the pinned CORN algebra. `Stage3Severity{g}` (g 1-4, single fold) = `full[g-1]`; `Stage3GradeProbability{g}` (g 1-5, five-fold only) = `full[g-1]` of the MEAN of the five fold vectors (never fold 0, never a vote); `Stage3Gradability` = 1 - `full[4]`. |
| Scope | `Stage3SingleFold{fold 0-4}` or `Stage3FiveFoldEnsemble` (exactly folds 0-4, each once). Stage I scope/target exist in the contract but are rejected by the job validator (Unavailable). |
| Occlusion | 8x8 grid, 64 px patches, stride 64, RGB channels 0-2 replaced by 0.0 (ImageNet mean colour); the five mask channels stay fixed (RGB influence only). Cell = baseline score minus perturbed score through the real session(s). Overlaps are averaged by contribution count. Five folds are run fold-major (one 0.8 GB session alive); every target is linear in the 5-way vector, so the mean of per-fold deltas equals the delta of the mean-probability target exactly. |
| Grad-CAM | `scripts/stage3_gradcam.py`, one fold, `ai_engine/.venv` + pinned upstream clone, `ema` weights, `weights_only=True`, size and SHA-256 checked. Layer: `norm(gate(F_img, F_les))`, 1536x16x16, the last spatial map before global pooling and both heads. alpha_k = mean dS/dA_k; map = ReLU(sum alpha_k A_k); explained S is the differentiable 5-way probability (same algebra as grading). Bilinear upsampling (align_corners=False). Five-fold Grad-CAM is NOT implemented and shows "Unavailable". |
| Stored artifact | Raw float32 map at native resolution (8x8 occlusion cells or 16x16 Grad-CAM) + typed metadata JSON + SHA-256, in SQLCipher (schema v4). Colours and opacity are never stored. |
| Geometry | The Stage III tensor's own crop box, resize and padding are recorded; the inverse (`restore_to_grid`) samples the model-space map back to source pixels (OpenCV centre convention) and zeroes everything outside the retina mask (channel sum > 21, the upstream fundus tolerance) and outside the fundus crop. |

## Milestones
| M | Verdict | Evidence |
|---|---|---|
| M0 audit | DONE | Vault and both repos read; parent clean at `59d522f`, GUI clean at `7097319`; Supabase tree untouched. Reference images used only as layout inspiration (two are duplicates). |
| M1 typed API | DONE | `attribution.rs` (enums, provenance, binding, validation, 15 unit tests). |
| M2 Stage I Grad-CAM | NOT DONE (PARTIAL) | Stage I preprocessing (square crop with border padding, Ben Graham, CLAHE, unsharp) does not expose a registered geometry yet. Reported as Unavailable, never faked. |
| M3 Stage III Grad-CAM / ensemble | DONE for one fold; ensemble Grad-CAM not implemented | Score parity 0.925166 = ONNX; sanity checks below; five-fold uses occlusion. |
| M4 occlusion | DONE | Real forwards, progress, cancel between forwards, restore of input, error paths. |
| M5 geometry/render | DONE | Non-square inverse test, FOV zeroing, opacity/colormap/scale/signed views, shared zoom/pan. |
| M6 page | DONE | `AppPage::Explainability` after Timeline, scrollable sections, inspector, history, methods, evidence, Quad link. Screenshots 1280, 1024, 700. |
| M7 persistence/PDF | DONE | Schema v4 with backup and migration test; restore after restart; PDF section with the real overlay read by `pdftotext`/`pdftoppm`. |
| M8 benchmark | DONE (single image) | Deletion/insertion curves, parity, regression run below. One technical image: not a population result. |
| M9 notes | DONE | This file, vault notes. No Phase 5. |

## Measured results (CPU, `OD-1.png` 640x640, technical only)
| Run | Result |
|---|---|
| Occlusion fold 0, target Grade 3 | baseline 0.925166 = PyTorch reference 0.925166; 65 forwards; 65.3 s (about 1 s per forward). Peak RSS was not recorded for this run; the fold-0 deletion run peaked at 1.47 GiB. |
| Occlusion five-fold, target Grade 3 mean | baseline 0.762148 = PyTorch five-fold reference 0.762148; 325 forwards; 352 s heavy test, 322.6 s in the GUI; peak RSS 1.53 GiB. Cells -0.00738 .. +0.08200. Log: `evidence/occlusion-five-fold-heavy.log`. |
| Grad-CAM fold 0, severity 3 | worker score 0.925166 = ONNX 0.925166; 11.9-14 s; PyTorch process peak 5.1 GiB (not CPU-light). `evidence/gradcam-parity.log`. |
| Grad-CAM sanity | Randomised heads: map correlation vs real -0.13; randomised backbone +0.10 (maps depend on the trained weights). Severity 2 vs severity 3 signed maps correlate -1.0: with global average pooling and saturated probabilities the CAM is the same spatial pattern with opposite sign, so Grad-CAM target switching changes sign and magnitude, not the pattern. Gradability is saturated at 1.0000: its ReLU map is empty and the app would reject it as a constant map. `evidence/gradcam-sanity-checks.json`. |
| Deletion / insertion (fold 0, Grade 3, 64 cells) | Delete top 10% cells by measured drop: 0.925 -> 0.012 (random order mean 0.868). Top 20%: 0.0002 (random 0.848). Insert top 10% into a blank image: 0.850 (random 0.006). Single image, single fold, self-consistent with the occlusion ranking; not a general faithfulness claim. `evidence/faithfulness-deletion-insertion.log`. |
| Regression (T36) | Real 7-model cascade through the service, five-fold: masks identical to upstream references, ensemble vs PyTorch reference max absolute difference 2.98e-8, grade 3, 54.9 s, peak RSS 2.07 GiB. `evidence/regression-full-cascade-five-fold.log`. |
| GUI live | Full analysis then five-fold explanation: 325/325 forwards, saved, PDF exported, window closed and relaunched, explanation restored from the encrypted database with no model run. |
| Storage | 8x8 occlusion map = 256 bytes + JSON; database file is ciphertext (header is not `SQLite format 3`). |

## Tests
| Command | Result |
|---|---|
| backend `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `build --release`, `git diff --check` | pass |
| backend `cargo test --workspace` | 157 passed, 0 failed, 21 ignored (13 + 8 heavy, env-gated). New: 15 attribution, 7 attribution store (+migration), 1 PDF picture |
| backend heavy, run now | occlusion fold 0 and five-fold, deletion/insertion, Grad-CAM parity, full cascade regression: all pass (not part of the 21 skipped; run individually) |
| GUI same gates | pass; `cargo test` 188 passed, 0 failed, 2 ignored (new: 5 explain service, 8 view/render, 1 report, 1 service identity/stale, 1 display) |
| Not run | `local_*` Stage I/II single-model heavy tests, CUDA, Windows |

## Matrix (T01-T40)
| # | Status | Evidence |
|---|---|---|
| T01-T04 | VERIFIED | `attribution::tests::target_scores_follow_the_five_way_vector`; heavy parity (target equals PyTorch reference, mean not fold 0). |
| T05, T06 | VERIFIED | `validate_combination`, `build_stage3_occlusion_artifact` (exactly 1 or 5 distinct folds), service `invalid_jobs_are_refused...`. |
| T07 | PARTIAL | Fold identity/hash pinned by `verify_stage3_fold_model` (shared with grading) and `verify_models`; no dedicated mismatch test added. |
| T08-T11 | VERIFIED | `binding_rejects_foreign_results`, store `identity_is_enforced_on_save_and_load`, `display_requires_the_same_image...`. |
| T12-T14 | VERIFIED | `invalid_maps_are_rejected`. Negative-only maps are accepted but flagged (no positive support notice). |
| T15-T17 | VERIFIED, with the caveat above | Real worker: finite map, gradient reaches the layer, target switch changes score/sign (pattern identical under GAP). |
| T16 | VERIFIED | Layer documented in code, script, inspector and PDF. |
| T18-T20 | VERIFIED | `grid_is_8x8_with_65_forward_calls_and_exact_deltas`, `overlap_is_normalised...`, parity with the real model. |
| T21-T24 | VERIFIED | `cancel_stops_between_forwards...`, service `cancel_stops_a_running_job`, `second_submission_is_refused_while_busy`, `worker_panic_and_failure...`; live cancel screenshot 03. |
| T25, T26 | VERIFIED | `inverse_transform_for_non_square_crop`, `outside_fov_is_zeroed`, `heat_is_transparent_outside...`. |
| T27 | VERIFIED | `opacity_changes_display_only`. |
| T28 | VERIFIED (logic + live) | Shared transform test; three frames share zoom/pan (Ctrl+wheel zoom, drag pan). |
| T29 | VERIFIED | Screenshots 01, 06 (honest empty/unavailable states). |
| T30 | VERIFIED | Screenshots 04/08/09 (1280x820), 11/12 (1024x768), 13/14 (700x650). |
| T31, T32 | VERIFIED | `raw_map_survives_a_restart_exactly`, `corrupted_forged_and_stale_results_are_rejected`, service stale test; live restore after restart (05). |
| T33 | VERIFIED | Locked database: wrong password cannot open; page shows nothing (01); ciphertext check. |
| T34 | VERIFIED | `evidence/report-with-explainability.pdf` (3 pages, real overlay on page 3, hashes, parameters, notice); unit test `report_includes_explanation_only_when_supplied`. |
| T35 | VERIFIED | `supabase/`, `memory/Backend/supabase-architecture.md` untouched; no cloud call added; nothing uploaded. |
| T36 | VERIFIED (CPU) | Full cascade run above. |
| T37 | BLOCKED | GPU in PCI error state; no CUDA claim. |
| T38 | HONEST | No clinical metric is shown or claimed; the evidence panel says the curves are technical. |
| T39 | VERIFIED | No reference image is bundled or drawn; empty states say so. |
| T40 | VERIFIED | Git status below; vault links added. |

## Files
New: `native/crates/retinacare-ai-bridge/src/{attribution.rs,stage3_explain.rs}`, `native/crates/retinacare-persistence/src/attributions.rs`, `scripts/stage3_gradcam.py`, GUI `rust/src/{explain_app.rs,engine/explain.rs,ui/explain.rs}`.
Edited: ai-bridge `lib.rs`, `model_runtime.rs` (fold identity check moved into one shared function, behaviour unchanged); persistence `lib.rs` (schema v4, errors), `masks.rs` (migration test uses the constant); reports `lib.rs` (picture pages); GUI `main.rs` (page, nav, stale on re-run, link), `domain.rs`, `ui/mod.rs`, `ui/viewer.rs` (Quad overlay + link, wording), `engine/{mod.rs,patient.rs,report.rs}`.

## Security and privacy
- Maps live only in the SQLCipher file; locked database shows nothing; deleting the examination cascades.
- Identity (patient, examination, run, eye, image hash) is checked on save, on load and before display; a changed image file or a re-run analysis refuses the stored result (stale).
- The Grad-CAM worker is a local subprocess with fixed arguments (no shell), 0700 temp directory, bounded stdout, timeout and kill on cancel; the checkpoint is hash-verified and loaded with `weights_only=True`.
- No network, no Supabase use, nothing uploaded. Exports (PNG/JSON) go to a local file chosen by the user.

## Limits and decisions
- Occlusion maps are coarse (64 px cells); Grad-CAM is 16x16 upsampled. Neither localises pathology.
- Five-fold explanation takes about 5.4 minutes on this CPU; a single fold about 1 minute; Grad-CAM about 12 s but needs about 5.1 GiB RAM.
- Grad-CAM under global average pooling cannot distinguish targets by pattern (see sanity checks); occlusion is the more target-specific method here.
- Overlay and PDF use the selected colormap/opacity at export time; the raw map is unchanged.
- Faithfulness numbers come from one technical image and one fold.
