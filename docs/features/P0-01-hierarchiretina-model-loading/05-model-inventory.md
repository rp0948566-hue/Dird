# HierarchiRetina model inventory

Upstream release: [v1.0](https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0). Sizes and checkpoint hashes below are copied from the upstream release manifest; no checkpoint was downloaded during this task.

| Stage | Upstream artifact | Size | SHA-256 |
| --- | --- | ---: | --- |
| I gate | `stage1_convnextv2l_768_best_auc_model.pth` | 1.61 GB | `89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff` |
| II MA | `stage2_hsmoe_aunet_MA_best_model.pth` | 0.87 GB | `42c776e9b1bc04f6fb267a3e325a0188a1fad54097293961f3e6be1200b092f8` |
| II HE | `stage2_hsmoe_aunet_HE_best_model.pth` | 0.76 GB | `c8fc44d5b14f54cb26c8418d6e40c61c283bf7daae7a1b3fd2ff31923371d94a` |
| II EX | `stage2_hsmoe_aunet_EX_best_model.pth` | 0.75 GB | `6a4bd1b40749475dbb36a1812e08d5caf3a4fce5d2c90892c0f53ab55e64453e` |
| II CWS | `stage2_hsmoe_aunet_CWS_best_model.pth` | 0.77 GB | `c42c9b7754c71c474d9f58587d3fbe63e17deb5611a3f5656e76f0415bd5bba2` |
| II vessels | `stage2_swinhrunetpp_best_dice.pth` | 0.44 GB | `32bad65f3a1cf099e847fd292ff5ff84199ed0882112c0a22499e423e481422c` |
| III fold 0 | `stage3_lgdrg_best_fold0.pt` | 0.80 GB | `c1593eacca1148213c89424f19c47b4997d30eaf62cdff971ed147e529ccf57c` |
| III fold 1 | `stage3_lgdrg_best_fold1.pt` | 0.80 GB | `9efb5c932ef7142e36f4e6b0b1f170936fdf771df659dbd53a0b82fa889714eb` |
| III fold 2 | `stage3_lgdrg_best_fold2.pt` | 0.80 GB | `2336a1a56cb82c2f6eb47eb981e57974d9f47b066382e2c2b1b9d923b6f91e0f` |
| III fold 3 | `stage3_lgdrg_best_fold3.pt` | 0.80 GB | `67341f8973c759bcf3dcb72e1dc75058d37a4b573e511fd7e180e663a9acdb99` |
| III fold 4 | `stage3_lgdrg_best_fold4.pt` | 0.80 GB | `239107b0c4f30cdf3aa92dd16b1bb7df564e4add5edac831ca59b85ca1ee888e` |
| III fold 0 γ fix | `stage3_lgdrg_gammafix_best_fold0.pt` | 0.80 GB | `18a9a2cf885174cd6f586d9eed478161e6d726ed016b88f78596360ae019df81` |

The local Stage I `.pth` exists at `models/stage1_convnextv2l_768_best_auc_model.pth` (1,614,202,699 bytes) and independently matches upstream SHA-256 `89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff`. The upstream source checkout is at commit `27f49f5ccc1d6297903dd651cef8a660eecc48ce`.

Local `models/stage1_convnextv2l_768/stage1.onnx` is 807,176,466 bytes, SHA-256 `9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac`. Read-only PyTorch/ONNX parity and the Rust ONNX golden test passed for stored synthetic inputs; see `08-test-results.md`. All model artifacts remain outside Git.
