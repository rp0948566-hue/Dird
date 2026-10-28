<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/chat-interface-architecture.md (sha256 f8cd050f1c78); verify current state in [[active-phase]] -->
# Chat Interface Architecture

- User message rendered in right-aligned card with surface #222428 and border #383C42.
- User message width clamps between 160 px and 78% of feed width based on content.
- User message header displays You label, formatted timestamp, and copy button.
- Embedded image attachments render with preview thumbnail, safe filename, and size.
- Embedded PDF attachments render with document icon, page count, and size in KB.
- Long user messages support Show more and Show less expand toggle.
- Assistant message renders directly on canvas without artificial enclosing card.
- Assistant header displays RetinaCare Clinical Assistant with status badge and time.
- Supported message lifecycle states are Queued, Streaming, Completed, Cancelled, Failed.
- Streaming state renders active typing indicator cursor at the end of text stream.
- Cancelled state strictly preserves previously accumulated tokens.
- Failed state renders error banner with warning icon and specific error detail.
- Assistant action toolbar includes Copy button, Retry button, and Stop button.
- Retry button re-submits the previous user prompt and attachments to the service.
- Stop button cancels the active request via nonblocking channel event.
- Composer send button dynamically transitions to Stop button during active generation.
- ChatService trait defines typed submit and cancel operations.
- UnavailableChatService provides truthful failure event when Freebuff backend is absent.
- Auto-scroll sticks to bottom only when user is near bottom, pausing on upward scroll.
- All 53 unit and integration tests pass cleanly under cargo test.
