# Stage II checkpoint inventory

Upstream size/hash values below are from the official v1.0
`scripts/weights_manifest.json` and release page. Local status is separately
verified where stated. Weight license: CC BY-NC 4.0. Official downloads are
under [release v1.0](https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0).

| Model / task | Upstream file | Bytes | Upstream SHA-256 | Architecture / tensor contract | Preprocessing / output | Local status / blocker |
|---|---|---:|---|---|---|---|
| MA / microaneurysm | `stage2_hsmoe_aunet_MA_best_model.pth` | 874,071,456 | `42c776e9b1bc04f6fb267a3e325a0188a1fad54097293961f3e6be1200b092f8` | HSMoE-AUNet base experts; f32 `[1,3,1024,1024]` → main logits `[1,1,1024,1024]` | Circle/stretch; no TTA; threshold `>0.01` | **Downloaded to `/tmp/retinacare-models/` outside Git; exact size/hash verified.** |
| HE / hemorrhage | `stage2_hsmoe_aunet_HE_best_model.pth` | 762,747,290 | `c8fc44d5b14f54cb26c8418d6e40c61c283bf7daae7a1b3fd2ff31923371d94a` | HSMoE-AUNet base + multiscale; same input/output | Circle/stretch; no TTA; threshold `>0.20` | **User-authorized; exact size/hash verified; converted and Rust CPU path checked.** Checkpoint remains outside Git. |
| EX / hard exudate | `stage2_hsmoe_aunet_EX_best_model.pth` | 751,909,002 | `6a4bd1b40749475dbb36a1812e08d5caf3a4fce5d2c90892c0f53ab55e64453e` | HSMoE-AUNet with multiscale/bright-spot/edge experts; same input/output | Rectangle/letterbox; 3-view TTA; threshold `>0.325` | **Absent**; separate approval required. |
| CWS / cotton-wool spot | `stage2_hsmoe_aunet_CWS_best_model.pth` | 771,587,386 | `c42c9b7754c71c474d9f58587d3fbe63e17deb5611a3f5656e76f0415bd5bba2` | HSMoE-AUNet with multiscale/bright/soft-region/texture experts; same input/output | Rectangle/letterbox; 3-view TTA; threshold `>0.40`; closing/components filter | **Absent**; separate approval required. |
| Vessel / retinal vessels | `stage2_swinhrunetpp_best_dice.pth` | 441,078,919 | `32bad65f3a1cf099e847fd292ff5ff84199ed0882112c0a22499e423e481422c` | SwinHRUNet++; f32 `[1,3,512,512]` → eval main logits `[1,1,512,512]` | FOV/CLAHE/green; square crop; 4-view TTA; threshold `>=0.46`, FOV/open/components | **Absent**; separate approval required. |

The five Stage II weights total **3,601,394,053 bytes** (about 3.60 GB decimal).
Only user-approved MA and HE checkpoints were downloaded; both are under
`/tmp/retinacare-models/` outside Git. Conversion graphs and local parity
fixtures are also stored there. No EX, CWS or vessel checkpoint was downloaded.
The upstream weights license is CC BY-NC 4.0; the repository code is MIT.

Source code MIT license does not grant model-weight rights. Keep these
CC BY-NC 4.0 weights outside Git and do not distribute them commercially.
