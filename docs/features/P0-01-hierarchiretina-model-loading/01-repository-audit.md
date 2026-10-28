# Repository audit

## Scope and boundaries

- Legacy DIRD+ remains in `src/` and `src-tauri/`; this phase changes only the native Rust backend, model metadata, conversion helper and related docs.
- The native Rust workspace is `native/`. Reused backend boundary is `retinacare-ai-bridge`; application exposure belongs in `retinacare-app`.
- `native/crates/retinacare-ui/` is owned by Antigravity and must remain untouched. Python worker remains available for its existing health/IPC duties.
- Model files are local, excluded from Git, and must not be downloaded without explicit approval.

## Baseline found

- `retinacare-ai-bridge` already had `WorkerClient`, a manifest SHA-256 gate and a CPU-only ONNX fixture session.
- The manifest gate streamed hashes but allowed unsafe checkpoint paths, did not bind expected file size, and did not distinguish PyTorch from ONNX artifacts.
- ONNX session checked only input tensor type/shape; output contract and exact input buffer length were not checked. No model registry or loaded-session lifecycle existed.
- `retinacare-app` had asynchronous Python worker supervision but no Rust model-facing application API.
- A local Stage I ONNX file and manifest are present. The ONNX bytes are 807,176,466 bytes; SHA-256 is `9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac`.
- The raw Stage I `.pth` checkpoint is not present at its conversion-script input path. No 10 GB release download was initiated.

## Classification

| Feature | Baseline |
| --- | --- |
| Persistent Python JSONL worker | Implemented; not used for this Rust inference path |
| Manifest parsing and hash check | Implemented, needed trust-boundary hardening |
| Rust ONNX CPU loading and synthetic inference | Implemented and previously tested |
| Local Stage I ONNX artifact | Present; manifest hash matches; Rust golden test must be rerun |
| PyTorch conversion/parity | Script exists; cannot rerun without original `.pth` |
| Registry, CUDA selection, session lifecycle | Missing at baseline |
| CUDA / RTX 5070 execution | Not verified |
