# P0.1 test results

Run date: 2026-10-28, Ubuntu host. Commands run from `native/` unless noted.

## Passed

- `pkg-config --modversion opencv4` → `4.10.0`; clang `22.1.8`; libclang
  resolves to `/usr/lib/x86_64-linux-gnu/libclang-22.so`.
- `cargo fmt --all --check` — pass after formatting the service regression test.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo build --release --workspace` — pass after OpenCV and app-service provider-test changes.
- `cargo test --workspace` — 53 passed, 0 failed, 3 ignored. Covers decoder,
  image-input validation, corrupted/oversized inputs, synthetic ONNX, app
  service errors, persistence recovery, and worker IPC.
- `cargo test -p retinacare-ai-bridge -- --ignored
  local_fundus_matches_python_tensor_and_typed_onnx_result --nocapture` — 1
  passed using the user's local-only approved demo image, Python reference
  tensor and local authentic Stage I ONNX artifact. Tensor max absolute error
  `0.000e0`, RMSE `0.000e0`; PyTorch/Rust ONNX logit absolute error `0.000e0`;
  typed route matched (`ContinueCascade`).
- `cargo test -p retinacare-app -- --ignored
  local_stage1_image_runs_through_the_nonblocking_model_service --nocapture` —
  1 passed. Loaded the authentic Stage I model on CPU, queued image inference
  through `ModelService`, and checked image SHA/dimensions, preprocessing
  version, timing fields, typed result, and returned CPU provider. The same
  test accepts `RETINACARE_STAGE1_PROVIDER=cuda` for target RTX verification.
- `../ai_engine/.venv/bin/python -m unittest discover -s ../ai_engine/tests`
  — 5 passed.
- Python preprocessing reference had already generated a local-only tensor and
  metadata with Python OpenCV `4.10.0` and PyTorch `2.14.1+cpu`; temporary
  outputs were kept under `/tmp` and removed after verification.

Reference image facts: 640×640, SHA-256
`32d756fc923262daa5296c6852c1569e01394b37db57e72385785fca9017c899`;
checkpoint SHA-256
`89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff`;
threshold `0.2391715943813324`. This is temporary technical parity evidence
only. The image's source/license is undocumented; formal licensed-image
acceptance is PENDING. No clinical inference is claimed.

## Not verified

- CUDA feature/provider execution, NVIDIA RTX 5070 detection, VRAM use, or
  inference performance. This environment has no exposed NVIDIA device; prior
  `nvidia-smi` could not communicate with the driver and `/dev/nvidia*` was
  absent.
- Windows build/runtime, GUI interaction, licensed-dataset validation,
  clinical performance, diagnosis quality, Stage II/III cascade, or P0.2.

P0.1 remains **PARTIAL**. CPU image-to-Stage-I technical pipeline is verified;
formal licensed-image and target CUDA gates are open. Keep P0.2 closed.
