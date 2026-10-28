# P0.1 — HierarchiRetina model loading

This folder records the repository audit, technical evidence, implementation, tests and handoff for P0.1. The production path is Rust `retinacare-ai-bridge` → `retinacare-app`; the Python worker remains a separate local worker and is not the production inference path for this phase.

Current result: Rust ONNX loading, typed Stage I decoding, exact OpenCV image preprocessing, CPU inference, and nonblocking image-service dispatch work. Tensor and PyTorch/Rust ONNX parity measured exact on one locally approved demo image. P0.1 remains **PARTIAL**: formal licensed-image acceptance and CUDA/RTX 5070 execution are pending; later cascade stages are absent. This is technical parity only, not clinical validation.

Read in order: [audit](01-repository-audit.md), [research](02-technical-research.md), [requirements](03-requirements.md), [plan](04-implementation-plan.md), [inventory](05-model-inventory.md), [contracts](06-interface-contracts.md), [test strategy](07-testing-strategy.md), [results](08-test-results.md), [file changes](09-file-change-report.md), [risks](10-risks-and-limitations.md), [handoff](11-next-phase-handoff.md). Stage I conversion evidence is in [stage1-conversion.md](stage1-conversion.md).
