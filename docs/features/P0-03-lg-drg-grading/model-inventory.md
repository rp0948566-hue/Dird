# LG-DRG model inventory (2026-10-28)

Only fold 0 is local (owner-approved download 2026-10-28). Folds 1-4 are absent and need one approval each. All weights CC BY-NC 4.0; keep out of Git.

| Asset | Bytes | SHA-256 (upstream `weights_manifest.json`) | Local | Needed |
|---|---|---|---|---|
| `stage3_lgdrg_best_fold0.pt` | 795,798,428 | `c1593eacca1148213c89424f19c47b4997d30eaf62cdff971ed147e529ccf57c` | present, SHA verified, `models/stage3_lgdrg/`; ONNX `stage3_lgdrg_fold0.onnx` 795,936,525 B `31f20d62…129cb7` | done (M1) |
| `stage3_lgdrg_best_fold1.pt` | 795,798,428 | `9efb5c932ef7142e36f4e6b0b1f170936fdf771df659dbd53a0b82fa889714eb` | absent | M4 |
| `stage3_lgdrg_best_fold2.pt` | 795,798,428 | `2336a1a56cb82c2f6eb47eb981e57974d9f47b066382e2c2b1b9d923b6f91e0f` | absent | M4 |
| `stage3_lgdrg_best_fold3.pt` | 795,798,428 | `67341f8973c759bcf3dcb72e1dc75058d37a4b573e511fd7e180e663a9acdb99` | absent | M4 |
| `stage3_lgdrg_best_fold4.pt` | 795,798,428 | `239107b0c4f30cdf3aa92dd16b1bb7df564e4add5edac831ca59b85ca1ee888e` | absent | M4 |
| `stage3_lgdrg_gammafix_best_fold0.pt` | 795,802,235 | `18a9a2cf885174cd6f586d9eed478161e6d726ed016b88f78596360ae019df81` | absent | not required |

Release: `https://github.com/asifuddin01/HierarchiRetina/releases/download/v1.0/<asset>`.
Cost estimate per fold (not measured): ~0.8 GB weights, ONNX roughly the same; CPU inference at 512×512 expected in the several-seconds range; CUDA sequential load-run-unload keeps peak far below 8 GB. Measure in M1/M4.
