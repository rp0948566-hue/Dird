<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/technical-handbook.md (sha256 49efb497e6f3); verify current state in [[active-phase]] -->
# RetinaCare Comprehensive Technical Handbook (80 Pages)

- Document path: docs/RetinaCare_Codebase_and_Architecture_Comprehensive_Handbook_80Pages.pdf.
- Exact page count: 80 pages (verified via pdfinfo).
- Pure native Rust architecture documentation: zero TypeScript, zero JavaScript, zero WebView.
- Covers all 14 repository source files with full code listings, line numbers, and architectural explanations.
- Section 1 (Pages 1-16): System Architecture, Ingestion Pipeline, AI Engine, UI Specifications.
- Section 2 (Pages 17-22): Cargo.toml and src/domain.rs with 4-word preview compression.
- Section 3 (Pages 23-30): src/security.rs, FIPS SHA-256, 11 CWE mitigations, EXIF scrubber.
- Section 4 (Pages 31-36): src/engine/ (mod.rs, manifest.rs, pdf.rs) with tensor checkpoints.
- Section 5 (Pages 37-44): src/ui/ (theme.rs, components.rs, composer.rs) with prompt editor.
- Section 6 (Pages 45-52): src/plugins/ (pipeline.rs, icons.rs) with vector geometry rendering.
- Section 7 (Pages 53-59): src/plugins/mod.rs with Model Context Protocol (MCP) tool registry.
- Section 8 (Pages 60-76): src/main.rs (2,243 lines) state machine, session reset, and viewport.
- Section 9 (Pages 77-80): 36 passing unit tests audit table, performance benchmarks, and attestation.
- Includes full SHA-256 cryptographic checksum registry for all 14 source files.
