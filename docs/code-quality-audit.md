# Phase 1.5 code-quality audit

| Severity | Confirmed issue | Fix and evidence |
|---|---|---|
| High | Checkout-relative worker paths broke packaged builds | Executable-relative resource discovery; packaged smoke passes |
| High | SQLCipher was not connected to native screens | Bounded background controller, patient/examination UI, round-trip tests |
| High | Worker timeout/drop could leave a child process | Cancellation, timeout kill/reap, shutdown and no-response tests |
| High | Rekey salt commit could strand a valid new key | Primary/next/recovering salt recovery tests |
| Medium | IPC accepted unbounded/mismatched responses | 1 MiB response limit plus request/protocol/status validation tests |
| Medium | Image decode blocked UI and allowed excessive allocation | Background decode with file, dimension, and allocation limits |
| Medium | Password/key material lived in ordinary strings | `Zeroizing` buffers at Rust boundaries |
| Medium | Installed model path was read-only | Per-user writable data-directory model path |
| Low | Windows app lacked executable/window icon | ICO embedded in PE resources and loaded by eframe |

Remaining production gaps: no trained model adapter, no audit-log design, no signed Windows binary/installer, and no target-hardware validation. Medical output remains intentionally unavailable.

RustSec scan of 393 locked crates found no vulnerabilities. It reported three unmaintained Linux accessibility transitives (`zbus_names`, `zvariant`, `zvariant_derive`) pulled through eframe/accesskit; replacement depends on upstream. Packaged Python `pip check` is part of release verification.
