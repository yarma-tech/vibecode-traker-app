-- Le plan d'un repo (issue #3).
--
-- Un repo cartographie, un dossier par ligne dans `modules`, avec ses lignes
-- de code cumulees. Le chemin absolu ne quitte jamais la machine : seule son
-- empreinte voyage, et tous les chemins de modules sont relatifs a la racine.

create table public.repos (
  id             uuid primary key default gen_random_uuid(),
  -- Rempli a partir du jeton de l'appelant : le daemon ne connait que sa
  -- machine, pas l'identifiant du compte, et n'a pas a le connaitre.
  user_id        uuid not null default auth.uid()
                 references auth.users (id) on delete cascade,
  machine_id     uuid not null references public.machines (id) on delete cascade,
  name           text not null,
  -- Empreinte du chemin absolu. Le chemin lui-meme contient le nom de
  -- l'utilisateur du systeme : il reste sur la machine.
  root_hash      text not null,
  remote_owner   text,
  remote_url     text,
  account_label  text,
  default_branch text,
  current_branch text,
  loc_total      int  not null default 0,
  file_count     int  not null default 0,
  scanned_at     timestamptz not null default now(),
  unique (machine_id, root_hash)
);

create index repos_user_id_idx on public.repos (user_id);
create index repos_machine_id_idx on public.repos (machine_id);

create table public.modules (
  id          uuid primary key default gen_random_uuid(),
  repo_id     uuid not null references public.repos (id) on delete cascade,
  -- Relatif a la racine du repo. La racine elle-meme porte le chemin vide.
  path        text not null,
  parent_path text,
  depth       int  not null,
  loc         int  not null default 0,
  file_count  int  not null default 0,
  updated_at  timestamptz not null default now(),
  unique (repo_id, path)
);

create index modules_repo_id_idx on public.modules (repo_id);
create index modules_parent_idx on public.modules (repo_id, parent_path);

alter table public.repos enable row level security;
alter table public.modules enable row level security;

grant select, insert, update, delete on public.repos to authenticated;
grant select, insert, update, delete on public.modules to authenticated;
grant all on public.repos to service_role;
grant all on public.modules to service_role;

alter publication supabase_realtime add table public.repos;
alter publication supabase_realtime add table public.modules;

-- ----------------------------------------------------------------- RLS repos
-- Une machine cartographie ses propres repos : contrairement a `machines`,
-- elle a donc le droit d'inserer. Mais uniquement pour elle-meme, et
-- uniquement tant qu'elle n'est pas revoquee.
create or replace function public.machine_active(p_machine_id uuid)
returns boolean
language sql
stable
set search_path = public, pg_catalog
as $$
  select
    public.machine_du_jeton() is null
    or (
      public.machine_du_jeton() = p_machine_id
      and exists (
        select 1 from public.machines m
        where m.id = p_machine_id and m.revoked_at is null
      )
    );
$$;

create policy repos_select_own on public.repos
  for select using (
    auth.uid() = user_id and public.machine_active(machine_id)
  );

create policy repos_insert_own on public.repos
  for insert with check (
    auth.uid() = user_id and public.machine_active(machine_id)
  );

create policy repos_update_own on public.repos
  for update using (
    auth.uid() = user_id and public.machine_active(machine_id)
  ) with check (
    auth.uid() = user_id and public.machine_active(machine_id)
  );

create policy repos_delete_own on public.repos
  for delete using (
    auth.uid() = user_id and public.machine_active(machine_id)
  );

-- --------------------------------------------------------------- RLS modules
-- Les modules n'ont pas de proprietaire propre : ils suivent leur repo.
create or replace function public.repo_accessible(p_repo_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public, pg_catalog
as $$
  select exists (
    select 1
    from public.repos r
    where r.id = p_repo_id
      and r.user_id = auth.uid()
      and public.machine_active(r.machine_id)
  );
$$;

create policy modules_select_own on public.modules
  for select using (public.repo_accessible(repo_id));

create policy modules_insert_own on public.modules
  for insert with check (public.repo_accessible(repo_id));

create policy modules_update_own on public.modules
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id));

create policy modules_delete_own on public.modules
  for delete using (public.repo_accessible(repo_id));
