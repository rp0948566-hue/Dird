<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/tech-stack-and-architecture.md (sha256 aa1cebc71cbd); verify current state in [[active-phase]] -->
# Tech Stack and Architecture

RetinaCare is a 100% native Rust desktop executable for Linux x86_64.
Zero TypeScript, zero JavaScript, zero React, and zero WebView.
GUI framework: eframe and egui 0.37 with native OpenGL / Wayland / X11 rendering.
Single self-contained ELF binary compiled to rust/target/release/retinacare.
Runs with zero external browser processes and zero Chromium memory overhead.
Security layer: rust/src/security.rs handles path validation, prompt sanitization, and DoS bounds.
Zero SQL engines are linked or present, giving absolute zero-SQL attack surface.
Domain state: rust/src/domain.rs with ChatSession, ChatMessage, LocalAttachment, and 4-word preview.
Engine layer: rust/src/engine/ containing pdf.rs, manifest.rs, and chat.rs.
PDF extractor: rust/src/engine/pdf.rs performs bounded lopdf extraction under 50 MB limits.
Clinical model audit: rust/src/engine/manifest.rs tracks HierarchiRetina v1.0 ONNX export status.
Chat engine: rust/src/engine/chat.rs provides nonblocking ChatService trait and streaming mock.
UI module: rust/src/ui/ containing theme.rs, components.rs, and composer.rs.
Floating Home Composer: rust/src/ui/composer.rs renders minimal dark card #242424 with 20 px radius.
Plugins subsystem: rust/src/plugins/ with 12 authentic MCP connectors and vector tiles.
UI entry point: rust/src/main.rs orchestrates navigation, docking, and workspace canvas.
Desktop integration: Wayland/X11 app_id retinacare, high-res 256x256 icon, .desktop launcher.
Obsidian memory vault: memory/ directory with interlinked Markdown documentation.
