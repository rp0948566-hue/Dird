# Stage II test plan

Per model: upstream checkpoint size/hash, converted graph hash/checker,
PyTorch/ORT and PyTorch/Rust full-map parity, threshold boundary, finite/range,
mask shape, deterministic replay, coordinate mapping, serialization and
missing/corrupt/mismatched artifact errors.

Preprocessing/geometry: nonclinical synthetic images for the MA/HE circle,
EX/CWS rectangle and letterbox, and vessel FOV crop; source-to-model-to-source
landmark registration; black frame/empty FOV behavior; RGB order; threshold and
postprocessing pixel fixtures. Synthetic fixtures are technical only.

Runtime: bounded async service, queue-full response, explicit unload/reload,
wrong manifest task/shape, CPU provider, explicit unavailable CUDA, canceled
or stale examination response, invalid/oversized image, no fabricated output
on failure. Add OOM/provider tests only when target provider is available.

Integration: opt-in, documented licensed image test per model, with original
image SHA and no committed image/mask. Rerun Stage I tensor/inference regression
plus full `cargo fmt --all --check`, strict workspace Clippy, workspace tests,
release build and `git diff --check`.

Do not count a Python-only result, dummy ONNX, schematic, or local demo as a
trained model inference or licensed-image acceptance.
