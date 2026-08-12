-- Le tableau manuel : blocs et references (issue #29, chantier 2 de la
-- conception, sans issues ni fermetures — elles arrivent avec #30 et #32).
--
-- Un bloc est l'unite du tableau : titre, type, statut, emplacement optionnel
-- (§4.1 de la conception). Les colonnes `prd_*` existent des maintenant pour
-- ne pas remigrer la table quand la lecture de PRD arrivera (#36+), mais rien
-- ne les remplit ici : cette issue ne lit aucun fichier.

create table public.blocs (
  id              uuid primary key default gen_random_uuid(),
  user_id         uuid not null default auth.uid()
                  references auth.users (id) on delete cascade,
  repo_id         uuid not null references public.repos (id) on delete cascade,
  ref             int  not null,                    -- VM-7 : le 7
  type            text not null default 'feature'
                  check (type in ('feature','correction','technique','exploration')),
  titre           text not null,
  statut          text not null default 'todo'
                  check (statut in ('todo','doing','done')),
  version         int  not null default 1,
  -- Emplacement du bloc simple. Null des qu'il est decoupe en issues (#30) :
  -- un bloc ne porte jamais les deux a la fois.
  chemin          text,
  -- Provenance depuis un PRD (#36+). Null = saisi a la main, le seul chemin
  -- que cette issue emprunte.
  prd_cle         text,
  prd_priorite    text,
  prd_a_clarifier boolean not null default false,
  prd_statut      text,
  prd_maj         date,
  prd_valide_le   date,
  prd_absent      boolean not null default false,
  position        int  not null default 0,
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now(),
  unique (repo_id, ref),
  unique (repo_id, prd_cle)
);

create index blocs_repo_id_idx on public.blocs (repo_id);
create index blocs_repo_statut_idx on public.blocs (repo_id, statut);

-- Le compteur de references d'un depot : un VM-7 designera un bloc ou, plus
-- tard, une issue (#30) au meme compteur — c'est ce qui permet a un message
-- de commit de nommer l'un ou l'autre sans que le lecteur ait a savoir lequel.
create table public.compteur_ref (
  repo_id  uuid primary key references public.repos (id) on delete cascade,
  prochain int not null default 1
);

alter table public.blocs enable row level security;
alter table public.compteur_ref enable row level security;

grant select, insert, update, delete on public.blocs to authenticated;
grant select, insert, update, delete on public.compteur_ref to authenticated;
grant all on public.blocs to service_role;
grant all on public.compteur_ref to service_role;

alter publication supabase_realtime add table public.blocs;
alter publication supabase_realtime add table public.compteur_ref;

-- ----------------------------------------------------------------- RLS blocs
-- Calquee sur `modules` (20260803000002) : pas de proprietaire propre, la
-- table suit son depot via `repo_accessible`.
create policy blocs_select_own on public.blocs
  for select using (public.repo_accessible(repo_id));

create policy blocs_insert_own on public.blocs
  for insert with check (public.repo_accessible(repo_id));

create policy blocs_update_own on public.blocs
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id));

create policy blocs_delete_own on public.blocs
  for delete using (public.repo_accessible(repo_id));

-- ------------------------------------------------------------ RLS compteur_ref
create policy compteur_ref_select_own on public.compteur_ref
  for select using (public.repo_accessible(repo_id));

create policy compteur_ref_insert_own on public.compteur_ref
  for insert with check (public.repo_accessible(repo_id));

create policy compteur_ref_update_own on public.compteur_ref
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id));

create policy compteur_ref_delete_own on public.compteur_ref
  for delete using (public.repo_accessible(repo_id));

-- --------------------------------------------------------------- creer_bloc
-- Attribution de reference par increment atomique du compteur, JAMAIS par
-- `max(ref) + 1` : une reference liberee par une suppression ne doit jamais
-- etre reattribuee, sinon un vieux message de commit fermerait une carte
-- neuve (PRD F1, FR-004). Le upsert verrouille la ligne du compteur le temps
-- de la transaction : deux creations concurrentes sur le meme depot ne
-- peuvent donc pas tirer la meme reference.
--
-- Fonction en droits de l'appelant (pas security definer) : c'est la RLS des
-- deux tables ci-dessus qui autorise ou refuse l'ecriture, pas cette fonction.
create or replace function public.creer_bloc(
  p_repo_id uuid,
  p_titre   text,
  p_type    text,
  p_chemin  text
) returns public.blocs
language plpgsql
set search_path = public, pg_catalog
as $$
declare
  v_ref  int;
  v_bloc public.blocs;
begin
  if p_titre is null or btrim(p_titre) = '' then
    raise exception 'il faut un titre';
  end if;

  if p_chemin is null or btrim(p_chemin) = '' then
    raise exception 'il faut un emplacement';
  end if;

  insert into public.compteur_ref (repo_id, prochain)
  values (p_repo_id, 2)
  on conflict (repo_id) do update
    set prochain = public.compteur_ref.prochain + 1
  returning prochain - 1 into v_ref;

  insert into public.blocs (repo_id, ref, type, titre, chemin)
  values (p_repo_id, v_ref, p_type, btrim(p_titre), btrim(p_chemin))
  returning * into v_bloc;

  return v_bloc;
end;
$$;

revoke execute on function public.creer_bloc(uuid, text, text, text) from public, anon;
grant execute on function public.creer_bloc(uuid, text, text, text) to authenticated;
