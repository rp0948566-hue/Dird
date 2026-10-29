---
title: Figma design system and redesign (design stage, awaiting approval)
type: status
status: DESIGN-DONE-AWAITING-APPROVAL
verified_on: 2026-10-29
---
# RetinaCare Figma redesign (design stage only; no Rust changed)

File: https://www.figma.com/design/zV5EBNbVGXyo8kRJTN9F3d (account: VANSH BTECH_2025's team, Student plan).
Prototype (click-through): page `Prototype`, start frame `Home — Research Analysis — Dark` (node 3:2).
Gate: user approval before any Rust work. Related: [[GUI/gui-active-phase]].

## Decisions
- Style: minimal layout plus skeuomorphism (raised buttons, inset fields, 1 px borders). Glass (translucent + blur) only on a few buttons: mode menu, icon buttons, toolbars, close, page controls. Panels and cards are NOT glass.
- Palette source of truth is CMYK (light bg C0 M1 Y5 K6 = #F0EDE4, dark bg K91 = #171717). Figma stores RGB; CMYK printed on swatches and in variable descriptions.
- One `Color` collection with real Dark/Light modes, plus Space, Radius, fx/highlight, fx/shadow, glass/surface, glass/sidebar.
- Fonts: Geist, Instrument Serif, Geist Mono.
- Real image samples (cropped from the user's own app screenshot): fundus, lesion overlay, vessel overlay. Heatmap is shown as "unavailable until a real run", never drawn.
- Dialogs are 85 % of the screen (1360x765 on 1600x900), centered, with a two-column layout; composite previews show them over the dimmed app.
- Report: six A4-style paper pages (summary, segmentation, explainability, doctor notes, provenance, limitations), left page rail, glass page controls, page-turn moves.
- Typing bar: attachments appear above the text as a fanned stack with a "3 files" pill; accent send button.
- Analysis gallery: six large tiles (original, MA, HE, EX, CWS, vessel).
- Timeline is an agent-style accordion; Explainability, Vessel and Timeline side panels have no dead space.

## Round 3 changes (2026-10-29)
- Every screen sits in an inset `App shell`: 10 px outer gap, 14 px corners, 1 px border (outer frame uses bg/inset). Mirrors the website shell prompt the user pasted.
- Window controls (minimise, maximise, close) are larger and use text/primary.
- Responsive Home gallery: big tiles, 3 per row (2 at 700 px).
- Chat: long user message is clamped to 4 lines with "Show more" and a meta row; short messages are compact bubbles; assistant answers carry rich blocks (pie of grade probabilities, lesion bars, vessel map) outside the bubble.
- Collapsed-sidebar twin for 21 screens x 2 themes, named `<screen> (rail) — <theme>`; the sidebar toggle swaps between them with a gentle spring smart-animate (0.6 s). All nav, buttons, report page turns and overlays use the same gentle easing.
- Fixed: light dialogs and light report frames had been created on the Cover page; now moved into Prototype.

## Prototype limits
- Figma only links frames on the same page, so all screens live on page `Prototype`.
- Overlay position and background dim are not settable through the API; set them in Figma if the default is wrong.
- Not click-tested by hand.

## Known gaps
- Screens are token-bound frames, not instances of the component library.
- Sample numbers and mask tiles are layout samples (MA, HE, EX and CWS tiles reuse one lesion composite).
- Responsive frames only cover Home.
- Dialog open animation from the clicked button is not possible; overlays move in from the bottom.

## Rust implementation (started 2026-10-29, design approved)
- New module `GUI/Hackthon dezine/rust/src/ui2/` runs beside the old UI; enable with `RETINACARE_UI=v2` (light: `RETINACARE_THEME=light`). Old UI untouched and kept until the user approves removal.
- Tokens pulled from Figma into `GUI/Hackthon dezine/design/figma-tokens.json`; `design/gen_tokens.py` generates `ui2/tokens.rs`; a test checks they stay in sync.
- Done: palette (CMYK-derived, dark+light), widgets (raised/primary/ghost/glass buttons, chip, toggle, segmented, cards, progress), inset shell, window bar with darker controls, collapsible sidebar (rail 64 px <-> 220 px, gentle spring), theme switch via the status card. Wired to the real app state (pages, chat clear, backend status text).
- Not done yet: Geist/Instrument Serif fonts, light-mode pass over old pages, new Retinal Review and Clinical Assessment pages, composer with attachments, chat bubbles with Show more and charts, report pages with doctor notes, 85 % dialogs, per-page reskin.
- Finding: light-mode white text on teal accent is 4.05:1 (AA wants 4.5 for small text); darken light accent to K50 if revised.
- Gates: 235 tests pass (10 new), clippy and fmt clean, release build ok. Evidence: `GUI/Hackthon dezine/docs/features/UI-V2/chrome-dark-1280x820.png`.

## Figma export to HTML (2026-10-29)
- New folder `GUI/figma-export/` (outside the old GUI project, nothing in it touched). Standalone HTML+CSS exported from Figma by a custom exporter run through `use_figma` (deduped classes, shared sidebar, icon sprite, tokens.css with both themes). Serve with `python3 -m http.server 8765` there.
- Exported so far: home, home-analysis, retinal-review, clinical-assessment (checked in browser, dark and light). Remaining pages listed in its README.
- Costs: official `get_design_context` is about 12k tokens per screen; the custom exporter is about 3x cheaper but each call returns at most 48 KB.
