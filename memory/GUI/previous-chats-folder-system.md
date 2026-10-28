<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/previous-chats-folder-system.md (sha256 93bb9dded6b2); verify current state in [[active-phase]] -->
# Previous Chats and History Workspace

Previous Chats page is hosted at AppPage::PreviousChats.
Architecture implements typed HistorySubroute with List and Detail destinations.
Desktop layout uses two-column master-detail view for screens 900px or wider.
Left column renders vertically scrollable consultation cards list.
Right column renders detailed conversation preview with quick actions.
Narrow layout uses single-column card stack with drill-down navigation.
Search input filters consultations across title, preview, and session identifiers.
Filter scope supports All, Active Only, and Patient Scoped queries.
Sort order supports Most Recent, Oldest First, Title, and Session ID.
Detail view displays full read-only transcript with message bubbles and timestamps.
Safety notice confirms AI inference and model re-execution are disabled on replay.
Truthful banner informs operator when backend chat persistence is disconnected.
Deep links provide one-click navigation to Patient Profile, Exam Detail, and Quad.
All synthetic fixtures are explicitly labeled with SYNTHETIC TEST FIXTURES badge.
Zero unencrypted plaintext PHI is dumped to persistent disk.
