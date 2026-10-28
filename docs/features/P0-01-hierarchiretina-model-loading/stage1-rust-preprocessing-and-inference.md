# Stage I Rust preprocessing and inference

## Implementation

Rust OpenCV image decode and the upstream Stage I transform live in
`native/crates/retinacare-ai-bridge/src/preprocessing.rs`. Input is limited to
JPEG/PNG, 64 MiB encoded size, and 40 million pixels. Decode produces uint8
BGR. Preprocessing selects the largest 8-connected retina component after
threshold and 5×5 close/open, applies the 8-pixel margin and centered square
crop/pad, then emits 768×768 using INTER_AREA for shrinking or INTER_LANCZOS4
for enlargement. It follows upstream green-channel CLAHE (clip 3, 8×8, 0.75 /
0.25 blend), Ben Graham (sigma max(10, trunc(0.045 × height)), 4/-4/128
weighted blend, filled radius-0.49 FOV mask), LAB-L CLAHE (clip 2.2, 8×8),
1.3/-0.3 unsharp mask (sigma 1.2), BGR→RGB, float32 ImageNet normalization,
and contiguous NCHW `[1,3,768,768]` output.

Image preprocessing is attached to `ModelRuntime::infer_stage1_image` and the
bounded background `ModelService::infer_stage1_image` command. Results retain
source SHA-256, original dimensions, preprocessing version, Stage I typed gate
output, actual execution provider, and elapsed/preprocessing timings. Errors
remain typed through the runtime and are returned asynchronously by the app
service. No Python production inference path was added.

Rust uses pinned `opencv = 0.101.1` with only `imgcodecs`, `imgproc`, and
`clang-runtime`; system OpenCV observed here is 4.10.0. The `image` crate is
used only to preflight format, dimensions and encoded-size limits. `image`'s
Lanczos3 and `imageproc`'s partial operators cannot reproduce this pipeline's
Lanczos4, CLAHE, connected-component behavior and OpenCV arithmetic closely
enough. The Python/OpenCV implementation remains a development oracle.

## Build prerequisites

Ubuntu packages, from configured distribution repositories:

```bash
sudo apt-get update
sudo apt-get install --no-install-recommends build-essential pkg-config clang libclang-dev libopencv-dev
pkg-config --modversion opencv4
pkg-config --cflags --libs opencv4
clang --version
ldconfig -p | grep libclang
```

The verified host reported OpenCV `4.10.0`, Ubuntu clang `22.1.8`, and
`/usr/lib/x86_64-linux-gnu/libclang-22.so`. Cargo fetched the pinned binding
and built it successfully in this environment. On hosts with restricted Cargo
networking, cache dependencies through the organization's approved Cargo
registry; do not replace required OpenCV operators with approximations.

## Numerical parity evidence

The reference uses the pinned upstream source checkout
`27f49f5ccc1d6297903dd651cef8a660eecc48ce`, Python OpenCV `4.10.0`, and the
locally present checkpoint whose SHA-256 is
`89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff`.
`scripts/stage1_preprocess_reference.py` writes only to caller-selected local
temporary paths; it is not used in production.

For the user-approved, local-only repository demo input
`public/demo-images/session-1-final/OD-1.png` (640×640; SHA-256
`32d756fc923262daa5296c6852c1569e01394b37db57e72385785fca9017c899`), Rust
and Python preprocessing yielded identical float32 tensors: max absolute
difference `0.000e0`, RMSE `0.000e0`, shape `[1,3,768,768]`. The same image's
authentic PyTorch and Rust ONNX CPU logits differed by `0.000e0`; both decoded
to `ContinueCascade` at the source threshold `0.2391715943813324`. This is
reproducible technical parity on one local example only. It is not clinical
validation, model accuracy, or evidence of a diagnosis.

The demo image was used only for temporary local testing under the user's
explicit instruction. It was not copied into a fixture, generated artifact,
or distribution. Its source dataset and usage license remain undocumented;
formal licensed-image validation is **PENDING**. The ONNX/checkpoint license
also remains subject to the existing CC BY-NC 4.0 limitations documented in
the model inventory.

## Current status and limits

- CPU Stage I JPEG/PNG → Rust preprocessing → authentic ONNX inference → typed
  threshold route works, including through the nonblocking Rust app service.
- Rust image tests cover valid PNG, corrupt bytes, encoded-size limit,
  invalid dimensions and the pixel-count limit. Local test also verifies
  tensor/logit parity, source hash, shape, and route.
- Formal image provenance/license acceptance is **PENDING**.
- RTX 5070/CUDA execution is **NOT VERIFIED**. No GPU claim is made. On the
  target Ubuntu host, place an authorized test image at a local path, then run:

  ```bash
  nvidia-smi
  cd native
  RETINACARE_STAGE1_PROVIDER=cuda RETINACARE_STAGE1_IMAGE=/absolute/path/to/authorized-fundus.png cargo test -p retinacare-app --features retinacare-ai-bridge/cuda -- --ignored local_stage1_image_runs_through_the_nonblocking_model_service --nocapture
  ```

  The test requests CUDA device 0, fails on provider/session errors, asserts
  the actual returned provider, and exercises image inference through the
  nonblocking service. Record `nvidia-smi`, returned provider and measured
  timings. A compiled CUDA provider alone does not verify GPU execution.
- Stage II/III cascade inference, clinical validation, and diagnostic
  suitability are out of scope and remain incomplete. P0.2 remains closed.
