# Migration progress

## Verified on Ubuntu (updated 2026-10-28)

- Legacy `src/` and `src-tauri/` remain present.
- Rust workspace is split into UI, app, domain, persistence, reporting, and AI bridge crates.
- Native eframe entry point has seven screens, bounded background image loading, zoom/pan, patient/examination operations, worker/device state, and explicit missing-model errors.
- Persistent Python JSONL worker, versioned IPC, bounded messages, timeout cancellation, reaping, logging, health/device probe, model discovery, preprocessing validation, and timing fields are implemented.
- SQLCipher patient/examination round trips, wrong-password handling, failure-safe rekey salt recovery, and legacy atomic `.dird` import regression coverage exist.
- Ubuntu release and self-contained portable package build. Packaged headless worker/database/icon smoke passes.
- Actual x86-64 Windows GUI PE cross-build and portable package creation pass. Windows execution has not been tested.
- P0.1: Rust manifest gate, bounded model registry, CPU ONNX Runtime session, opt-in CUDA provider path,
  loaded-session lifecycle, and nonblocking app service exist. Rust OpenCV now performs bounded JPEG/PNG
  decode and exact Stage I transform; it is connected to authentic CPU ONNX inference and typed gate
  decoding through the app service. On one user-approved, local-only demo image, Rust/Python float32
  preprocessing tensors and PyTorch/Rust ONNX logits matched exactly; direct bridge and app-service
  image-to-result tests passed. Image provenance/license is undocumented, so licensed-image acceptance
  remains PENDING. CUDA/RTX 5070 is unavailable here and Stages II/III remain absent. No diagnosis or
  clinical validation is claimed. See the P0.1 feature documentation.

## Latest gates

- Python worker: 5 tests passed on 2026-10-28.
- Rust workspace latest: 53 tests passed, 3 ignored; formatting and Clippy `-D warnings` pass. The local-only image parity and app-service inference tests each passed separately. `cargo build --release --workspace` passed with OpenCV integration.
- Ubuntu release: passed.
- Windows GNU release cross-build: passed; PE GUI subsystem and `.rsrc` verified.
- RustSec: no vulnerabilities; three unmaintained eframe/accesskit Linux transitive warnings are tracked in `code-quality-audit.md`.
- Ubuntu virtual display: Xvfb cannot bind because `/tmp/.X11-unix` is owned by `nobody:nogroup`; fixing ownership requires administrator authentication. Direct active-Wayland launch reached a live process, but automated GUI report did not complete before the 25-second bound. GUI behavior therefore remains only partially verified.
- Legacy Tauri Rust tests remain blocked by missing `gdk-3.0 >= 3.22` development files.

## Not verified

- Native Windows launch, DLL loading, navigation, SQLCipher behavior, or clean worker exit on Windows.
- RTX 5070/CUDA detection, VRAM telemetry, or performance.
- Clinical validation, heatmaps, lesion masks, accuracy, or performance benchmark. One technical CPU model run on a repository demo image is documented but is not clinical evidence.
- Windows MSVC build/signing/installer; current deliverable is an unsigned portable GNU-target package.

## Next milestone

Run `test-windows.ps1` after reboot, archive its reports/log, then integrate one licensed checksum-pinned model adapter and validate CUDA on the RTX 5070.

For P0.1: compile/run CUDA on a host exposing the RTX 5070 and obtain licensed-image validation evidence.
Do not start P0.2 until its acceptance gate is fully verified.
