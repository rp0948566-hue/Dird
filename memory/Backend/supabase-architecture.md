---
title: Supabase private chat persistence (design)
type: design
status: PARTIAL
verified_on: 2026-10-29
---
# Supabase chat persistence

Stage A done and the chat migration APPLIED (2026-10-28, user-approved): RLS forced on 5 tables, private bucket, security advisors clean. No Rust yet; isolation tests pending the user's two synthetic accounts. Full design, security model, auth lifecycle, conflict rules and test plan: `docs/features/SUPABASE-INTEGRATION/README.md`. SQL: `supabase/migrations/20261028120000_chat_persistence.sql`; patient schema design (unapplied): `supabase/design/patient_cloud_design.sql`.

- Project `ebflochdtajcflmlhgqa` reachable, empty `public` schema. Publishable key only; refresh token in the Linux Secret Service, no plaintext fallback.
- RLS forced on every table, owner-scoped via `auth.uid()`; private Storage bucket; `data_class` blocks clinical data in the schema.
- SQLCipher stays authoritative; sync is an outbox with idempotent upserts. Offline, Syncing, Connected, Authentication Required states are real.
- Next: approval of the SQL, then Stage B (Rust auth, keyring, chat store, sync) and Stage C (Previous Chats wiring). Needs two synthetic accounts created by the user. See [[Backend/security]], [[GUI/previous-chats-folder-system]], [[Handoffs/next-actions]].

## Phase 5 client (2026-10-29)
- Client: `engine/supabase.rs` (publishable key only, HTTPS or loopback). Refresh token in OS keyring only (`engine/keystore.rs`), no file fallback.
- Outbox and pull: `engine/cloud.rs`. Only general text chats with no attachments or analysis cards are eligible (enforced in `chats.rs`).
- Live two-user RLS test: BLOCKED until user supplies two test accounts.
