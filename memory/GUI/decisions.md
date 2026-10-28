<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/decisions.md (sha256 706467ad975e); verify current state in [[active-phase]] -->
# Architectural Decisions

## Core Architecture
RetinaCare frontend is 100% native Rust using egui and eframe 0.37.
Zero web technologies are permitted: no HTML, CSS, JS, TS, React, Electron, Tauri, or WebView.
Backend integration connects to Codex crates retinacare-app and retinacare-ai-bridge via path dependencies.
Model inference runs strictly in a background worker thread over nonblocking channels.
In-process MedicalInferenceService trait separates live backend from deterministic test mocks.
Stage I screening gate uses HierarchiRetina ConvNeXt-V2-Large with attention pooling.
Stage I threshold is pinned to 0.2391716 separating NegativeRoute from ContinueCascade.
Stage I screening is decoupled from conversational ChatService to prevent hallucinated advice.
Stages II, III, and IV remain marked ArtifactUnavailable until authentic backend models exist.
Decorated or simulated Grad-CAM heatmaps are strictly prohibited.
Retinal images and sensitive clinical records remain on local disk without network transmission.
Inference requests carry stable correlation UUIDs and match active examination IDs.
Stale results arriving after image or consultation switching are dropped silently.
Sidebar toggle button is located on the top left of the title bar to avoid body overlap.
Title bar window drag region excludes the toggle button to prevent accidental dragging.
Report overview is isolated in ReportViewModel to prevent direct coupling between raw backend structs and presentation.
Mandatory research caveat is enforced on all clinical overview views and export payloads.
Export state machine handles nonblocking channel updates across Idle, Preparing, Generating, Saved, and Failed.
PDF service truthfully reports BackendUnavailable because retinacare-reports lacks generation engine.
Audit service truthfully reports Unavailable because retinacare-persistence lacks audit_events table.
Scoped patient session export packages only active patient and examination data into local JSON.
PatientService trait decouples live SQLCipher backend from test mocks.
LiveCodexPatientService truthfully reports DatabaseLocked when unauthenticated.
MockPatientService test fixtures activate only under RETINACARE_DEV_FIXTURES=1 or cfg(test).
Switching patients strictly invalidates pending requests and purges previous fundus textures.
Eye laterality OD vs OS is explicitly clinician-assigned and never inferred from appearance.
Patient session JSON export is explicitly labeled Plaintext Sensitive Unencrypted JSON.
Patient navigation routes use typed PatientSubroute with Directory, Profile, and ExamDetail.
Report workspace routes use typed ReportSubroute with Quad, Overview, PDF, Session, and Audit.
Viewport tools render strictly in QuadViewer subroute to prevent toolbar overcrowding.
Large desktop widths use two-column master-detail layout in Patient Directory for scannability.
Export and audit views are presented as routed full-context views rather than modal dialogs.
Low-frequency 500ms heartbeat is scheduled via ctx.request_repaint_after to prevent idle freeze.
Glow OpenGL rendering is configured by default on Linux to ensure stability across monitor sleep.
RETINACARE_RENDERER environment variable allows operators to select Wgpu when explicitly requested.
All Obsidian memory notes must remain under 80 lines with one fact per line and no emoji.
