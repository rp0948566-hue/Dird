-- RetinaCare cloud-compatible clinical schema (Stage A DESIGN ONLY - NOT A MIGRATION, NOT APPLIED).
-- Kept outside supabase/migrations on purpose. Local SQLCipher stays the authoritative protected copy.
-- Every table is limited by CHECK to data_class = 'synthetic', so real patient data cannot be inserted
-- even with a valid session. Relaxing that needs a separate, reviewed migration after privacy, consent,
-- security and regulatory requirements are met. No pixels, images or PDF bytes are ever stored here.

create table public.cloud_patients (
  id           uuid primary key default gen_random_uuid(),
  owner_id     uuid not null default auth.uid() references auth.users (id) on delete cascade,
  local_id     bigint not null,
  patient_code text not null check (char_length(patient_code) between 1 and 64),
  display_name text not null check (char_length(display_name) between 1 and 200),
  data_class   text not null default 'synthetic' check (data_class = 'synthetic'),
  version      bigint not null default 1,
  created_at   timestamptz not null default clock_timestamp(),
  updated_at   timestamptz not null default clock_timestamp(),
  unique (owner_id, local_id)
);

create table public.cloud_examinations (
  id          uuid primary key default gen_random_uuid(),
  owner_id    uuid not null default auth.uid() references auth.users (id) on delete cascade,
  patient_id  uuid not null references public.cloud_patients (id) on delete cascade,
  local_id    bigint not null,
  eye         text not null check (eye in ('', 'left', 'right')),
  examined_at timestamptz not null,
  data_class  text not null default 'synthetic' check (data_class = 'synthetic'),
  version     bigint not null default 1,
  updated_at  timestamptz not null default clock_timestamp(),
  unique (owner_id, local_id)
);

create table public.cloud_examination_events (
  id             uuid primary key default gen_random_uuid(),
  owner_id       uuid not null default auth.uid() references auth.users (id) on delete cascade,
  examination_id uuid not null references public.cloud_examinations (id) on delete cascade,
  event_type     text not null check (char_length(event_type) between 1 and 64),
  status         text not null default '' check (char_length(status) <= 64),
  occurred_at    timestamptz not null,
  data_class     text not null default 'synthetic' check (data_class = 'synthetic')
);

-- Analysis metadata only: grade, state, hashes, timings. No masks and no images.
create table public.cloud_analysis_metadata (
  examination_id      uuid primary key references public.cloud_examinations (id) on delete cascade,
  owner_id            uuid not null default auth.uid() references auth.users (id) on delete cascade,
  state               text not null,
  technical_grade     smallint check (technical_grade between 0 and 4),
  research_only       boolean not null default true check (research_only),
  image_sha256        text check (image_sha256 ~ '^[0-9a-f]{64}$'),
  model_sha256        text[] not null default '{}',
  total_seconds       real,
  data_class          text not null default 'synthetic' check (data_class = 'synthetic')
);

-- Report records: reference and hash only, never the PDF bytes.
create table public.cloud_reports (
  id             uuid primary key default gen_random_uuid(),
  owner_id       uuid not null default auth.uid() references auth.users (id) on delete cascade,
  examination_id uuid not null references public.cloud_examinations (id) on delete cascade,
  report_sha256  text not null check (report_sha256 ~ '^[0-9a-f]{64}$'),
  generated_at   timestamptz not null,
  data_class     text not null default 'synthetic' check (data_class = 'synthetic')
);

-- Audit references: ids and statuses only, mirroring the local append-only ledger.
create table public.cloud_audit_refs (
  id             uuid primary key default gen_random_uuid(),
  owner_id       uuid not null default auth.uid() references auth.users (id) on delete cascade,
  examination_id uuid references public.cloud_examinations (id) on delete set null,
  local_event_id bigint not null,
  event_type     text not null,
  status         text not null default '',
  recorded_at    timestamptz not null,
  data_class     text not null default 'synthetic' check (data_class = 'synthetic'),
  unique (owner_id, local_event_id)
);

-- Stored-mask metadata: manifest hash and per-mask hashes. Pixels never leave SQLCipher.
create table public.cloud_mask_metadata (
  examination_id      uuid primary key references public.cloud_examinations (id) on delete cascade,
  owner_id            uuid not null default auth.uid() references auth.users (id) on delete cascade,
  manifest_sha256     text not null check (manifest_sha256 ~ '^[0-9a-f]{64}$'),
  source_image_sha256 text not null check (source_image_sha256 ~ '^[0-9a-f]{64}$'),
  image_width         integer not null,
  image_height        integer not null,
  kinds               text[] not null default array['ma','he','ex','cws','vessel'],
  created_at          timestamptz not null,
  data_class          text not null default 'synthetic' check (data_class = 'synthetic')
);

-- RLS (same owner-scoped pattern as chat persistence), applied to every table above:
--   alter table public.<t> enable row level security; alter table public.<t> force row level security;
--   revoke all on public.<t> from public, anon;  grant select, insert, update, delete to authenticated;
--   create policy <t>_owner on public.<t> for all to authenticated
--     using (owner_id = (select auth.uid())) with check (owner_id = (select auth.uid()));
-- Child tables additionally require the parent row to belong to the caller (same exists() check as
-- conversation_messages). Final per-operation policies are written when this design is approved.
