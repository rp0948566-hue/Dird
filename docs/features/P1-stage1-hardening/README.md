# P1 Stage I screening hardening (2026-10-28)

Research prototype. Everything below is technical evidence, not clinical validation. Weights are CC BY-NC 4.0.
Status: engineering CPU path VERIFIED_COMPLETE for hashes, routing and failure paths; CUDA BLOCKED (GPU in error state); licensed-image acceptance BLOCKED (no authorized image set).

## M0 Hash and provenance (recomputed 2026-10-28, `sha256sum`)
| File | SHA-256 | Pinned value | Result |
|---|---|---|---|
| `models/stage1_convnextv2l_768/stage1.onnx` (807,176,466 B) | `9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac` | manifest `sha256` | match |
| `models/stage1_convnextv2l_768_best_auc_model.pth` (1,614,202,699 B) | `89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff` | manifest `validation.upstream_checkpoint_sha256` | match |
Upstream commit pinned in the manifest: `27f49f5ccc1d6297903dd651cef8a660eecc48ce`. Source: `native/crates/retinacare-ai-bridge/src/{stage1,preprocessing,model_runtime,onnx_session,model_manifest}.rs`; routing in `native/crates/retinacare-app/src/cascade.rs`.

## M1 Routing truth table (threshold 0.2391715943813324, `STAGE1_GATE_THRESHOLD`)
| Stage I outcome | Cascade state | Grade | Later stages |
|---|---|---|---|
| score < threshold (adjacent float below) | `NegativeRoute` | grade 0 from gate, provenance `Grade0FromStage1Gate` | none run |
| score == threshold | continue | decided by later stages | run |
| score > threshold | continue | decided by later stages | run |
| NaN / +-inf logit, wrong output count | `Failed` (`InvalidOutput`) | none | none |
| any `StageFailureKind` at Screening (9 kinds) | `Failed` | none, no screening record | none |
Tests: `stage1::tests::{route_flips_exactly_at_the_adjacent_float_boundary, logit_sweep_around_the_threshold_is_monotone_with_exactly_one_flip, extreme_and_signed_zero_logits_never_error_or_misroute, threshold_constant_is_the_pinned_upstream_value}`; `cascade::tests::{every_stage1_failure_kind_ends_failed_without_grade0_or_later_stages, stage1_score_at_threshold_continues_and_just_below_stops_with_no_later_stage}`. A 512-step logit sweep (adjacent f32) around logit(threshold) flips exactly once. Boundary errors: 0.

## M2 Licensed-image acceptance: BLOCKED
No authorized, licensed retinal image set exists locally. `dist/demo-images/*/OD-1.png` has unknown provenance and is a technical fixture only. No labels, no sensitivity/specificity. Needed from the user: images + license/consent + labels.

## M3 CUDA: BLOCKED (software stack proven, GPU unavailable)
- Hardware: RTX 5070 Laptop, 8151 MiB, driver 580.178.04 (nvidia-smi worked at 14:34 and listed the GPU).
- Build: `cargo ... --features cuda` (ORT 1.31 via `ort` 2.0.0-rc.15 download-binaries) compiled; `libonnxruntime_providers_cuda.so` needs CUDA 13 (`libcublasLt.so.13`, `libcudart.so.13`), not CUDA 12. First CUDA 12 attempt failed with that exact missing-library error: no CPU fallback happened (session is built with `with_disable_cpu_fallback` + `error_on_failure`).
- With the CUDA 13 libs on `LD_LIBRARY_PATH` the provider loaded and then failed honestly: `CUDA failure 100: no CUDA-capable device is detected`. At that time `nvidia-smi` printed "Unable to determine the device handle ... No devices were found" and `/sys/bus/pci/devices/0000:01:00.0/power/runtime_status` = `error`. A reboot or driver reload (root) is required; not done here.
- No GPU timing, parity or VRAM number exists. Do not claim CUDA works. Re-run once the GPU is back: `scripts/stage1_cuda_check.sh` (prints a `CUDA-PROFILE` line; asserts provider is `Cuda`, logit gap < 1e-2 vs CPU, same route).
- Approved downloads (all from PyPI, official NVIDIA wheels, kept outside git in `/home/claude/Desktop/retinacare-cuda/`): CUDA 12 set (cublas 12.9.2.10, cuda-runtime 12.9.79, cudnn-cu12 9.27.0.42, cufft 11.4.1.4, curand 10.3.10.19; unused, can be deleted) and CUDA 13 set (cublas 13.8.2.12, cuda-runtime 13.4.92, cudnn-cu13 9.27.0.42, cufft 12.4.0.43, curand 10.4.4.72); SHA-256 lists in `wheels.sha256` and `wheels13.sha256` there. Note: the approval named CUDA 12; the provider needs 13, same sources and purpose.

## M4 Performance (CPU, release, OD-1.png 640x640, `local_stage1_cold_and_warm_profile_is_deterministic`)
| Cycle | Load incl. SHA-256 verify (ms) | Inference total incl. preprocess, 3 runs (ms) | RSS after / HWM (MiB) |
|---|---|---|---|
| 1 cold | 5127 | 4890, 4475, 3742 | 1430 / 1969 |
| 2 | 3868 | 4454, 4158, 4545 | 1799 / 2338 |
| 3 | 2072 | 4354, 4635, 4307 | 1925 / 2464 |
Median inference 4454 ms (min 3742, max 4890, n=9); preprocessing 0.13-0.38 s of that. Logit bit-identical across all 9 runs. Unloading a session does not return RSS to baseline (cycle-2 start 1.18 GiB), so session reuse (load once, keep) is cheaper than reload; GUI already preloads Stage I. First-run speedup is load-time only; inference showed no warm speedup beyond noise.
Failure/edge tests (all automated, synthetic): truncated PNG/JPEG, BMP/TIFF/empty bytes, alpha/gray/16-bit PNG, 1x1/black/white frames, PNG header claiming 40000x40000 (rejected before decode), file above the encoded-size limit, corrupt ONNX bytes, manifest vs graph output-shape/name and input-shape/name mismatch, NaN input, wrong-size input, tampered/missing checkpoint.

## Finding: Stage I does not reject non-fundus images
Real model on synthetic technical images (never clinical): black frame logit 0.954 (score 0.722), uniform gray 0.954, orange disc 1.144 (0.758), random noise 1.460 (0.812). All route `ContinueCascade`. The gate has no out-of-distribution check, so a negative route can only come from a real fundus image the model scores low; none is available locally, so a real negative route has not been seen. A separate "is this a fundus" guard would be new functionality and is not implemented (backlog, needs a decision).

## Not done / not claimed
Licensed-image benchmark, real negative-route image, CUDA run, Windows, clinical accuracy. Real desktop GUI route tests beyond the earlier session (cancel, identity) were not repeated; scripted GUI route tests still pass (see regression below).
