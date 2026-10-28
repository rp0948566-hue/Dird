---
title: CUDA, hardware and packaging status
type: operations
domain: platforms
status: PARTIAL
verified_on: 2026-10-28
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
