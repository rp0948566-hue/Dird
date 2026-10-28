<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/security.md (sha256 fb8dfcd3e3cd); verify current state in [[active-phase]] -->
# Security Architecture and Defensive Hardening

Security principles and threat mitigation for RetinaCare native software.
RetinaCare is a 100% native Rust desktop binary with zero WebView and zero browser runtime.
Never commit or hardcode credentials; read strictly from environment variables.
Zero SQL engine is compiled, linked, or imported, ensuring complete immunity against SQL injection.
All user inputs pass through security::sanitize_prompt_text before ingestion.
Prompt sanitization strips terminal ANSI escape sequences (\x1b) to prevent terminal injection.
Prompt sanitization strips null bytes (\0) and unprintable ASCII control characters.
Legitimate newlines and tab indentation are preserved by prompt sanitization.
User prompt length is hard-capped at 10,000 characters to prevent memory exhaustion DoS.
File attachments are validated via security::validate_attachment_path before opening.
Path traversal attempts containing ../ or ..\ are rejected immediately.
File paths are strictly canonicalized to resolve symlink redirections.
Target files must be regular filesystem files, blocking /dev/urandom and character devices.
Access to sensitive system paths (/etc, /proc, /sys, /dev, /boot, /root) is forbidden.
Attachment file size is hard-capped at 50 MB via security::verify_file_size.
PDF parsing runs offline via lopdf within bounded page limits (maximum 100 pages).
Unused network and server crates (axum, tokio, serde, serde_json) are pruned from dependencies.
All clinical screening and inference remain strictly local on device with zero telemetry.
FIPS 180-4 SHA-256 digests are computed for all ingested scans and reports.
Constant-time byte comparison (security::constant_time_eq) defeats timing side-channel attacks.
NIST SP 800-38D AES-256-GCM authenticated encryption secures consultation archives at rest.
ZeroizedBuffer uses volatile memory writes (write_volatile) to zero sensitive RAM upon drop.
Medical EXIF and JFIF metadata is stripped from images to ensure HIPAA patient de-identification.
Top bar displays a tactical shield indicator: [MIL-SPEC HARDENED] with green status dot.
Attachment chips display cryptographic verification tags with short SHA-256 hashes.
