---
title: Model and artifact inventory
type: inventory
domain: AI
status: PARTIAL
verified_on: 2026-10-28
source_refs:
  - "../models/stage1_convnextv2l_768/manifest.json"
  - "../docs/features/P0-02-stage2-segmentation/02-checkpoint-inventory.md"
owner: Codex/Claude backend
tags: [models, hashes, licensing]
---
# Model inventory

| Artifact | Current local evidence | Rights / status |
|---|---|---|
| Stage I ONNX | Dird models/stage1_convnextv2l_768/stage1.onnx, 807,176,466 bytes, SHA-256 9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac | Manifest says CC BY-NC 4.0 weights; outside-Git ignored storage. |
| Stage I PyTorch | models/stage1_convnextv2l_768_best_auc_model.pth is present; manifest records upstream SHA 89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff | CC BY-NC 4.0 restriction. |
| Stage II MA | /tmp/retinacare-models/stage2_hsmoe_aunet_MA_best_model.pth (874,071,456 bytes); ONNX /tmp/retinacare-models/ma/stage2_ma.onnx | User-authorized; checkpoint and graph hashes in linked test report; outside Git. |
| Stage II HE | /tmp/retinacare-models/stage2_hsmoe_aunet_HE_best_model.pth (762,747,290 bytes); ONNX /tmp/retinacare-models/he/stage2_he.onnx | User-authorized; checkpoint SHA c8fc44d5b14f54cb26c8418d6e40c61c283bf7daae7a1b3fd2ff31923371d94a; graph SHA 16f767a8deb3751962d00abfaa5aca27fb40b43c64a77108c78610111260b966. |
| Stage II EX/CWS/vessel | No local files found in repository scan or temporary model listing | No authorization or download; pending. |
| Stage III LG-DRG | No local checkpoint/manifest or integrated model path verified | Pending. |

MA/HE local hash and parity evidence is in
[P0.2 test results](../docs/features/P0-02-stage2-segmentation/07-test-results.md).
MA/HE Stage II weights are CC BY-NC 4.0; keep outside Git and do not use
commercially. See [[model-pipeline]], [[licensing-and-data]].
