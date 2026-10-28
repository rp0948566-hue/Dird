<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/pixel-accurate-frame-recreation.md (sha256 4546f6bb5777); verify current state in [[active-phase]] -->
# Pure Rust Desktop Executable & Pixel-Accurate UI

## Status
- **Architecture**: 100% pure Rust desktop software executable (`eframe` / `egui 0.37.0` + `rfd 0.17.2` + `image 0.25.10`).
- **Binary**: `rust/target/release/retinacare` (ELF 64-bit Ubuntu executable).
- **Execution**: Runs directly on Ubuntu desktop (`DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`).

## Updates Implemented
1. **Single Image Upload**:
   - Only Box 0 ("Original Image") displays the uploaded scan.
   - Boxes 1, 2, and 3 remain completely empty.
2. **Clean Home Page**:
   - Center box is completely empty (pure soft-black `#242526`, zero text, zero prompt clutter).
3. **Report Page**:
   - All clinical findings, biomarker tables, and diagnostic data moved exclusively to the dedicated Report page.
4. **Sidebar Toggle Button**:
   - Fixed unresponsiveness by using `ui.interact` and adding dedicated gutter clearance when collapsed.
5. **Ubuntu Dock Icon**:
   - Linked window class with `with_app_id("retinacare")`, 256x256 high-resolution icon, and `retinacare.desktop` launcher so the generic grey gear is replaced with the crisp RetinaCare icon.
6. **Ghost.md**:
   - Created `Ghost.md` logging all bugs, fixes, and architecture.
