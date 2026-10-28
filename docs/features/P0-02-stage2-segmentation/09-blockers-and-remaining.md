# Blockers and remaining work

## Approval required

The local scan initially found no Stage II weights. The user approved and the
official MA checkpoint, `stage2_hsmoe_aunet_MA_best_model.pth`, 874,071,456 bytes,
SHA-256 `42c776e9b1bc04f6fb267a3e325a0188a1fad54097293961f3e6be1200b092f8`,
from [release v1.0](https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0),
is downloaded outside Git and hash-verified. Weights are CC BY-NC 4.0.

The user separately authorized the official HE checkpoint. It is downloaded to
`/tmp/retinacare-models/stage2_hsmoe_aunet_HE_best_model.pth`, outside Git, and
its upstream size and SHA-256 are verified. EX, CWS and Vessel still require
separate approval before download. The upstream helper's `--stage 2` fetches all
four lesion files; do not run it.

## Not yet demonstrated

- EX, CWS and vessel checkpoint approval, conversion, graph hashes and parity.
- At least one licensed/permissioned retinal image for formal end-to-end acceptance.
- Cancellation during active inference and explicit OOM recovery are not
  implemented. MA and HE preprocessing, inference, full-frame registration and
  nonblocking app-service delivery are implemented and exercised on CPU.
- RTX 5070/CUDA execution and memory measurements.

P02-A and MA/HE P02-B/P02-C slices are complete. P02-D remains open for three
models; P02-E remains open for licensed-image acceptance and other model checks.
Full Rust/Python quality gates pass, but stage acceptance remains **PARTIAL**.
