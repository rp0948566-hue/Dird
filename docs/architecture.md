# RetinaCare architecture

## Phase 1 boundaries

`native/` is an independent Rust workspace. `retinacare-domain` owns typed clinical entities; `retinacare-persistence` owns encrypted storage; `retinacare-reports` owns report structures; `retinacare-ai-bridge` owns the subprocess protocol; `retinacare-app` coordinates state; `retinacare-ui` owns the eframe interface.

`ai_engine/` is a persistent Python JSON Lines worker. Rust sends one versioned request per line and receives one response per line. Images stay on local paths; worker stdout is protocol-only and diagnostics go to stderr.

## Trust boundaries

The renderer has no browser persistence and no raw SQL command. The worker validates request shape and local paths. Clinical output requires a verified task-compatible trained checkpoint; missing weights return an explicit error.

## Later migration

Keep the legacy React/Tauri tree unchanged as reference while migrating data formats and features incrementally. Do not claim parity until import/export and clinical behavior have regression coverage.
