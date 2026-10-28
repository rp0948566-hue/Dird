<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/retinal-viewer-architecture.md (sha256 1987eeff762c); verify current state in [[active-phase]] -->
# Retinal Viewer and Quad-Panel Architecture

- Report page renders four equal-priority diagnostic viewports in a 2x2 grid.
- ViewPanelId enum defines Original, Lesions, Vessels, and Explainability panels.
- Viewport surfaces use matte dark charcoal #101012 with neutral border #383838.
- aspect_fit calculates inscribed rectangle preserving raw optical sensor aspect ratio.
- compute_display_rect scales fitted base bounds by zoom and applies pan translation.
- screen_to_image_coords maps mouse pointer to discrete image pixel coordinates (X, Y).
- screen_to_image_coords normalizes coordinates to range [0.0, 1.0].
- Coordinates outside active display rect strictly return None to reject letterbox margins.
- cursor_centered_zoom keeps image point under mouse pointer stationary during wheel zoom.
- Zoom range is clamped between 0.25x and 10.0x.
- Double-clicking any viewport resets pan to zero and zoom to 1.0.
- Independent mode allows each panel to maintain private pan and zoom coordinates.
- Synchronized mode propagates zoom and pan across all panels displaying aligned artifacts.
- Panel maximization expands selected panel across full workspace with Escape shortcut to restore.
- Eye laterality supports explicit OD (Right Eye) and OS (Left Eye) examination metadata.
- Laterality is never inferred from visual features alone.
- AI Lesion Detection panel provides category toggles for MA, HE, EX, and CWS.
- Vessel Analysis panel supports opacity slider and mask-only versus overlay toggle.
- AI Explainability panel displays model attribution method, version tag, and legend.
- Missing AI inference outputs render truthful awaiting backend notice cards.
- Zero synthetic lesion markers, fake vessel maps, or decorative heatmaps are rendered.
- All 69 unit and integration tests pass under cargo test without warnings.
