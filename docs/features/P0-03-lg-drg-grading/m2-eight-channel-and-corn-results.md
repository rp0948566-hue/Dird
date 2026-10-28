# M2 eight-channel assembler and CORN algebra — results (2026-10-28)

Status: **technical, no checkpoint needed.** Not an end-to-end grade. Demo image provenance is undocumented, so this is local technical verification only.

## Delivered (Rust, `retinacare-ai-bridge`)
- `stage3_input.rs`: `assemble_stage3_input` (typed Stage II results) and `assemble_stage3_input_raw`. Port of upstream `crop_fundus_circle` (channel mean > 7, same box for masks), `clahe_lab` (clip 2.5, 8×8), `LongestMaxSize(512)` with Python rounding, zero centred padding, ImageNet normalisation of RGB only, masks `> 127` → {0,1} with nearest resize.
  - Rejects: missing, duplicate, incomplete, stale-examination, other-image (SHA), wrong-size, non-0/255 masks. Upstream would silently zero-fill; RetinaCare does not.
  - Channels are placed by mask kind in the order MA, HE, EX, CWS, VESSEL, independent of input order.
- `stage3_grading.rs`: `full_vector` (upstream `lgdrg_predict`), `ensemble_mean`, `corn_from_full`, `decode_consecutive`, `decode_full`; thresholds t = (0.38, 0.60, 0.19), gradability 0.10, Stage I gate 0.2391716 as constants sourced from the pinned release.
- `stage2_preprocessing.rs`: `clahe_lab` shared helper (also used by the vessel path). No behaviour change for Stage II.
- Fixtures/scripts: `scripts/stage3_reference_fixture.py` (upstream `load_image_and_masks`, `build_aug(train=False)`, `lgdrg_predict`, `decode_consecutive`), small synthetic `fixtures/stage3_corn_vectors.json` (48 single-fold, 24 five-fold cases; no patient data).

## Verification
| Check | Result |
|---|---|
| Eight-channel tensor vs upstream on `OD-1.png` with the five Rust-verified Stage II masks (heavy, ignored test) | all 8 channels max abs 0, RMSE 0 |
| CORN vectors, probabilities and grades vs upstream (48 cases) | pass (vector tol 2e-6, p tol 1e-9) |
| Five-fold mean and decision vs upstream (24 cases) and fold-order invariance | pass |
| Decode boundaries (strict `>`, first-failure stop, ungradable priority) | pass |
| Channel one-hot, nonbinary and size rejection, typed-result ordering/missing/duplicate/stale/source/size | pass |
| `cargo fmt --all --check`, strict Clippy, release build, `git diff --check` | pass |
| `cargo test --workspace` | 76 passed, 0 failed, 15 ignored |

## Not done
- No Stage III checkpoint, ONNX graph or Rust inference (M1, M3). No grade is produced.
- No ensemble parity with real fold outputs (M4). No cascade wiring (M5).
- No labeled evaluation (no QWK or confusion matrix). No CUDA run.
