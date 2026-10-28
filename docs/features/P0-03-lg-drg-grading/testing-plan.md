# Testing plan
- Unit: CORN forward/inverse round-trip against numpy output; consecutive vs count decode; threshold boundaries (`>` strict); ungradable priority; ensemble mean and fold ordering; absent-fold errors.
- Assembler: elementwise parity vs upstream tensor (max abs, RMSE), one-hot mask-channel tests, mask/frame mismatch, stale examination, non-finite and non-binary masks, black frame, tiny frame.
- Heavy (ignored, env-gated): ONNX parity per fold, Rust single-fold vs PyTorch, five-fold ensemble vs reference, five-model + Stage III cascade on the demo image.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --release --workspace`, `git diff --check`.
- CUDA: only with a real CUDA ONNX provider run on the RTX 5070; otherwise PENDING.
- Metrics (QWK, confusion matrix): only with licensed labeled data. None available now.
