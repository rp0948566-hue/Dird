<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/gotchas.md (sha256 231979ad2815); verify current state in [[active-phase]] -->
# Gotchas

## Project Pitfalls and Operational Rules
Strict Rule: Do strictly and only what is instructed. Never over-engineer.
Ponytail Ladder: YAGNI -> reuse local -> stdlib -> native platform -> one line -> minimum code.
Shortest diff wins: Avoid speculative abstractions and unwanted scaffolding.
Never use Id::unique inside per-frame egui render loops; it allocates new IDs every frame and drops clicks.
Always use stable Id::new or ui.auto_id_with for buttons, window controls, and animations.
String slicing on UTF-8 text must never slice raw bytes; use strip_prefix or chars().take().
Adaptive chat bubble width must clamp minimum width to avoid squished text or over-stretching.
Folder card preview text must be strictly compressed to at most 4 words.
Message count badge must remain omitted from folder card footers.
Plugins deck must focus strictly on plugins and exclude skills.
RetinaCare runs as a pure Rust desktop binary without WebView or embedded browser runtimes.
Obsidian memory vault files must stay under 80 lines with one fact per line.
Home composer popup menus must use stable egui::Area with fixed_pos to avoid flicker.
Never fabricate AI inference metrics in frontend; show awaiting backend or offline mode.
Chat event channels must use nonblocking try_recv in GUI frame loop to prevent frame stalling.
Never clear user composer input before local message acceptance is confirmed.
Coordinate inspection must clamp u and v between 0.0 and 1.0 and reject positions outside display_rect.
Cursor-centered zoom must compute new_pan relative to the focal point under mouse pointer, not image center.
Never execute git push or git add without explicit user confirmation.
Cargo path dependencies can resolve workspace-inherited versions by traversing to external workspace root.
In std::sync::mpsc::Receiver is Send but not Sync; wrap service in Mutex to satisfy Send + Sync bounds.
In OpenCV Rust bindings, &to_string_lossy produces &Cow which does not satisfy AsRef<OsStr>; pass PathBuf directly.
In unit tests, avoid loading 800MB ONNX models across multiple app instances; use cfg(test) to select mock service.
Examination isolation must match result examination_id against active exam_id to reject stale background events.
Stage I screening is a binary routing gate and must never be represented as a conversational LLM or 5-class DR grade.
pkill -f matches full command lines including active bash subshells; use pkill -x with exact binary name instead.
Title bar custom buttons must offset the window drag interact rect to avoid dragging window on button clicks.
Capturing X11 windows with ffmpeg requires polling until xwininfo reports Map State IsViewable.
Grabbing an IsUnMapped window produces blank 0-byte frames or black images; wait 1.2s after viewable.
ffmpeg grab with -window_id $WIN_ID works reliably on Linux X11 whereas coordinate grab can misalign on Wayland.
Codex retinacare-reports crate contains only ReportData struct and lacks PDF rasterization or printpdf engine.
Codex retinacare-persistence SqlCipherStore lacks audit_events table and audit query APIs.
Do not fabricate synthetic PDF files or fake audit records in production; report BackendUnavailable truthfully.
Codex SqlCipherStore requires Argon2id unlock passphrase and returns Database Locked if unauthenticated.
Never populate production patient UI with mock data; use RETINACARE_DEV_FIXTURES=1 for testing only.
Switching patients must purge fundus textures and active request IDs to prevent cross-patient contamination.
Eye laterality OD (right) and OS (left) must never be inferred automatically from retinal image features.
Exporting patient session as unencrypted JSON must be explicitly labeled Plaintext Sensitive Export.
In egui 0.37 Frame::none() is removed; use Frame::new() instead.
Sizing child scroll areas in egui 0.37 requires ui.new_child(egui::UiBuilder::new().max_rect(rect)).
Enums deriving Copy must not call .clone(); clippy warns on clone_on_copy.
Deriving Default on enums requires marking a variant with the default attribute.
On Wayland and X11 idle windows without a heartbeat suffer missed frame callbacks and unresponsiveness.
Scheduling ctx.request_repaint_after with 500ms heartbeat maintains event loop responsiveness.
Glow OpenGL renderer provides superior sleep and DPMS recovery compared to default Wgpu on Linux.
