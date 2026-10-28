# RetinaCare native workspace

This independent workspace is being built alongside the original DIRD+ React/Tauri tree. It is licensed under AGPL-3.0-or-later, matching the repository license. Legacy source remains untouched for progressive migration and behavior comparison.

Crates are split by domain, encrypted persistence, report data, Python worker IPC, application lifecycle, and eframe UI. Build with `cargo build --manifest-path native/Cargo.toml`; run with `cargo run --manifest-path native/Cargo.toml -p retinacare-ui`.

The application is an early migration shell. Patient storage is not yet wired into the UI, and no trained model or inference adapter is bundled.
