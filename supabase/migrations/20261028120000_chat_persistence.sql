-- RetinaCare private chat persistence (Stage A, DRAFT - NOT APPLIED).
-- Research prototype. Holds general or synthetic chat data only; see docs/features/SUPABASE-INTEGRATION/README.md.
-- Non-destructive: creates new objects only. Re-running fails on "already exists" instead of overwriting.
-- Review before applying. Every table has RLS enabled and forced; anon has no access.

begin;

-- ---------------------------------------------------------------------------------------------
-- Helper triggers (SECURITY INVOKER, empty search_path)
-- ---------------------------------------------------------------------------------------------

-- Keeps updated_at current. Conversations bump `version` only when title/archive/delete change, so
-- appending messages never invalidates a pending rename (see conflict rules in the design doc).
create function public.rc_conversation_touch()
returns trigger language plpgsql set search_path = '' as $$
begin
  if new.owner_id is distinct from old.owner_id then
    raise exception 'owner_id is immutable';
  end if;
  new.updated_at := clock_timestamp();
  if (new.title, new.archived_at, new.deleted_at) is distinct from (old.title, old.archived_at, old.deleted_at) then
    new.version := old.version + 1;
  else
    new.version := old.version;
  end if;
  return new;
end $$;

create function public.rc_row_touch()
returns trigger language plpgsql set search_path = '' as $$
begin
  if new.owner_id is distinct from old.owner_id then
    raise exception 'owner_id is immutable';
  end if;
  new.updated_at := clock_timestamp();
  new.version := old.version + 1;
  return new;
end $$;

-- Assigns the per-conversation message order under a lock, so concurrent writers never share a seq.
create function public.rc_message_assign_seq()
returns trigger language plpgsql set search_path = '' as $$
begin
  perform pg_advisory_xact_lock(hashtextextended(new.conversation_id::text, 0));
  select coalesce(max(m.seq), 0) + 1 into new.seq
    from public.conversation_messages m where m.conversation_id = new.conversation_id;
  return new;
end $$;

-- Any new message moves the conversation to the top of the list.
create function public.rc_message_after_insert()
returns trigger language plpgsql set search_path = '' as $$
begin
  update public.conversations set updated_at = clock_timestamp() where id = new.conversation_id;
  return new;
end $$;

revoke all on function public.rc_conversation_touch() from public, anon, authenticated;
revoke all on function public.rc_row_touch() from public, anon, authenticated;
revoke all on function public.rc_message_assign_seq() from public, anon, authenticated;
revoke all on function public.rc_message_after_insert() from public, anon, authenticated;

-- ---------------------------------------------------------------------------------------------
-- Tables
-- ---------------------------------------------------------------------------------------------

create table public.conversations (
  id          uuid primary key default gen_random_uuid(),
  owner_id    uuid not null default auth.uid() references auth.users (id) on delete cascade,
  title       text not null check (char_length(title) between 1 and 200),
  -- Never 'clinical': patient-identifiable chat content must not be synced (see design doc).
  data_class  text not null default 'general' check (data_class in ('general', 'synthetic')),
  version     bigint not null default 1,
  created_at  timestamptz not null default clock_timestamp(),
  updated_at  timestamptz not null default clock_timestamp(),
  archived_at timestamptz,
  deleted_at  timestamptz
);
create index conversations_owner_updated_idx on public.conversations (owner_id, updated_at desc);

create table public.conversation_messages (
  id              uuid primary key default gen_random_uuid(),
  conversation_id uuid not null references public.conversations (id) on delete cascade,
  owner_id        uuid not null default auth.uid() references auth.users (id) on delete cascade,
  -- Stable client-generated id: makes retries idempotent (no duplicate messages).
  client_msg_id   uuid not null,
  seq             bigint not null,
  role            text not null check (role in ('user', 'assistant', 'system')),
  content         text not null check (char_length(content) <= 100000),
  created_at      timestamptz not null default clock_timestamp(),
  unique (conversation_id, client_msg_id),
  unique (conversation_id, seq)
);
create index conversation_messages_owner_idx on public.conversation_messages (owner_id);

create table public.conversation_attachments (
  id              uuid primary key default gen_random_uuid(),
  conversation_id uuid not null references public.conversations (id) on delete cascade,
  message_id      uuid references public.conversation_messages (id) on delete set null,
  owner_id        uuid not null default auth.uid() references auth.users (id) on delete cascade,
  storage_path    text not null unique,
  file_name       text not null check (char_length(file_name) between 1 and 255),
  mime_type       text not null check (mime_type in ('image/png', 'image/jpeg', 'application/pdf', 'text/plain')),
  size_bytes      bigint not null check (size_bytes between 1 and 10485760),
  sha256          text not null check (sha256 ~ '^[0-9a-f]{64}$'),
  data_class      text not null default 'general' check (data_class in ('general', 'synthetic')),
  created_at      timestamptz not null default clock_timestamp(),
  -- Path must be <owner>/<conversation>/<file>: ties the object to its owner for storage policies.
  check (storage_path like owner_id::text || '/' || conversation_id::text || '/%'),
  -- Images may carry patient data, so the database only accepts them when explicitly synthetic.
  check (mime_type not like 'image/%' or data_class = 'synthetic')
);
create index conversation_attachments_owner_idx on public.conversation_attachments (owner_id, conversation_id);

create table public.conversation_metadata (
  conversation_id uuid primary key references public.conversations (id) on delete cascade,
  owner_id        uuid not null default auth.uid() references auth.users (id) on delete cascade,
  pinned          boolean not null default false,
  tags            text[] not null default '{}' check (cardinality(tags) <= 20),
  preview         text not null default '' check (char_length(preview) <= 300),
  message_count   integer not null default 0 check (message_count >= 0),
  version         bigint not null default 1,
  updated_at      timestamptz not null default clock_timestamp()
);

