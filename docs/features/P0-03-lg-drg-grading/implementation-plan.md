# Implementation plan (M1–M6)

## Rust placement (reuse, no new server)
- `retinacare-ai-bridge`: new `stage3_input.rs` (eight-channel assembler), `stage3_grading.rs` (CORN algebra, decode, ensemble), `Stage3*` contract types next to `stage2_contract.rs`; `model_runtime.rs` gets a Stage III branch (manifest validation, ONNX two-output session reuse via existing session adapter).
- `retinacare-app`: extend `ModelService` events with `Stage3Inference`; keep nonblocking and stale-request rules; later a cascade request (Stage I → II → III).
- Dev-only Python in `scripts/`: `stage3_export_parity.py`, `stage3_reference_fixture.py`.
- No GUI crate or GUI repo files.

## Milestones
- **M0** (done): research docs, inventory, this plan.
- **M1**: one approved fold → size/SHA check → safe load (`weights_only` needs checking; EMA key) → ONNX export with both outputs → PyTorch/ONNX parity (logits, probabilities, decisions) → manifest → Rust session loads it.
- **M2** (no checkpoint required): Rust eight-channel assembler vs upstream reference tensor on the demo image with the five existing Stage II masks (fixtures in `/tmp/retinacare-models`). Reject missing, stale, mis-sized, non-finite masks and wrong order. Channel-order one-hot tests.
- **M3**: Rust single-fold inference; CORN algebra and consecutive decode with the pinned thresholds; typed result labelled SINGLE-FOLD TECHNICAL TEST.
- **M4**: remaining four folds, one approval each; mean of 5-way vectors; sequential load-run-unload; measure memory; compare with upstream ensemble.
- **M5**: cascade in `ModelService` (Stage I gate τ=0.2391716, then Stage II masks, then Stage III), cancellation, stale-ID rejection, missing-model states, typed result for the later GUI.
- **M6**: gates, heavy tests, docs, vault, file-change report. Labeled metrics only with authorized labeled data.

## Work that does not need a download (can start now)
1. M2 assembler and its parity fixture generator.
2. CORN algebra, decode and ensemble in Rust with Python-generated vectors.
3. Stage III contract types and error enums.

## Approvals requested
- **One** checkpoint now: fold 0 (795,798,428 B, SHA `c1593eac…f57c`), needed for M1/M3. Folds 1–4 later, one at a time.

## Risks
- ONNX export of the cross-attention head, shape specialisation to 512×512, and BatchNorm folding.
- Albumentations rounding and CLAHE parity (verified quirks listed in [upstream-research](upstream-research.md)).
- Upstream silently zero-fills missing masks; RetinaCare rejects.
- Demo image licence is undocumented: technical tests only. Mask influence on severity is weak per the paper's ablation; make no causal claim.
