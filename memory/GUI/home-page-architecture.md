<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/home-page-architecture.md (sha256 879f7dbb3103); verify current state in [[active-phase]] -->
# Home Page Architecture

- Workspace canvas fill is #191919.
- Home composer background fill is #242424.
- Outer border stroke is 1.0 px #383838.
- Corner radius is 20 px on the floating composer card.
- Text input uses multiline TextEdit with frame(false).
- Text placeholder is "Ask anything, or task an agent...".
- Attachment plus button toggles egui popup for image and PDF selection.
- Model selector pill button toggles egui popup showing backend connectivity status.
- Truthful states displayed: "HierarchiRetina (Awaiting Backend)" and "Offline Mode".
- Send button is 34 px circular button with upward arrow.
- Send button is enabled only when non-empty text or attachments exist.
- Supporting utility row sits 14 px below composer with Choose Project and Plugins.
- Red status dot and top/bottom highlight bevel lines are completely eliminated.
- File filters strictly separate images (PNG, JPG, WEBP, BMP) from PDF clinical reports.
- All 53 unit and integration tests pass without warnings under -D warnings.
