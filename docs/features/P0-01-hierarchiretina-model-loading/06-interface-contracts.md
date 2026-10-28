# Model interface contracts

## Manifest v1

The manifest names a single local artifact. `checkpoint` must be one filename, not a path. `size_bytes` and `sha256` bind the artifact. Input and output tensor names, float32 types and fixed shapes must agree with ONNX graph metadata. `validation` must link evidence; a valid manifest alone is not clinical validation.

```json
{
  "schema_version": 1,
  "model_id": "example-model",
  "version": "1.0.0",
  "task": "synthetic_tensor_check",
  "architecture": "documented-architecture",
  "checkpoint": "model.onnx",
  "size_bytes": 186,
  "sha256": "<64 lowercase hexadecimal characters>",
  "license": "reviewed-license-id",
  "source_url": "https://example.org/source",
  "input": {
    "name": "image",
    "data_type": "float32",
    "shape": [1, 3, 4, 4],
    "color_space": "RGB",
    "normalization": "documented normalization"
  },
  "outputs": {
    "schema": "tensor",
    "name": "result",
    "data_type": "float32",
    "shape": [1, 3, 4, 4]
  },
  "validation": {"report": "path-or-url-to-evidence"}
}
```

## Rust backend API

- `model_manifest::inspect_manifest` validates metadata and local file shape/size without hashing; `verify_manifest` adds streamed SHA-256 verification.
- `model_registry::ModelRegistry::discover` returns bounded catalog entries. ONNX entries are `AwaitingIntegrityVerification`; `.pt`/`.pth` entries are `ConversionRequired`.
- `onnx_session::VerifiedSession::open_with_provider` verifies before session creation; it reports typed errors for conversion-required inputs, metadata mismatch, invalid input length, unavailable CUDA build and runtime errors.
- `model_runtime::ModelRuntime` owns loaded sessions and exposes load/infer/unload.
- `retinacare_app::models::ModelService` submits catalog/load/infer/unload commands to a bounded background queue and returns events via nonblocking polling. Queue-full is explicit.
- `ModelRuntime::infer_stage1_gate` and `ModelService::infer_stage1` provide a typed Stage I single-logit result with source threshold, route, verified artifact identity, actual provider, and elapsed time. The input remains a caller-supplied float32 tensor; preprocessing parity is not established.

Current session contract supports one fixed-shape float32 input and one fixed-shape float32 output. Multi-input LG-DRG and mask outputs require a later explicit schema and adapter; they are not silently coerced into this API.
