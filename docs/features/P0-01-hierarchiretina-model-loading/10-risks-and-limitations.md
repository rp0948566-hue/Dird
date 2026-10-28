# Risks and limitations

- The one local demo image was used only for temporary technical parity under
  the user's instruction. Source dataset and usage license are undocumented;
  formal licensed-image validation remains PENDING. No image was added to
  source fixtures or distribution.
- Stage I CPU image preprocessing, authentic ONNX inference, and typed gate
  decoding match the pinned upstream reference for the one tested image. This
  is numerical implementation evidence, not clinical validation, calibration,
  diagnostic accuracy, or suitability evidence.
- CUDA is opt-in and not exercised. The environment has no exposed NVIDIA
  device; RTX 5070 execution, driver/cuDNN compatibility, VRAM use and
  performance remain unverified.
- Stage II/III adapters and the grader's image-plus-five-mask contract are not
  implemented. P0.2 remains closed.
- The ONNX artifact and upstream trained weights are CC BY-NC 4.0; deployment
  and redistribution require separate license review.
- Runtime validates image bytes, format, dimensions and pixel limits before
  allocation/decode. Callers must still protect patient-image paths and logs;
  only SHA-256 and image metadata are attached to typed inference output.
- Model manifest hashing does not prevent a privileged/concurrent process from
  replacing artifacts after verification. Keep model directories user-owned
  and avoid shared writable locations.
