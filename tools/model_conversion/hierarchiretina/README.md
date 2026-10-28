# HierarchiRetina development conversion

Use the existing [Stage I export/parity script](../../../scripts/stage1_export_parity.py); this folder intentionally contains no second exporter. It is development-only and is not imported by the shipped Rust application.

The script requires the original Stage I checkpoint, the exact upstream source revision, and a development environment with PyTorch, timm, OpenCV, ONNX and Python ONNX Runtime. It verifies the upstream checkpoint SHA before export, checks the ONNX graph, compares PyTorch and ONNX Runtime on identical inputs, and writes the local manifest and golden file. The checkpoint is not present in this checkout; do not download the roughly 10 GB release without explicit approval.

Run from the repository root with `ai_engine/.venv/bin/python scripts/stage1_export_parity.py`. See [conversion evidence](../../../docs/features/P0-01-hierarchiretina-model-loading/stage1-conversion.md) and the [upstream v1.0 license and checksums](https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0).