create table public.sync_state (
  owner_id       uuid not null default auth.uid() references auth.users (id) on delete cascade,
  device_id      text not null check (char_length(device_id) between 8 and 64),
  last_pulled_at timestamptz,
  last_pushed_at timestamptz,
  version        bigint not null default 1,
  updated_at     timestamptz not null default clock_timestamp(),
  primary key (owner_id, device_id)
);

create trigger conversations_touch before update on public.conversations
  for each row execute function public.rc_conversation_touch();
create trigger conversation_metadata_touch before update on public.conversation_metadata
  for each row execute function public.rc_row_touch();
create trigger sync_state_touch before update on public.sync_state
  for each row execute function public.rc_row_touch();
create trigger conversation_messages_seq before insert on public.conversation_messages
  for each row execute function public.rc_message_assign_seq();
create trigger conversation_messages_bump after insert on public.conversation_messages
  for each row execute function public.rc_message_after_insert();

-- ---------------------------------------------------------------------------------------------
-- Privileges: nothing for anon; only what the app needs for authenticated.
-- ---------------------------------------------------------------------------------------------

revoke all on public.conversations, public.conversation_messages, public.conversation_attachments,
              public.conversation_metadata, public.sync_state from public, anon;
revoke all on public.conversations, public.conversation_messages, public.conversation_attachments,
              public.conversation_metadata, public.sync_state from authenticated;

grant select, insert, update, delete on public.conversations to authenticated;
grant select, insert                 on public.conversation_messages to authenticated;       -- append-only
grant select, insert, delete         on public.conversation_attachments to authenticated;   -- immutable rows
grant select, insert, update         on public.conversation_metadata to authenticated;
grant select, insert, update         on public.sync_state to authenticated;

-- ---------------------------------------------------------------------------------------------
-- Row Level Security: owner-scoped through Supabase Auth identity
-- ---------------------------------------------------------------------------------------------

alter table public.conversations            enable row level security;
alter table public.conversation_messages    enable row level security;
alter table public.conversation_attachments enable row level security;
alter table public.conversation_metadata    enable row level security;
alter table public.sync_state               enable row level security;
alter table public.conversations            force row level security;
alter table public.conversation_messages    force row level security;
alter table public.conversation_attachments force row level security;
alter table public.conversation_metadata    force row level security;
alter table public.sync_state               force row level security;

create policy conversations_select on public.conversations for select to authenticated
  using (owner_id = (select auth.uid()));
create policy conversations_insert on public.conversations for insert to authenticated
  with check (owner_id = (select auth.uid()));
create policy conversations_update on public.conversations for update to authenticated
  using (owner_id = (select auth.uid())) with check (owner_id = (select auth.uid()));
create policy conversations_delete on public.conversations for delete to authenticated
  using (owner_id = (select auth.uid()));

-- Messages: the row AND its parent conversation must belong to the caller; no update or delete policy.
create policy messages_select on public.conversation_messages for select to authenticated
  using (owner_id = (select auth.uid()));
create policy messages_insert on public.conversation_messages for insert to authenticated
  with check (
    owner_id = (select auth.uid())
    and exists (
      select 1 from public.conversations c
      where c.id = conversation_id and c.owner_id = (select auth.uid()) and c.deleted_at is null
    )
  );

create policy attachments_select on public.conversation_attachments for select to authenticated
  using (owner_id = (select auth.uid()));
create policy attachments_insert on public.conversation_attachments for insert to authenticated
  with check (
    owner_id = (select auth.uid())
    and exists (
      select 1 from public.conversations c
      where c.id = conversation_id and c.owner_id = (select auth.uid()) and c.deleted_at is null
    )
  );
create policy attachments_delete on public.conversation_attachments for delete to authenticated
  using (owner_id = (select auth.uid()));

create policy metadata_select on public.conversation_metadata for select to authenticated
  using (owner_id = (select auth.uid()));
create policy metadata_insert on public.conversation_metadata for insert to authenticated
  with check (
    owner_id = (select auth.uid())
    and exists (select 1 from public.conversations c where c.id = conversation_id and c.owner_id = (select auth.uid()))
  );
create policy metadata_update on public.conversation_metadata for update to authenticated
  using (owner_id = (select auth.uid())) with check (owner_id = (select auth.uid()));

create policy sync_state_select on public.sync_state for select to authenticated
  using (owner_id = (select auth.uid()));
create policy sync_state_insert on public.sync_state for insert to authenticated
  with check (owner_id = (select auth.uid()));
create policy sync_state_update on public.sync_state for update to authenticated
  using (owner_id = (select auth.uid())) with check (owner_id = (select auth.uid()));

-- ---------------------------------------------------------------------------------------------
-- Private Storage bucket for attachments. Never public.
-- ---------------------------------------------------------------------------------------------

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values ('chat-attachments', 'chat-attachments', false, 10485760,
        array['image/png', 'image/jpeg', 'application/pdf', 'text/plain'])
on conflict (id) do nothing;

-- Objects live under <auth.uid()>/<conversation_id>/<file>; only the owner folder is reachable.
create policy chat_attachments_select on storage.objects for select to authenticated
  using (bucket_id = 'chat-attachments' and (storage.foldername(name))[1] = (select auth.uid())::text);
create policy chat_attachments_insert on storage.objects for insert to authenticated
  with check (bucket_id = 'chat-attachments' and (storage.foldername(name))[1] = (select auth.uid())::text);
create policy chat_attachments_delete on storage.objects for delete to authenticated
  using (bucket_id = 'chat-attachments' and (storage.foldername(name))[1] = (select auth.uid())::text);

commit;
