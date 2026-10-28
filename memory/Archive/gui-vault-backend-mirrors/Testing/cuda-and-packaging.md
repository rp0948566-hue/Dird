---
title: CUDA, hardware and packaging status
type: operations
domain: platforms
status: PARTIAL
verified_on: 2026-10-28
source_refs: ["../../../../docs/migration-progress.md", "../../../../docs/windows-build.md", "../../../../docs/exe-verification.md"]
owner: Codex/Claude backend
tags: [CUDA, RTX-5070, Windows, packaging]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Testing/cuda-and-packaging.md (sha256 9ecd9fc272f7); verify current state in [[active-phase]] -->
# CUDA and packaging

CUDA/RTX 5070 execution is not verified. Prior status says nvidia-smi could
not communicate with the driver and no accessible NVIDIA device nodes were
available. Do not infer GPU support from a compiled provider or model metadata.
No GPU timings or VRAM measurements are known.

Project docs report a successful Ubuntu package and Windows GNU-target PE
cross-build. Windows has not been booted and tested; DLL loading, GUI rendering,
worker startup, SQLCipher behavior, clean shutdown and RTX detection remain
open. See [Windows checklist](../../../../docs/exe-verification.md),
[[Archive/gui-vault-backend-mirrors/Testing/build-and-test]], [[Archive/gui-vault-backend-mirrors/Decisions/risks-and-blockers]].

The user can verify on target hardware using existing scripts and record
stdout, logs, driver/CUDA versions, actual provider and per-stage timings.
Do not claim Windows MSVC signing, installer, CUDA performance or GPU inference
until measured.
