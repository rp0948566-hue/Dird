# Stage II design and implementation plan

## Current verified baseline

Branch `main` at `7286cf7` equals `origin/main`. Existing worktree has 10
modified tracked files and broad untracked project files; preserve all. Stage I
CPU image preprocessing, ONNX inference, and the nonblocking service test
passed in the immediately preceding 2026-10-28 verification. Workspace was
53 passed/0 failed/3 ignored; Python worker 5 passed; fmt, strict Clippy, and
release build passed. P0.1 stays PARTIAL for CUDA and licensed-image evidence.

## Build sequence

1. P02-A: inventory all five official artifacts and pin v1.0 source contracts.
   Done in [01](01-upstream-research.md) and [02](02-checkpoint-inventory.md).
2. P02-B: MA and HE conversion and CPU parity complete. Safe load used verified
   bytes, `weights_only=True`, strict key load, eval mode and main-logit-only
   export. Exact hashes and parity are in [07](07-test-results.md).
3. P02-C: MA/HE typed inference, source-matched OpenCV preprocessing,
   registered mask restore and nonblocking service paths are implemented and
   exercised through `VerifiedSession`.
4. P02-D: repeat for EX, CWS and vessel only after individual approval, hash
   check, conversion, parity and Rust CPU execution. Add vessel after lesions.
5. P02-E: run a documented licensed image, Stage I regression and all Rust
   gates; document true masks, hashes, timings and open issues.

Stage II API stays in `retinacare-ai-bridge` and `retinacare-app`; GUI and
database/report code remain untouched. Results carry examination ID, source
SHA, optional laterality, model kind/version/hash, frame and mask dimensions,
registered transform, output encoding, threshold metadata, status and error.
No persistent patient image/mask files should be created by this backend task.

Detailed pixel and value fields are in [04](04-mask-spatial-contract.md).
Conversion acceptance is in [05](05-conversion-parity.md). Each remaining
checkpoint needs separate authorization. MA uses CPU only in current test
evidence; CUDA remains unverified.
