# Supabase private chat persistence - design (Stage A, 2026-10-28)

Status: migration `chat_persistence` APPLIED to the cloud project on 2026-10-28 (user-approved); no Rust written yet. Research prototype. Cloud holds general or synthetic chat data only. Project: `ebflochdtajcflmlhgqa` ("Hackthone", eu-central-1, ACTIVE_HEALTHY, empty `public` schema at 2026-10-28).
SQL: `supabase/migrations/20261028120000_chat_persistence.sql` (proposed migration) and `supabase/design/patient_cloud_design.sql` (design only, not a migration).

## Architecture
- SQLCipher (existing) stays the authoritative local store. New local tables hold conversations, messages, attachments metadata, and an outbox of pending sync operations; all writes are one local transaction.
- A background sync worker (own thread, never the egui thread, never the ModelService thread) pushes the outbox to Supabase PostgREST and pulls changes by `updated_at` cursor.
- Client uses only the project URL and the PUBLISHABLE key (read from verified local config, fetched from the project, never a secret/service-role key or DB password). All authorization is Supabase Auth JWT + RLS.
- Status shown in the GUI is real: Offline (no network or request failing), Syncing (outbox non-empty or request in flight), Connected (empty outbox, last request ok), Authentication Required (no valid session).

## Authentication lifecycle
1. First launch: config file in the OS app-data dir (`~/.local/share/RetinaCare/config.json`: project URL, setup-complete flag, device id; no secrets). Local encrypted DB is created once at the existing default path and reused afterwards.
2. Sign in or register with Supabase Auth (email + password, over HTTPS). The password is never stored or logged.
3. The refresh token goes only to the Linux Secret Service (`gnome-keyring` is running here). Access token stays in memory. No plaintext fallback: if the keyring is missing, the app asks for sign-in on each launch.
4. Later launches: read the refresh token from the keyring, refresh the session, load the user's data. Expired, revoked or unrefreshable session = "Authentication Required"; local SQLCipher data stays available offline, cloud sync pauses.
5. Local DB unlock stays the existing SQLCipher password. Cloud session and local unlock are independent.
6. Sign out deletes the keyring entry and the in-memory tokens, keeps local data.
Decision needed later: whether the SQLCipher password should also be offered via the keyring. Not planned.

## Schema (summary of the SQL)
`conversations` (owner, title 1-200, data_class general|synthetic, version, created/updated/archived/deleted), `conversation_messages` (append-only, `client_msg_id` unique per conversation, server-assigned `seq` under an advisory lock, role user|assistant|system), `conversation_attachments` (private bucket path `<uid>/<conversation>/<file>`, mime allowlist, 10 MiB cap, sha256; image types only when `data_class = 'synthetic'`), `conversation_metadata` (pinned, tags, preview, count, version), `sync_state` (per owner+device cursors). Private bucket `chat-attachments` (public = false).

## Security model
- RLS enabled AND forced on every table; policies are `to authenticated` with `owner_id = (select auth.uid())`; `anon` has no grants.
- Messages: select and insert only (no update/delete); insert needs the parent conversation to be the caller's and not deleted. `owner_id` is immutable (trigger). Arbitrary record ids from another user return no rows.
- Storage: policies allow only objects whose first folder equals `auth.uid()`; bucket is private.
- No clinical data: `data_class` can never be 'clinical' in chat tables; the patient design adds `CHECK (data_class = 'synthetic')` on every table. The GUI keeps cloud sync off for patient records, and attachment upload off unless the user marks the file synthetic.
- Secrets: none in the repo, logs or binary; errors show a generic reason, never tokens or passwords.
- Known limits: a user who holds a valid session can read their own rows (by design); RLS does not protect against a compromised user account; Auth settings (email confirmation, password policy, rate limits) are dashboard config to review.

## Sync and conflict rules
- Idempotent: every message has a client-generated `client_msg_id`; push is an upsert that ignores duplicates, so retries never duplicate. Retry with exponential backoff and jitter, capped; failures stay in the outbox.
- Messages are append-only. Order is the server `seq`; concurrent offline devices merge by union of `client_msg_id`, then take server order (local display order is by `created_at` until the server confirms).
- Conversation rename/archive/delete use the `version` column (compare-and-set): if the version moved, the client re-reads and applies: delete beats rename; archive and rename merge (independent fields); two renames = last writer by server time wins and the loser is kept in the local conflict log.
- Delete is a soft tombstone (`deleted_at`) synced to all devices; hard delete happens after a retention window, owner only. Cascade removes messages and attachment rows.
- Pull uses `updated_at > cursor` per device (`sync_state`); tombstones are included.

## Test plan (needs the user's two synthetic accounts)
Unit/integration without cloud: outbox idempotency, ordering, backoff, conflict rules, keyring failure path (no plaintext), error redaction, SQLCipher restart. Live: two-user isolation via arbitrary ids (select, insert into other user's conversation, storage path of the other user), anon denied, offline queue replay, duplicate retry, session expiry, then the GUI roundtrip: save, close, relaunch, restore. Supabase advisors (security and performance) run after any approved apply.

## Not covered by this stage
Rust code, GUI wiring, applying SQL, live tests. The SQL is unexecuted: syntax and RLS behaviour are unverified until applied (a local Postgres was not available: docker needs privileges here). Storage policy creation may need the dashboard SQL editor if the migration role cannot own `storage.objects` policies.

## Approval needed
Apply migration 20261028120000 to the empty project as written? Then Stages B and C follow. `patient_cloud_design.sql` stays unapplied.

## Applied result (2026-10-28)
Migration applied via the Supabase connector without the outer begin/commit (the tool wraps it). Verified live: 5 tables with RLS enabled and forced, 15 table policies plus 3 storage policies (all `authenticated`), bucket `chat-attachments` private (10 MiB limit), no `anon` grants. Security advisors: 0 findings. Performance advisors: INFO only - 3 foreign keys without a covering index (`conversation_attachments.conversation_id`, `conversation_attachments.message_id`, `conversation_metadata.owner_id`; candidate follow-up migration) and 3 unused indexes (expected on an empty database). Cross-user isolation is NOT yet tested: it needs the two synthetic accounts the user will create.
