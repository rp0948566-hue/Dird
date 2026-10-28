<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/outer-edge-inset-shell.md (sha256 61d7cb9a1cd8); verify current state in [[active-phase]] -->
# Outer-Edge Inset Shell

## Design Treatment
- **Outer Viewport Background**: Pure black / smoky soft black (`#0d0d0f`).
- **Inset Spacing**: Exactly 8px on all 4 sides (`padding: 8px`).
- **Inner Shell Frame**:
  - `border-radius: 12px` on all four corners.
  - `border: 1px solid rgba(255, 255, 255, 0.12)`.
  - `box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3)`.
  - `overflow: hidden` to prevent content or scroll bars from escaping rounded corners.
- **Dot Matrix Placement**: The dot-grid pattern lives **INSIDE** the rounded frame, NOT outside.
- **Scroll Containment**: Inner content uses independent `overflow-y: auto`.
