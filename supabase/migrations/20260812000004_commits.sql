-- La fermeture par reference (issue #32, chantier 3 de la conception, §5).
--
-- « Termine » ne veut dire qu'une chose : un commit LOCAL a nomme le travail
-- (PRD, "Deux signaux, deux portees"). Pas fusionne, pas deploye - le daemon
-- lit le disque, pas la forge. Pas de `commit_paths` : les chemins d'un
-- commit ne ferment jamais rien (FR-013), ils n'ont jamais servi qu'a router
-- une fermeture qui ne route plus par chemin depuis que la reference existe.

-- `commits.rs` a sa cadence propre (30 s daemon, entre les journaux et la
-- cartographie) et doit savoir ou il s'est arrete pour ne jamais rejouer le
-- passe (PRD, risques : « mieux vaut rater une fermeture que d'en inventer
-- une »).
alter table public.repos add column last_commit_sha text;

create table public.commits (
  id          uuid primary key default gen_random_uuid(),
  repo_id     uuid not null references public.repos (id) on delete cascade,
  sha         text not null,
  -- Le message part en clair : il le fait deja pour les commits affiches
  -- ailleurs, et c'est lui qui porte la reference qui ferme (PRD §7.1, liste
  -- fermee de ce qui quitte la machine).
  message     text not null,
  branch      text,
  authored_at timestamptz not null,
  created_at  timestamptz not null default now(),
  unique (repo_id, sha)
);

create index commits_repo_id_idx on public.commits (repo_id);

-- Le commit qui a ferme chaque version d'un travail (FR-024). Une seule des
-- deux cibles est renseignee : une reference designe un bloc simple ou une
-- issue, jamais les deux (elles puisent au meme compteur, §4.2).
create table public.fermetures (
  bloc_id   uuid references public.blocs (id)  on delete cascade,
  issue_id  uuid references public.issues (id) on delete cascade,
  commit_id uuid not null references public.commits (id) on delete cascade,
  version   int  not null,
  ferme_le  timestamptz not null default now(),
  check (num_nonnulls(bloc_id, issue_id) = 1)
);

create index fermetures_bloc_idx   on public.fermetures (bloc_id);
create index fermetures_issue_idx  on public.fermetures (issue_id);
create index fermetures_commit_idx on public.fermetures (commit_id);

-- Sans cle primaire, Postgres n'a pas de REPLICA IDENTITY par defaut : la
-- publication realtime refuse alors toute suppression qui atteint cette
-- table (par exemple la cascade depuis `blocs` quand un bloc est supprime),
-- avec "cannot delete ... because it does not have a replica identity".
-- `fermetures` reste volontairement sans identifiant propre (§5.1 de la
-- conception) - la ligne entiere en tient lieu.
alter table public.fermetures replica identity full;

alter table public.commits enable row level security;
alter table public.fermetures enable row level security;

-- Sans les grants, PostgREST repond "permission denied" malgre des policies
-- justes (CONTRIBUTING.md).
grant select, insert, update, delete on public.commits to authenticated;
grant all on public.commits to service_role;

-- `fermetures` est en select seulement pour authenticated (§9 de la
-- conception) : aucun insert/update/delete n'est accorde ici, la seule porte
-- d'ecriture est `fermer_par_reference` plus bas, en security definer - son
-- proprietaire (postgres) possede deja la table, il n'a pas besoin de ces
-- grants pour y ecrire.
grant select on public.fermetures to authenticated;
grant all on public.fermetures to service_role;

alter publication supabase_realtime add table public.commits;
alter publication supabase_realtime add table public.fermetures;

-- -------------------------------------------------------------- RLS commits
-- Identique en tout point a `blocs` et `issues` : la table suit son depot
-- via `repo_accessible`, quatre policies.
create policy commits_select_own on public.commits
  for select using (public.repo_accessible(repo_id));

create policy commits_insert_own on public.commits
  for insert with check (public.repo_accessible(repo_id));

create policy commits_update_own on public.commits
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id));

create policy commits_delete_own on public.commits
  for delete using (public.repo_accessible(repo_id));

-- ------------------------------------------------------------ RLS fermetures
-- Ni bloc_id ni issue_id ne porte directement repo_id, et le check
-- `num_nonnulls` interdit de toute facon de dupliquer la colonne : la
-- visibilite se joint donc a travers celle des deux cibles qui est
-- renseignee.
create policy fermetures_select_own on public.fermetures
  for select using (
    exists (
      select 1 from public.blocs b
      where b.id = fermetures.bloc_id and public.repo_accessible(b.repo_id)
    )
    or exists (
      select 1 from public.issues i
      where i.id = fermetures.issue_id and public.repo_accessible(i.repo_id)
    )
  );

