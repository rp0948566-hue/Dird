# Implementation plan and delivered design

1. Reuse existing `retinacare-ai-bridge`; avoid a second inference crate or Python production inference path.
2. Harden its manifest boundary: bounded JSON, safe local file path, regular-file and size checks, tensor contract, format and streamed hash.
3. Add a bounded local registry that reports invalid manifests and distinguishes ONNX from conversion-required PyTorch files.
4. Extend ONNX session loading with explicit CPU/CUDA requests, strict metadata checks, exact input-buffer validation and typed failures.
5. Add `ModelRuntime` for load/infer/unload and expose a bounded asynchronous `ModelService` in `retinacare-app` for callers that must not block the GUI thread.
6. Keep the synthetic ONNX fixture, add regression tests, run focused and workspace Rust checks, then record target-specific gaps.
7. Preserve the existing Stage I ONNX artifact and conversion script; do not fetch any weights.

The repository has implemented verified loading and typed output decoding. The app UI crate was not modified. Stage II/III adapters and clinical pipeline are separate follow-on scope.

## Stage I real-image plan and library decision

Pinned implementation: `ai_engine/.venv/hierarchiretina-src/hierarchiretina/stage1/preprocessing.py::preprocess_gate`, source commit `27f49f5ccc1d6297903dd651cef8a660eecc48ce`; tensor conversion in `stage1/data.py`; means/std in `stage1/config.py`.

Decision: use Rust's `opencv` bindings backed by system OpenCV, with only `imgcodecs` and `imgproc`. `image` is useful for bounded format/dimension preflight, but its documented resampling filters stop at Lanczos3 while the source uses Lanczos4 for enlargement. `imageproc` provides useful morphology/contour primitives but not the full OpenCV-compatible CLAHE, connected-component statistics, resizing, BGR/LAB conversion, and saturated weighted arithmetic pipeline. Approximate substitutions would fail parity. Rust remains the application implementation; OpenCV supplies source-equivalent image operators.

### Ordered implementation

1. Bound encoded bytes, preflight JPEG/PNG dimensions, and reject malformed, oversized, zero-dimension, or non-finite inputs.
2. Decode to uint8 BGR and implement ROI thresholding, close/open, largest 8-connected bounds, square crop/pad, resize selection, green CLAHE blend, Ben Graham circular mask, LAB L CLAHE, and unsharp mask exactly as upstream.
3. Convert BGR to RGB; normalize float32 as `(pixel / 255 - mean) / std`; emit contiguous NCHW `[1,3,768,768]`.
4. Include source-image SHA-256 and preprocessing version in typed results; submit inference through the existing bounded background service.
5. Compare uint8 preprocessing output with upstream OpenCV; require zero changed pixels for matching OpenCV version and parameters. Compare normalized float32 tensors at max absolute error `<= 1e-6`, tightening to exact equality if measured. Record actual errors and versions.
6. Compare PyTorch and Rust ONNX logits on an authorized image; require absolute logit error `<= 1e-3` and identical threshold routing, matching the current graph parity tolerance.

### Verified implementation state

The host exposes OpenCV 4.10.0, clang 22.1.8 and libclang. Pinned Rust
OpenCV bindings compile with `imgcodecs`, `imgproc`, and `clang-runtime`.
`preprocessing.rs` implements the full source transform and bounded JPEG/PNG
decode. A local-only Python/OpenCV 4.10 reference comparison on the
user-approved repository demo produced an exact float32 tensor match (max
absolute error 0, RMSE 0). Authentic PyTorch and Rust ONNX CPU logits also
matched exactly and produced the same typed threshold route. The same image
was run through the nonblocking Rust app service. Its provenance/license is
undocumented, so licensed-image validation remains PENDING and this is not
clinical validation. See `stage1-rust-preprocessing-and-inference.md`.

The complete Rust workspace and Python worker suites pass; strict Clippy and
the full release workspace build pass. CUDA and target GPU execution remain
unavailable in this environment.
