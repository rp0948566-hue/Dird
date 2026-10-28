<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/clinical-timeline.md (sha256 0d682e12e2ce); verify current state in [[active-phase]] -->
# Clinical Timeline Architecture

Clinical Timeline page is hosted at AppPage::Timeline as primary sidebar route.
Left sidebar navigation orders Home, Report, Patients, Chats, Timeline, Plugins.
Custom timeline icon renders clock dial with horizontal axis and milestone node.
Architecture implements typed TimelineSubroute with Overview and EventDetail.
Top scope bar displays active patient code and synthetic dev fixture badge.
Filter toolbar provides Laterality toggles for Both Eyes, OD, and OS.
Event kind toggles filter Captures, Stage I Screenings, Masks, and Reports.
Time ruler displays chronological horizontal axis with tick marks and date nodes.
Ruler controls support Zoom In, Zoom Out, and Fit range actions.
Event lanes group examinations chronologically with OD and OS badges.
Retinal imaging rail displays fundus image thumbnails with offline fallback.
Contextual right inspector panel occupies 300px on desktop screens.
Inspector displays selected event metadata, laterality, and gate metrics.
Inspector provides one-click deep links to Quad, Report, Patient, and Exam Detail.
Responsive narrow layout stacks ruler, cards, and inspector vertically below 900px.
Cross-patient isolation purges selected event and thumbnails on patient change.
Navigating timeline never re-executes AI models or triggers inference.
Truthful banner informs operator when timeline persistence service is offline.
