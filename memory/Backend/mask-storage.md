---
title: Five-mask storage (schema v3)
type: contract
status: VERIFIED
verified_on: 2026-10-28
---
# Five-mask storage

Source: `native/crates/retinacare-persistence/src/masks.rs`. Evidence, matrix, threat model: `docs/features/P2-mask-persistence/README.md`. Research prototype; masks are technical outputs, not clinical findings.

## Contract
- Tables `mask_runs` (one per examination) and `mask_artifacts` (exactly five: ma, he, ex, cws, vessel) inside the SQLCipher database; written in ONE transaction (all or nothing; a rerun replaces the old run).
- Per mask: kind, size (equal to the image), model version/SHA, checkpoint SHA, preprocessing version, threshold, provider, timing, transform JSON, RLE payload, payload SHA-256. Per run: exam, patient, eye, request id, source image SHA-256, manifest SHA-256.
- API: `save_mask_run`, `load_mask_run(exam, MaskExpectation)`, `has_mask_run`, `delete_mask_run`. Load re-verifies every hash and the caller's patient, eye and image hash; failure is `MaskIntegrity` or `MaskMismatch` and returns no pixels.
- Migration: v2 to v3 additive, backup `<db>.pre-v3.bak`; legacy exams load as `None`.
- GUI: `main.rs` `save_masks_note` / `restore_stored_masks`; see [[Integration/gui-backend-integration]].

## Limits
Binary masks only (no probabilities, heatmaps or images). Not tamper-proof against a holder of the database password. Labelled Dice/IoU, CUDA and Windows are unverified; see [[Models/stage-ii-segmentation]], [[Backend/data-contracts]], [[Backend/security]].
