# P0.2 — Stage II segmentation

Status: **PARTIAL — MA and HE checkpoints verified, converted, numerically
compared, and run through Rust CPU inference. EX/CWS/vessel remain absent.**
P0.1 remains PARTIAL separately: CUDA/RTX 5070 and documented licensed-image
acceptance are open.

Stage II contract: four independent HSMoE-AUNet lesion streams (MA, HE, EX,
CWS) and one SwinHRUNet++ vessel stream. Each must produce its own original
image-aligned typed mask. No lesion counts, DR grade, vessel caliber, artery/
vein labels, or clinical claim may be inferred from missing/unverified models.

MA and HE lesion models are integrated with their original thresholds and no
TTA. Their official checkpoints are SHA-256 verified and kept outside Git under
`/tmp/retinacare-models/`. Weights are CC BY-NC 4.0; retain for noncommercial
use only. EX, CWS and vessel still require separate approval. See [inventory](02-checkpoint-inventory.md)
and [test results](07-test-results.md).

| Document | Purpose |
|---|---|
| [01 upstream research](01-upstream-research.md) | Pinned source, operators, tensor contracts, licenses |
| [02 checkpoint inventory](02-checkpoint-inventory.md) | Five upstream hashes/sizes and local availability |
| [03 implementation plan](03-design-and-implementation-plan.md) | Ordered milestones and Rust integration design |
| [04 mask contract](04-mask-spatial-contract.md) | Values, thresholding, coordinate registration |
| [05 conversion/parity](05-conversion-parity.md) | Trusted conversion and comparison gates |
| [06 test plan](06-test-plan.md) | Required tests and commands |
| [07 results](07-test-results.md) | Evidence actually run |
| [08 file changes](08-file-change-report.md) | This phase's changed files |
| [09 blockers](09-blockers-and-remaining.md) | Permissions and remaining work |
| [10 Stage III handoff](10-stage3-handoff.md) | Future five-mask input only; no Stage III work |
