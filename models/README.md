# Model acquisition

No model weights are bundled. Store approved local checkpoints under this directory; `.pt`, `.pth`, `.onnx`, and `.gguf` files are ignored by Git. Do not download checkpoints automatically or use weights without a compatible license and task-specific validation.

Before enabling inference, add a manifest with model ID/version, SHA-256, architecture, task, input shape, preprocessing, output schema, label map, source URL, license, and validation evidence. The phase 1 worker can discover local PyTorch checkpoint files but deliberately does not deserialize or execute them until a trusted architecture/manifest contract exists.
