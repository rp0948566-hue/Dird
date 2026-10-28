# Requirements and status

| Requirement | Status | Evidence |
| --- | --- | --- |
| Validate schema, typed tensor metadata, path, size and SHA-256 | Implemented | `model_manifest.rs` tests |
| Distinguish ONNX from PyTorch checkpoints | Implemented | format classification and conversion-required tests |
| Discover local model manifests with bounded scan | Implemented | `model_registry.rs` tests |
| Load ONNX in Rust and inspect input/output contract | Implemented | synthetic fixture inference test |
| CPU provider and optional explicit CUDA provider | Implemented; CPU tested, CUDA untested | `device.rs`, `onnx_session.rs` |
| Loaded model lifecycle and unload | Implemented | `model_runtime.rs` |
| Nonblocking app-facing operation queue | Implemented | `retinacare-app::models::ModelService` test |
| Authentic Stage I source checkpoint/parity | Synthetic numerical parity passed | Local checkpoint SHA verified; parity on 2 stored synthetic inputs, not clinical data |
| Production Rust image preprocessing/output decoding | Not implemented | Runtime currently accepts normalized float32 tensors and returns raw output |
| Stage II/III graph adapters and cascade | Not implemented | follow-on work |
| RTX 5070/CUDA performance | Blocked/unverified | `nvidia-smi` cannot communicate with driver; `/dev/nvidia*` absent |

Do not interpret loader readiness or model metadata as clinical validation. No diagnostic result, mask, heatmap or performance figure is created by this generic runtime layer.
