-- Les worktrees en surimpression (issue #6).
--
-- Un canal a part, strictement separe de l'etat d'activite : un module peut
-- etre `ecrit` (ambre) ET porter un worktree (hachure verte) en meme temps. Le
-- daemon lance `git worktree list --porcelain` par repo toutes les 30 s et
-- pousse la liste des worktrees ouverts ; la base la reconcilie.
--
-- Le worktree principal (le depot lui-meme) n'entre jamais ici : le daemon ne
-- pousse que les worktrees LIES. Un chemin absolu ne quitte pas la machine :
-- `path` ne porte que le nom du dossier du worktree.

create table public.worktrees (
  id          uuid primary key default gen_random_uuid(),
  -- Rempli a partir du jeton de l'appelant, comme repos et modules.
  user_id     uuid not null default auth.uid()
              references auth.users (id) on delete cascade,
  repo_id     uuid not null references public.repos (id) on delete cascade,
  -- Nom du dossier du worktree, relatif : le chemin absolu reste sur la machine.
  path        text not null,
  -- La branche du worktree, celle que portera le badge. Jamais la branche
  -- principale du repo : c'est tout l'interet du canal.
  branch      text not null,
  -- Premier releve ou ce worktree est apparu ouvert.
  detected_at timestamptz not null default now(),
  -- Renseigne quand le worktree disparait d'un releve : il est ferme. Un
  -- worktree ferme reste en base, mais sort de la surimpression.
  closed_at   timestamptz,
  -- Deux releves du meme worktree se rapprochent par son dossier : rouvrir un
  -- worktree ferme reutilise sa ligne plutot que d'en empiler une seconde.
  unique (repo_id, path)
);

create index worktrees_repo_id_idx on public.worktrees (repo_id);
create index worktrees_ouverts_idx on public.worktrees (repo_id) where closed_at is null;

alter table public.worktrees enable row level security;

-- Le service_role est explicite : sans ce grant, les aides de test a la cle
-- service prendraient un 403 sur une table neuve.
grant select, insert, update, delete on public.worktrees to authenticated;
grant all on public.worktrees to service_role;

-- La surimpression apparait et disparait sur signal : l'ouverture (INSERT) comme
-- la fermeture (UPDATE de closed_at) doivent atteindre l'ecran sans attendre.
alter publication supabase_realtime add table public.worktrees;

-- ------------------------------------------------------------------- RLS
-- Les worktrees suivent leur repo, qui suit sa machine : revoquer une machine
-- coupe l'ecriture a la milliseconde.
create policy worktrees_select_own on public.worktrees
  for select using (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy worktrees_insert_own on public.worktrees
  for insert with check (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy worktrees_update_own on public.worktrees
  for update using (auth.uid() = user_id and public.repo_accessible(repo_id))
  with check (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy worktrees_delete_own on public.worktrees
  for delete using (auth.uid() = user_id and public.repo_accessible(repo_id));

-- ------------------------------------------------------ noter les worktrees
-- Le daemon envoie la liste COMPLETE des worktrees ouverts d'un repo. La base
-- la reconcilie en deux temps : rouvrir ou creer ce qui est present, fermer ce
-- qui a disparu. Ainsi ouvrir un worktree le fait apparaitre, le fermer le fait
-- disparaitre, sans que le daemon ait a suivre les transitions lui-meme.
create or replace function public.noter_worktrees(
  p_repo_id   uuid,
  p_worktrees jsonb
)
returns void
language plpgsql
security invoker
set search_path = public, pg_catalog
as $$
begin
  -- Rouvre ou cree chaque worktree present. Rouvrir remet la pendule a l'heure :
  -- un worktree ferme puis revu redemarre son `detected_at`.
  insert into public.worktrees (repo_id, path, branch)
  select p_repo_id, w->>'path', w->>'branch'
  from jsonb_array_elements(coalesce(p_worktrees, '[]'::jsonb)) as w
  on conflict (repo_id, path) do update set
    branch      = excluded.branch,
    detected_at = case
                    when public.worktrees.closed_at is not null then now()
                    else public.worktrees.detected_at
                  end,
    closed_at   = null;

  -- Ferme ceux que ce releve ne mentionne plus. Une liste vide ferme donc tout :
  -- c'est ce qui fait disparaitre le dernier worktree du plan.
  update public.worktrees
  set closed_at = now()
  where repo_id = p_repo_id
    and closed_at is null
    and path not in (
      select w->>'path'
      from jsonb_array_elements(coalesce(p_worktrees, '[]'::jsonb)) as w
    );
end;
$$;

grant execute on function public.noter_worktrees(uuid, jsonb) to authenticated;

-- ------------------------------------------------------ lire les worktrees
-- Les worktrees ouverts d'un repo, ou de tous ceux que l'appelant peut voir.
-- L'ecran lit ceci pour poser la hachure et le badge de branche.
create or replace function public.worktrees_ouverts(p_repo_id uuid default null)
returns table (
  repo_id     uuid,
  path        text,
  branch      text,
  detected_at timestamptz
)
language sql
stable
security invoker
set search_path = public, pg_catalog
as $$
  select w.repo_id, w.path, w.branch, w.detected_at
  from public.worktrees w
  where w.closed_at is null
    and (p_repo_id is null or w.repo_id = p_repo_id)
  order by w.detected_at desc, w.branch;
$$;

grant execute on function public.worktrees_ouverts(uuid) to authenticated, anon;
