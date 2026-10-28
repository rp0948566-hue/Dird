<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/multimodal-ingestion-and-hierarchiretina.md (sha256 3fd68339f8ea); verify current state in [[active-phase]] -->
# Multimodal Ingestion and HierarchiRetina

## Input Validation Gates
Ingestion pipeline validates mime types, file signatures, and extensions before decode.
Rejects oversized payloads, path traversal attempts, and symlink escapes.
Pixel count, expanded byte size, and PDF page counts are strictly bounded.
Raw clinical assets are assigned immutable session IDs and stable SHA-256 hashes.
Local clinical data respects SQLCipher encryption and per-user directory privacy.

## Fundus Image Processing
Decodes image bytes using the Rust image library.
Maintains original image geometry and coordinate transform matrices for overlay mapping.
Generates bounded mipmap pyramid tiles for interactive pan and zoom in Report view.
Never downsamples or blurs away microaneurysms during clinical preprocessing.
Tracks orientation, lateral metadata (OD/OS), and calibration parameters.

## PDF Extraction Engine
Implemented in rust/src/pdf_engine.rs utilizing lopdf.
Safely extracts text spans while preserving sequential page numbering and order.
Scanned-only or image-only pages are identified and reported without invented OCR.
Extracted text is formatted with page citations and bounded character lengths.

## HierarchiRetina v1.0 Model Manifest
Documented in rust/src/model_manifest.rs.
Tracks four clinical stages: Ingestion Gate, HSMoE-AUNet, SwinHRUNet++, and LG-DRG.
Honest runtime status reports BlockedNeedsOnnxExport until PyTorch weights are converted.
Pre-configured ONNX target paths ready in weights/ directory.
Zero hallucinated model outputs or simulated diagnosis claims.
