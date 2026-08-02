-- Table machines : un poste de developpement appaire.
-- Source de verite du contrat de donnees (ADR 0001) : les structures Rust
-- et les types TypeScript descendent de ce fichier, jamais l'inverse.

create table public.machines (
  id           uuid primary key default gen_random_uuid(),
  user_id      uuid not null references auth.users (id) on delete cascade,
  label        text not null,
  platform     text,
  paired_at    timestamptz not null default now(),
  last_seen_at timestamptz,
  revoked_at   timestamptz
);

create index machines_user_id_idx on public.machines (user_id);

alter table public.machines enable row level security;

-- Droits sur la table. La RLS filtre les lignes, les grants ouvrent la porte :
-- il faut les deux. `anon` n'obtient rien, aucune lecture sans session.
grant select, insert, update, delete on public.machines to authenticated;
grant all on public.machines to service_role;

-- Un utilisateur ne voit et ne touche que ses propres machines.
create policy machines_select_own on public.machines
  for select using (auth.uid() = user_id);

create policy machines_insert_own on public.machines
  for insert with check (auth.uid() = user_id);

create policy machines_update_own on public.machines
  for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy machines_delete_own on public.machines
  for delete using (auth.uid() = user_id);

-- Le web s'abonne aux changements pour afficher "a jour, il y a N s".
alter publication supabase_realtime add table public.machines;
