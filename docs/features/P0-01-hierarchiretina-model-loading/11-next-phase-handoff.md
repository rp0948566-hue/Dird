# Next phase handoff

## State

Rust Stage I JPEG/PNG preprocessing, authentic CPU ONNX execution, threshold
decoding, and nonblocking app-service integration are implemented. OpenCV
preprocessing tensor parity and PyTorch/Rust ONNX output parity are exact on a
single locally approved demo image. P0.1 remains **PARTIAL** because formal
licensed-image acceptance and RTX 5070/CUDA execution have not passed; Stage
II/III are also absent.

## Remaining work

1. Obtain a non-identifiable retinal image with documented provenance and
   permitted use; run the same local parity test without publishing or
   committing patient imagery.
2. On the user's Ubuntu host with the RTX 5070 exposed, verify `nvidia-smi`,
   build the CUDA feature, load the actual model with an explicit CUDA request,
   run image inference, and record actual device/provider, timings and VRAM.
   Do not infer GPU success from provider compilation.
3. Keep P0.2 closed until the complete P0.1 acceptance gate is met.
4. Antigravity owns GUI work. Use `retinacare-app::models::ModelService`; do not
   edit `native/crates/retinacare-ui/` or `rust/src/`.

No clinical or diagnostic claim is supported by this technical parity test.
