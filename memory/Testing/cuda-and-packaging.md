---
title: CUDA, hardware and packaging status
type: operations
domain: platforms
status: PARTIAL
verified_on: 2026-10-29
source_refs: ["../../docs/migration-progress.md", "../../docs/windows-build.md", "../../docs/exe-verification.md"]
owner: Codex/Claude backend
tags: [CUDA, RTX-5070, Windows, packaging]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/cuda-and-packaging.md (sha256 fcb1cdde124a); verify current state in [[active-phase]] -->
# CUDA and packaging

CUDA/RTX 5070 execution is not verified. Prior status says nvidia-smi could
not communicate with the driver and no accessible NVIDIA device nodes were
available. Do not infer GPU support from a compiled provider or model metadata.
No GPU timings or VRAM measurements are known.

Project docs report a successful Ubuntu package and Windows GNU-target PE
cross-build. Windows has not been booted and tested; DLL loading, GUI rendering,
worker startup, SQLCipher behavior, clean shutdown and RTX detection remain
open. See [Windows checklist](../../docs/exe-verification.md),
[[Testing/build-and-test]], [[Decisions/risks-and-blockers]].

The user can verify on target hardware using existing scripts and record
stdout, logs, driver/CUDA versions, actual provider and per-stage timings.
Do not claim Windows MSVC signing, installer, CUDA performance or GPU inference
until measured.

## Update 2026-10-28
Every stage in the app reports provider CPU; CUDA was not run and nothing claims it. The runnable artifact is `GUI/Hackthon dezine/rust/target/release/retinacare` (60.9 MB; models and the database live outside it). Models are not bundled; packaging needs a model bundle + hash manifest, the CUDA check, and a Windows run. Idle GUI RSS ~1.4 GB, cascade peak ~2.95 GB (release).

## Update 2026-10-28 (P1-M3 CUDA attempt)
`--features cuda` builds (ORT 1.31 provider needs CUDA 13 + cuDNN 9; libs live outside git in `/home/claude/Desktop/retinacare-cuda/`, SHA-256 lists there). Provider loads, then fails honestly: `no CUDA-capable device`, because the GPU entered PCI runtime state `error` (nvidia-smi worked at 14:34, then "No devices were found"). CPU fallback is disabled, so no silent CPU run. Needs a reboot or driver reload by the user, then `scripts/stage1_cuda_check.sh`. No GPU timing, parity or VRAM data exists. Details: `docs/features/P1-stage1-hardening/README.md`.

## Update 2026-10-29 (GPU provider switch, VERIFIED on the RTX; supersedes the 2026-10-28 "not verified" notes)
CUDA now runs. Policy `RETINACARE_PROVIDER=cpu|auto|cuda` (`GpuPolicy` in `device.rs`; unset = auto when the `cuda` feature is compiled, else cpu). `ModelRuntime::load` is the single choke point; only tasks in `GPU_VERIFIED_TASKS` may run on CUDA: dr_screening_gate, microaneurysm / hemorrhage / hard_exudate / cotton_wool_spot / vessel segmentation. Stage III five-fold grading and explainability are NOT GPU-verified: in auto they run on CPU, under strict cuda they return `GpuNotVerified`. `load_unverified` pins explainability to CPU. Auto falls back to CPU when CUDA fails and logs `GPU unavailable for {what}: {reason}; using CPU`.
CPU vs GPU parity (final source, 6 installed models, all within 0.2% relative error): stage1 rel 0.00047 (CPU 3291 ms, GPU 136 ms); cws 0.00079 (4513/190); ex 0.00059 (4440/181); he 0.00055 (4469/176); ma 0.00100 (4761/177); vessel 0.00163 (1289/41).
Provider matrix (stage1 / stage3 / explain): cpu = cpu/cpu/cpu; auto+libs = cuda/cpu/cpu; cuda+libs = cuda/refused/refused; auto without libs = cpu/cpu/cpu with fallback log; cuda without libs = error/refused/refused.
CUDA 13 + cuDNN 9 libs are NOT bundled or committed; put them on `LD_LIBRARY_PATH` (here `/home/claude/cuda13/nvidia/{cu13,cudnn}/lib`). Without them auto silently-but-logged uses CPU.
Still open: Stage III GPU parity, explainability GPU parity, a real end-to-end retinal analysis on GPU, Windows. Do not claim the full GPU pipeline works until those pass.

## Update 2026-10-29 (demo verification, VERIFIED in the real release GUI)
Real run, RTX 5070, `RETINACARE_PROVIDER=auto` with CUDA libs on `LD_LIBRARY_PATH`: Stage I + five Stage II models report `CUDA device 0`, Stage III five-fold and Grad-CAM report CPU (saved report and PDF list the same). `OD-1.png` cascade 39.9 s (was 54-74 s CPU); negative-route image 3.5 s. Parity re-run on the final source: same six numbers (rel 0.00047 to 0.00163). Provider matrix passes with asserted expectations (`RETINACARE_EXPECT_STAGE1/STAGE3/EXPLAIN`): cpu cpu/cpu/cpu, auto+libs cuda/cpu/cpu, cuda+libs cuda/error/error, auto no libs cpu/cpu/cpu, cuda no libs error/error/error.
Parity test needs env: `RETINACARE_MODELS_DIR`, `RETINACARE_STAGE1_MANIFEST`, `RETINACARE_STAGE3_MANIFEST` (= `models/stage3_lgdrg/manifest_fold0.json`), plus the three `RETINACARE_EXPECT_*` for the mode test. The MODES print shows `explain=error` because stage3 is already loaded; the assertion reloads cleanly.
GUI label for a run is built from the stages' real providers (`chat_app.rs grounded_summary`), no hardcoded device text.
