# Model integration contract

## Production model path

Native inference is Rust `retinacare-ai-bridge` ONNX Runtime, exposed asynchronously through `retinacare-app::models::ModelService`. The existing Python worker continues to serve its health/IPC duties; it does not deserialize production weights or supply diagnoses. See [P0.1 contracts](features/P0-01-hierarchiretina-model-loading/06-interface-contracts.md).

Stage I accepts bounded JPEG/PNG paths through `ModelRuntime::infer_stage1_image` or the nonblocking `ModelService::infer_stage1_image`. Rust OpenCV performs pinned upstream preprocessing and returns a typed gate result with source image SHA-256, original dimensions, preprocessing version and timing. CPU end-to-end parity was measured against the pinned Python/OpenCV + PyTorch reference on one user-approved local demo image. Its dataset/license is undocumented; formal licensed-image validation remains PENDING. See [Stage I implementation evidence](features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md).

## Checkpoint policy

No checkpoint is downloaded automatically. A local manifest must provide model identity, source, license, file name, exact byte size, SHA-256, typed fixed input/output contracts and validation evidence. Only `.onnx` can reach ONNX Runtime; `.pt`/`.pth` are marked conversion-required and never deserialized. Loading rechecks the hash and graph metadata. The Stage I ONNX artifact is local, excluded from Git and licensed CC BY-NC 4.0; it is not evidence of clinical validation.

CPU is the default. CUDA is opt-in and fails explicitly if the provider is not compiled or registration fails. CUDA capability reporting alone does not prove hardware or compatible libraries; target execution must be tested separately.

## Clinical boundary

Image preprocessing and worker health are not diagnoses. Do not return a DR grade, lesion map, segmentation, or heatmap without a verified model for that task.