-- ------------------------------------------------------------ fermer_par_reference
-- Ferme exactement ce que le message du commit designe par sa reference
-- (FR-012), jamais par les chemins qu'il touche - il n'y en a d'ailleurs
-- aucun ici a lire (FR-013). Le motif est `VM-([0-9]+)`, JAMAIS
-- `#([0-9]+)` : les messages de ce depot contiennent deja `feat(#7):` et
-- `Merge pull request #17`, qui designent des issues GitHub, pas des
-- travaux du tableau (FR-016).
--
-- Attention a la difference entre le SET et le RETURNING d'un UPDATE : les
-- expressions du SET (le `case when statut = 'done' ...`) lisent toutes la
-- ligne AVANT l'ecriture, jamais entre elles - c'est ce qui permet a la
-- meme instruction de lire `doing` et `version + 1` ensemble pour une
-- reouverture. Le RETURNING, lui, rend la ligne APRES l'ecriture : c'est ce
-- qu'on relit dans `v_statut` pour decider si CETTE reference vient de
-- FERMER (elle entre dans `fermetures`) ou de ROUVRIR (elle n'y entre pas -
-- `fermetures` conserve des cloture, pas des reprises). Les deux moities
-- sont prouvees par des tests plutot que par ce commentaire seul.
create or replace function public.fermer_par_reference(p_commit_id uuid) returns void
language plpgsql security definer set search_path = public, pg_catalog as $$
declare
  v_repo_id uuid;
  v_message text;
  v_ref     int;
  v_id      uuid;
  v_version int;
  v_statut  text;
begin
  select repo_id, message into v_repo_id, v_message
    from public.commits where id = p_commit_id;

  if v_repo_id is null then
    return;
  end if;

  for v_ref in
    select distinct (regexp_matches(v_message, 'VM-([0-9]+)', 'g'))[1]::int
  loop
    -- Une reference designe une issue OU un bloc simple, jamais les deux
    -- (meme compteur, §4.2) : on tente l'issue d'abord.
    update public.issues
       set statut     = case when statut = 'done' then 'doing' else 'done' end,
           version    = case when statut = 'done' then version + 1 else version end,
           updated_at = now()
     where repo_id = v_repo_id and ref = v_ref
    returning id, version, statut into v_id, v_version, v_statut;

    if found then
      if v_statut = 'done' then
        insert into public.fermetures (issue_id, commit_id, version)
        values (v_id, p_commit_id, v_version);
      end if;
      continue;
    end if;

    -- ... sinon un bloc SIMPLE : un bloc decoupe ne se ferme jamais
    -- directement, il derive de ses issues (§4.4 de la conception). La
    -- clause `not exists` ecarte silencieusement ce cas - exactement comme
    -- une reference inconnue (FR-015) - plutot que de heurter le garde-fou
    -- `bloc_statut_protege` (issue #30) avec une ecriture directe.
    update public.blocs
       set statut     = case when statut = 'done' then 'doing' else 'done' end,
           version    = case when statut = 'done' then version + 1 else version end,
           updated_at = now()
     where repo_id = v_repo_id and ref = v_ref
       and not exists (select 1 from public.issues where bloc_id = blocs.id)
    returning id, version, statut into v_id, v_version, v_statut;

    if found and v_statut = 'done' then
      insert into public.fermetures (bloc_id, commit_id, version)
      values (v_id, p_commit_id, v_version);
    end if;

    -- Ni l'un ni l'autre n'a matche : reference inconnue (ou bloc decoupe),
    -- ignoree sans bruit (FR-015). La boucle passe a la reference suivante.
  end loop;
end $$;

revoke execute on function public.fermer_par_reference(uuid) from public, anon;
grant execute on function public.fermer_par_reference(uuid) to authenticated;

-- ------------------------------------------------------------------ ingerer_commit
-- Le point d'entree du daemon (`commits.rs`, cadence 30 s) : une seule
-- transaction pour l'insertion et la fermeture qu'elle declenche. Reingerer
-- un commit deja connu (redemarrage, reseau coupe en cours de route) ne doit
-- jamais refermer une seconde fois - seule une insertion qui cree
-- REELLEMENT une ligne appelle `fermer_par_reference` (§5.2 de la
-- conception).
--
-- Droits de l'appelant (pas security definer) : c'est la RLS de `commits`
-- qui autorise ou refuse l'insertion, comme pour `creer_bloc` et
-- `creer_issue`.
create or replace function public.ingerer_commit(
  p_repo_id     uuid,
  p_sha         text,
  p_message     text,
  p_branch      text,
  p_authored_at timestamptz
) returns void
language plpgsql
set search_path = public, pg_catalog
as $$
declare
  v_commit_id uuid;
begin
  insert into public.commits (repo_id, sha, message, branch, authored_at)
  values (p_repo_id, p_sha, p_message, p_branch, p_authored_at)
  on conflict (repo_id, sha) do nothing
  returning id into v_commit_id;

  if v_commit_id is not null then
    perform public.fermer_par_reference(v_commit_id);
  end if;
end;
$$;

revoke execute on function public.ingerer_commit(uuid, text, text, text, timestamptz) from public, anon;
grant execute on function public.ingerer_commit(uuid, text, text, text, timestamptz) to authenticated;
