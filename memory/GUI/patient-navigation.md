<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/patient-navigation.md (sha256 d65263c9dd01); verify current state in [[active-phase]] -->
# Patient Navigation and Routed Screens

GUI Phase 1 Task 06.1 navigation redesign eliminates toolbar overcrowding and modal sprawl.
PatientSubroute enum manages Directory, Profile, and ExamDetail navigation states.
ReportSubroute enum manages QuadViewer, ReportOverview, PdfExport, SessionExport, and AuditHistory.
Patient Directory uses responsive master-detail layout with 54 percent list and 44 percent preview card.
Narrow desktop windows under 900px stack directory into a single scrollable column.
Dedicated Patient Profile screen displays patient details, dirty tracking, and exam timeline.
Dedicated Examination Detail screen displays fundus scan preview and Stage I AI screening metrics.
Stage II and Stage III advanced models remain clearly marked with truthful unavailable badges.
Report workspace isolates Quad viewport controls from clinical summary actions.
PDF Export, Session Dossier, and Audit History are rendered as dedicated subroute views.
Subroutes include standardized breadcrumb header with direct back navigation.
All navigation transitions maintain strict cross-patient isolation and zero data leakage.
Automated unit and integration test suite verifies all routing paths across 109 passing tests.
Desktop verification confirmed with 7 native X11 window captures under Ubuntu Linux.
