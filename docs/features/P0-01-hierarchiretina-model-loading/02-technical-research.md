# Technical research

## Upstream HierarchiRetina v1.0

The upstream project describes a three-stage cascade: Stage I ConvNeXt V2-L at 768×768 screens for DR; Stage II runs four lesion HSMoE-AUNet models plus SwinHRUNetPP vessel segmentation; Stage III LG-DRG grades using the image and five masks. See the [upstream README](https://github.com/asifuddin01/HierarchiRetina).

The [v1.0 release](https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0) lists 12 PyTorch files totalling about 10 GB and provides SHA-256 values. Upstream code is MIT; trained weights and released predictions are CC BY-NC 4.0. The weights are not treated as commercially reusable. No upstream ONNX artifact is identified in that release.

## Rust ONNX Runtime

The workspace pins `ort = 2.0.0-rc.15` with ONNX Runtime 1.31 APIs and CPU binaries. See [ort 2.0.0-rc.15](https://docs.rs/ort/2.0.0-rc.15/ort/) and [execution-provider docs](https://ort.pyke.io/perf/execution-providers).

CUDA is an opt-in Cargo feature. A CUDA request registers the CUDA EP with fail-on-registration behavior and disables CPU EP fallback; a successful session is reported as CUDA-configured. The compiled-provider probe alone does not prove a GPU or compatible driver is installed. ONNX Runtime documents build-specific CUDA/cuDNN compatibility: [CUDA EP compatibility](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html).

## Export policy

Python/PyTorch remains development-only for export and parity. Production Rust accepts `.onnx`; `.pt`/`.pth` files are identified and rejected with a conversion-required error. Export behavior is recorded in [stage1-conversion.md](stage1-conversion.md) and `tools/model_conversion/hierarchiretina/README.md`.
