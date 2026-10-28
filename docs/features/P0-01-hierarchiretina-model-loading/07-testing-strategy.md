# Testing strategy

## Automated

- Manifest: valid metadata, missing/malformed fields, unsupported schema, bad hash, missing file, exact size, traversal, symlink and unsupported extension.
- Registry: empty directories, ONNX readiness state and PyTorch conversion-required state; bounded traversal.
- Runtime: valid synthetic ONNX graph loads on CPU, input/output metadata match, numeric `x * 2` inference, wrong input length rejected, invalid graph fails, PyTorch is never passed to ONNX Runtime, missing loaded ID is typed, unload drops session.
- App: catalog discovery returns asynchronously through the application event interface.
- Stage I decoder: one finite logit, stable sigmoid extremes, inclusive threshold boundary, invalid output rejection, serialization; service reports unloaded-model errors asynchronously.
- Image preprocessing: bounded JPEG/PNG format and dimensions before decode, valid PNG→NCHW tensor, corrupt bytes, encoded-size cap, invalid dimensions and decoded pixel cap. The opt-in local test compares a Python/OpenCV float32 golden tensor, authentic PyTorch and Rust ONNX logits, source SHA, preprocessing version, and route.
- App service: opt-in local image test loads the authentic Stage I manifest, queues image inference through the bounded background thread, and checks typed result/provenance/timing without GUI changes.
- CUDA hardware run on target Ubuntu: `RETINACARE_STAGE1_PROVIDER=cuda RETINACARE_STAGE1_IMAGE=/absolute/path/to/authorized-fundus.png cargo test -p retinacare-app --features retinacare-ai-bridge/cuda -- --ignored local_stage1_image_runs_through_the_nonblocking_model_service --nocapture`; the test asserts returned `ActualProvider::Cuda` and fails on provider/session errors.
- Session trust boundary: rejects NaN/Inf tensor elements before ONNX Runtime.
- Authentic gate: opt-in test compares Rust ONNX Runtime output to stored Python-generated golden cases. It requires the local Stage I artifact and fixture metadata; it does not run PyTorch.

## Commands

```bash
cargo fmt --all --check --manifest-path native/Cargo.toml
cargo clippy --workspace --all-targets --manifest-path native/Cargo.toml -- -D warnings
cargo test --workspace --manifest-path native/Cargo.toml
cargo build --release --workspace --manifest-path native/Cargo.toml
cargo test -p retinacare-ai-bridge --manifest-path native/Cargo.toml -- --ignored
```

The optional CUDA feature should be compiled/tested only in a supported CUDA build environment; a successful provider compile is not a GPU execution test. Record actual RTX 5070 tests separately.
