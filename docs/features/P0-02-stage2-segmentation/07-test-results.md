# Stage II test results

Updated 2026-10-28.

## Baseline rechecked

- `git status --short --branch`: `main...origin/main`; HEAD `7286cf7`, equal to
  origin/HEAD. Existing modified tracked files and broad untracked project
  files were preserved; no commit/reset/clean was run.
- Pinned upstream source v1.0 tag is `bf84b1d3c7409d2575dff84c7575c67577493a30`;
  local Stage II source tree compares identical.
- Local model scan found only Stage I checkpoint and ONNX plus package/sample
  ONNX files; no Stage II model weights.
- Stage I baseline: in the immediately preceding 2026-10-28 turn, workspace
  tests 53 passed/0 failed/3 ignored; Python worker 5 passed; fmt, strict
  Clippy, release build passed; local preprocessing tensor and PyTorch/Rust
  ONNX parity each measured zero error; nonblocking image-service test passed.
  The demo image's provenance/license remains undocumented, so this is LIMITED
  technical evidence, not licensed-image acceptance. P0.1 stays PARTIAL.

## Stage II

- Pinned source v1.0: `bf84b1d3c7409d2575dff84c7575c67577493a30`.
- User authorized MA only. Downloaded checkpoint to
  `/tmp/retinacare-models/stage2_hsmoe_aunet_MA_best_model.pth`, outside Git;
  size `874071456`; SHA-256
  `42c776e9b1bc04f6fb267a3e325a0188a1fad54097293961f3e6be1200b092f8`.
  SHA and size verified before safe `torch.load(weights_only=True)`.
- Strict-loaded official MA HSMoE-AUNet configuration with `pretrained=False`;
  exported only main logits to opset 17 ONNX. Graph:
  `/tmp/retinacare-models/ma/stage2_ma.onnx`, size `872293637`, SHA-256
  `ac874bee868bbd7109d1358deaa35ab59f38d7664a4ea7eec22e00c8be79aac5`.
- Python/CPU ONNX Runtime, same local-only image: output `[1,1,1024,1024]`;
  max absolute logit difference `0.0004062653`, RMSE `0.0000603426`, max
  probability difference `0.0000569991`; thresholded mask agreement `1.0` at
  upstream `>0.01`. Both repeated outputs were bit-identical. The `1e-3`
  absolute-logit gate is 2.46x the observed max difference and covers measured
  CPU operator/export drift while retaining exact decisions on this input.
- Rust Stage II preprocessing tensor vs Python reference: max absolute
  difference `0.0` over all `3×1024×1024` float values. Upstream crop
  `[x1,y1,x2,y2]=[28,9,640,623]`, source frame `640×640`.
- Rust `VerifiedSession` CPU output vs PyTorch complete logits: max absolute
  difference `0.000408173`, RMSE `0.000060387`; full source-frame thresholded
  binary mask exactly matched Python. Rust result recorded source image SHA
  `32d756fc923262daa5296c6852c1569e01394b37db57e72385785fca9017c899`,
  ONNX SHA above, frame `640×640`, 58 positive pixels, total preprocessing +
  inference `4,341,122 µs` on this Ubuntu CPU.
- The image `public/demo-images/session-1-final/OD-1.png` was used only for
  temporary local technical testing under prior user authorization. Its
  provenance/license is undocumented; it is not formal licensed-image
  acceptance, has not been copied into model fixtures, and no diagnosis or
  clinical validation is claimed.
- Typed contracts and synthetic tests cover model thresholds, mask payload
  validation, shape, transform bounds and registration. Actual Rust runtime and
  nonblocking app-service image integration passed, including exact PyTorch
  mask agreement.
- Full Rust workspace after MA integration: 59 passed, 0 failed, 5 ignored.
  Ignored tests are optional local model/image checks; separate MA real-image
  direct-runtime and nonblocking-service tests were both run and passed.
- Python worker plus checkpoint-integrity tests: 7 passed.
- `cargo fmt --all --check`, strict workspace Clippy, release workspace build
  and final `git diff --check` passed.
- User-authorized HE checkpoint: `/tmp/retinacare-models/stage2_hsmoe_aunet_HE_best_model.pth`,
  exact size `762747290` and SHA-256
  `c8fc44d5b14f54cb26c8418d6e40c61c283bf7daae7a1b3fd2ff31923371d94a` verified.
  The converted CPU ONNX graph is `/tmp/retinacare-models/he/stage2_he.onnx`,
  `760971040` bytes, SHA-256
  `16f767a8deb3751962d00abfaa5aca27fb40b43c64a77108c78610111260b966`.
- HE Python PyTorch/ONNX Runtime parity on the same temporary local-only image:
  input `[1,3,1024,1024]`, logits `[1,1,1024,1024]`, max logit difference
  `0.0003967285`, RMSE `0.0000034762`, max probability difference
  `0.0000120616`, threshold-mask agreement `1.0` at strict `>0.20`; repeat
  outputs exact. CPU provider. Absolute-logit tolerance `0.001`.
- Rust HE direct runtime: input tensor matched Python with max error `0.0`;
  max logit difference `0.000419617`, RMSE `0.000003558`; registered source
  frame `640×640`, `1574` positive pixels, source image SHA-256
  `32d756fc923262daa5296c6852c1569e01394b37db57e72385785fca9017c899`, total
  preprocessing plus inference `4,162,171 µs` on Ubuntu CPU. Full mask exactly
  matched Python. Rust nonblocking app-service integration also passed.
- After HE integration the full Rust workspace reported 59 passed, 0 failed,
  7 ignored; Python tests reported 8 passed. `cargo fmt --all --check`, strict
  workspace Clippy, release build and `git diff --check` passed.
- No EX/CWS/vessel weights downloaded. P0.2 remains PARTIAL; P0.1 CUDA and
  documented licensed-image gates remain open.
- `nvidia-smi` failed to communicate with the NVIDIA driver. CUDA/RTX 5070
  execution and memory measurements remain unverified.
