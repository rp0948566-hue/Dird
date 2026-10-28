---
title: Active phase and verified project state
type: status
domain: project
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../../docs/features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md"
  - "../../../docs/features/P0-02-stage2-segmentation/07-test-results.md"
  - "../../../models/stage2_hsmoe_aunet"
  - "../../../GUI/Hackthon dezine/docs/features/GUI-P05-clinical-report-export/09-test-results.md"
owner: Codex backend; Antigravity GUI
tags: [status, handoff]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/active-phase.md (sha256 d51b3307623f); verify current state in [[active-phase]] -->
# Active phase — evidence snapshot (updated 2026-10-28)

Canonical backend vault: this folder. Backend Git repo: `/home/claude/Desktop/Dird`,
branch `main`, HEAD `8fb2fee` (Stage II five models in Rust) on top of `9f13b5f`,
local commits 2 ahead of `origin/main` (`7286cf7`, the previous memory reference, is
its parent). Not pushed. Working tree clean at check time. GUI repo:
`/home/claude/Desktop/Hackthon dezine`, branch `dezine`, HEAD `f23120f` (memory
previously said `b1cd4b7`), 21 changed entries. GUI notes stay with Antigravity in
`Hackthon dezine/memory`.

## Current state

- **P0.1 Stage I — PARTIAL.** `Dird/models/stage1_convnextv2l_768/stage1.onnx` (807,176,466 bytes)
  and `Dird/models/stage1_convnextv2l_768_best_auc_model.pth` present (existence verified
  2026-10-28; hashes REPORTED). Demo-image CPU parity REPORTED. CUDA and licensed-image gates PENDING.
- **P0.2 Stage II — PARTIAL (5 of 5 models in Rust, technical only).** MA, HE, EX, CWS and VESSEL run through Rust ONNX on CPU and match upstream-derived reference masks on the demo image, individually and in one sequential five-mask ModelService run. Evidence: `Dird/docs/features/P0-02-stage2-segmentation/` files 11, 13, 14, 15. Licensed-image acceptance PENDING. CUDA PENDING. Mask persistence (artifact manifest, atomic writes) not built. Stage II is NOT complete. See [[Models/stage-ii-segmentation]].
- **Stage III — PARTIAL (fold 0 only; M5A cascade done).** Fold 0 grades through Rust ONNX (logits within 5.7e-6 of PyTorch). The nonblocking `ModelService` cascade Stage I → five Stage II masks → fold 0 ran for real on the demo image: Stage I continued, all five masks equal the upstream references, technical grade 3, CPU, peak RSS 2.2 GB, 232 s total (model loading dominates). SINGLE-FOLD TECHNICAL TEST, research only. Folds 1-4, five-fold ensemble, CUDA and licensed-image acceptance PENDING. See [[Models/stage-iii-grading]] and `Dird/docs/features/P0-03-lg-drg-grading/m5a-cascade-results.md`.
- **GUI Tasks 01–05 — REPORTED/PARTIAL.** Owned by Antigravity. See [[Integration/gui-source-findings]].
- **GPU — UPDATED.** `nvidia-smi` reports RTX 5070 Laptop GPU, driver 580.178.04, 8151 MiB. CUDA ONNX execution NOT yet verified.
- **Native packaging — PARTIAL.** Windows runtime and target GPU untested.

## Quality gates

- REPORTED (not rerun): Rust workspace 59 passed, 0 failed, 7 ignored; Python 8 passed.
  Evidence: [P0.2 test record](../../../docs/features/P0-02-stage2-segmentation/07-test-results.md).
- Verified 2026-10-28 by `cargo test --workspace` (after CWS): 63 passed, 0 failed, 11 ignored. Heavy MA/HE/EX/CWS tests pass in ai-bridge and ModelService with env vars. fmt, strict Clippy, release build and `git diff --check` pass.

Next actions: [[Handoffs/next-actions]]. Safety limits: [[Models/licensing-and-data]],
[[Testing/cuda-and-packaging]], [[Decisions/risks-and-blockers]].
