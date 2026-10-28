---
title: Model and artifact inventory
type: inventory
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../../models/stage1_convnextv2l_768/manifest.json"
  - "../../docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md"
owner: Codex/Claude backend
tags: [models, hashes, licensing]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Hackthon dezine/Hackthon Backend/model-inventory.md (sha256 76011b4caa3f); verify current state in [[active-phase]] -->
# Model inventory

| Artifact | Current local evidence | Rights / status |
|---|---|---|
| Stage I ONNX | Dird models/stage1_convnextv2l_768/stage1.onnx, 807,176,466 bytes, SHA-256 9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac | Manifest says CC BY-NC 4.0 weights; outside-Git ignored storage. |
| Stage I PyTorch | models/stage1_convnextv2l_768_best_auc_model.pth is present; manifest records upstream SHA 89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff | CC BY-NC 4.0 restriction. |
| Stage II MA | /tmp/retinacare-models/stage2_hsmoe_aunet_MA_best_model.pth (874,071,456 bytes); ONNX /tmp/retinacare-models/ma/stage2_ma.onnx | User-authorized. Persisted copies in `Dird/models/stage2_hsmoe_aunet/ma/` (gitignored). SHA-256 re-verified 2026-10-28: pth 42c776e9…092f8, ONNX ac874bee…aac5 (both match manifest). Originals in /tmp. CC BY-NC 4.0. |
| Stage II HE | /tmp/retinacare-models/stage2_hsmoe_aunet_HE_best_model.pth (762,747,290 bytes); ONNX /tmp/retinacare-models/he/stage2_he.onnx | User-authorized. Persisted copies in `Dird/models/stage2_hsmoe_aunet/he/` (gitignored). SHA-256 re-verified 2026-10-28: pth c8fc44d5…371d94a, ONNX 16f767a8…260b966 (both match manifest). Originals in /tmp. CC BY-NC 4.0. |
| Stage II EX | User-approved 2026-10-28. `Dird/models/stage2_hsmoe_aunet/ex/stage2_hsmoe_aunet_EX_best_model.pth` (751,909,002 B) SHA `6a4bd1b4…4453e` (matches upstream). ONNX `stage2_ex.onnx` 750,097,739 B SHA `2aa92099…2612207`. CC BY-NC 4.0. Parity pass. Rust runtime with TTA pass on demo image (0 px mask disagreement vs upstream reference). Licensed-image acceptance BLOCKED. |
| Stage II CWS | User-approved 2026-10-28. `Dird/models/stage2_hsmoe_aunet/cws/stage2_hsmoe_aunet_CWS_best_model.pth` (771,587,386 B) SHA `c42c9b77…5bba2` (matches upstream). ONNX `stage2_cws.onnx` 769,742,070 B SHA `6c928a75…7967882`. CC BY-NC 4.0. Parity pass. Rust runtime with TTA and component postprocess matches upstream reference on demo image. Licensed-image acceptance BLOCKED. |
| Stage II VESSEL | User-approved 2026-10-28. `Dird/models/stage2_swinhrunetpp/stage2_swinhrunetpp_best_dice.pth` (441,078,919 B) SHA `32bad65f…1422` (matches upstream). EMA weights used. ONNX `stage2_vessel.onnx` 449,950,205 B SHA `cc7ef0bb…5d76e0`. CC BY-NC 4.0. Parity pass. Rust runtime matches upstream reference on demo image. Licensed-image acceptance PENDING. |
| Stage III LG-DRG fold 0 | `Dird/models/stage3_lgdrg/stage3_lgdrg_best_fold0.pt` (795,798,428 B) SHA `c1593eac…f57c` verified. ONNX `stage3_lgdrg_fold0.onnx` 795,936,525 B SHA `31f20d62…129cb7`, outputs `[1,4]` (3 CORN logits + gradability logit). EMA weights. CC BY-NC 4.0. Parity pass; Rust single-fold run matches. | single-fold technical |
| Stage III LG-DRG folds 1-4 | Present (approved 2026-10-28): `stage3_lgdrg_best_fold{1..4}.pt` 795,798,428 B each, SHA-256 equal to the upstream manifest; ONNX `stage3_lgdrg_fold{1..4}.onnx` 795,936,525 B with manifests in `models/stage3_lgdrg/` (gitignored). Hashes: `docs/features/P3-five-fold-ensemble/README.md`. CC BY-NC 4.0. | five-fold technical ensemble |

MA/HE local hash and parity evidence is in
[P0.2 test results](../../docs/features/P0-02-stage2-segmentation/07-test-results.md).
MA/HE Stage II weights are CC BY-NC 4.0; keep outside Git and do not use
commercially. See [[Models/model-pipeline]], [[Models/licensing-and-data]].

## Update 2026-10-28
- Stage II MA/HE manifests now exist under `Dird/models/stage2_hsmoe_aunet/{ma,he}/manifest.json` (copied from `/tmp/retinacare-models`; sha256 matched the ONNX files; gitignored). The cascade loads seven manifests: stage1, ma, he, ex, cws, vessel, stage3 fold 0 (`discover_cascade_paths`).
- Model hashes shown in the GUI report/PDF come from the backend result, not from constants. Stage III fold 0 ONNX sha256 `31f20d62...129cb7`.
- Nothing downloaded in this task. Folds 1-4 still need separate approval.
