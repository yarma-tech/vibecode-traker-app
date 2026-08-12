-- Decouper un bloc en issues (issue #30, chantier 2 de la conception,
-- deuxieme moitie). Un chantier long se lit desormais « 12 / 17 » sur une
-- ligne au lieu d'une case opaque : le bloc suit ses issues, sans que deux
-- etats aient a etre tenus a la main (§4.2 a §4.4 de la conception).

create table public.issues (
  id         uuid primary key default gen_random_uuid(),
  user_id    uuid not null default auth.uid()
             references auth.users (id) on delete cascade,
  repo_id    uuid not null references public.repos (id) on delete cascade,
  bloc_id    uuid not null references public.blocs (id) on delete cascade,
  ref        int  not null,                    -- meme compteur que les blocs (compteur_ref)
  titre      text not null,
  -- Relatif a la racine du depot ; '' = racine. Pas de defaut : une issue
  -- porte toujours un emplacement, herite du bloc pour la premiere (voir
  -- bloc_coherent() ci-dessous) ou saisi pour les suivantes.
  chemin     text not null,
  statut     text not null default 'todo'
             check (statut in ('todo','doing','done')),
  version    int  not null default 1,
  position   int  not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (repo_id, ref)
);

-- Pas d'index d'unicite sur (repo_id, chemin) : six issues peuvent partager
-- un dossier, c'est delibere depuis que la fermeture passe par la reference
-- et non par le chemin (§4.3 de la conception).
create index issues_bloc_idx on public.issues (bloc_id);
create index issues_route_idx on public.issues (repo_id, chemin);

alter table public.issues enable row level security;

-- Sans les grants, PostgREST repond "permission denied" malgre des policies
-- justes (CONTRIBUTING.md) : la table suit son depot via `repo_accessible`,
-- comme `blocs` et `modules`.
grant select, insert, update, delete on public.issues to authenticated;
grant all on public.issues to service_role;

alter publication supabase_realtime add table public.issues;

create policy issues_select_own on public.issues
  for select using (public.repo_accessible(repo_id));

create policy issues_insert_own on public.issues
  for insert with check (public.repo_accessible(repo_id));

create policy issues_update_own on public.issues
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id));

create policy issues_delete_own on public.issues
  for delete using (public.repo_accessible(repo_id));

-- ------------------------------------------------------------- bloc_coherent
-- Un bloc ne porte jamais a la fois un emplacement et des issues (FR-007).
-- La premiere issue inseree fait DESCENDRE le chemin du bloc dans elle-meme
-- et vide celui du bloc, plutot que de refuser l'insertion : on decoupe
-- apres coup et rien ne se perd (§4.3). La descente est inconditionnelle -
-- elle ecrase tout chemin fourni a la creation - car c'est un invariant tenu
-- par la base, pas une simple valeur par defaut cote application.
create or replace function public.bloc_coherent() returns trigger
language plpgsql security definer set search_path = public, pg_catalog as $$
declare
  v_chemin_bloc text;
begin
  select chemin into v_chemin_bloc from public.blocs where id = new.bloc_id;

  if v_chemin_bloc is not null
     and not exists (select 1 from public.issues where bloc_id = new.bloc_id)
  then
    new.chemin := v_chemin_bloc;
    update public.blocs set chemin = null, updated_at = now() where id = new.bloc_id;
  end if;

  return new;
end $$;

create trigger issues_bloc_coherent
  before insert on public.issues
  for each row execute function public.bloc_coherent();

-- ----------------------------------------------------------------- etat_bloc
-- L'etat d'un bloc decoupe se deduit de celui de ses issues (§4.4, FR-017) :
-- toutes a faire -> a faire ; au moins une entamee ou terminee -> en cours ;
-- toutes terminees -> termine. `v_total = 0` (bloc simple) ne touche a rien -
-- son statut lui appartient encore, rien ne le lui a jamais retire.
create or replace function public.etat_bloc(p_bloc_id uuid) returns void
language plpgsql security definer set search_path = public, pg_catalog as $$
declare v_total int; v_done int; v_doing int;
begin
  select count(*), count(*) filter (where statut = 'done'),
         count(*) filter (where statut = 'doing')
    into v_total, v_done, v_doing
    from public.issues where bloc_id = p_bloc_id;

  if v_total = 0 then return; end if;

  update public.blocs set
    statut = case when v_done = v_total then 'done'
                  when v_doing > 0 or v_done > 0 then 'doing'
                  else 'todo' end,
    updated_at = now()
  where id = p_bloc_id;
end $$;

-- Appelee apres tout changement d'une issue (insertion, mise a jour du
-- statut, suppression) : un bloc decoupe n'est donc jamais ecrit directement,
-- ni par le web ni par la machine a etats a venir (#31, #32).
create or replace function public.issues_etat_bloc() returns trigger
language plpgsql security definer set search_path = public, pg_catalog as $$
begin
  if tg_op = 'DELETE' then
    perform public.etat_bloc(old.bloc_id);
    return old;
  end if;

  perform public.etat_bloc(new.bloc_id);

  -- Une issue peut changer de bloc ? Non - la colonne n'est jamais modifiee
  -- par ce produit -, mais si l'ancien bloc differe on le derive aussi, pour
  -- ne jamais laisser un bloc orphelin de sa derniere issue sur un statut
  -- perime.
  if tg_op = 'UPDATE' and old.bloc_id is distinct from new.bloc_id then
    perform public.etat_bloc(old.bloc_id);
  end if;

  return new;
end $$;

create trigger issues_etat_bloc_maj
  after insert or update or delete on public.issues
  for each row execute function public.issues_etat_bloc();

-- ----------------------------------------------------------- bloc_statut_protege
-- FR-018 : un bloc decoupe ne peut pas etre deplace directement. Seule
-- `etat_bloc()`, appelee depuis le trigger ci-dessus, a le droit d'ecrire son
-- statut - une ecriture venue de la, comme de tout autre trigger imbrique,
-- s'execute a une profondeur de trigger superieure a 1 ; une ecriture directe
-- (web, API, machine a etats future) s'execute a la profondeur 1, celle du
-- trigger lui-meme. C'est ce qui distingue l'une de l'autre.
create or replace function public.bloc_statut_protege() returns trigger
language plpgsql set search_path = public, pg_catalog as $$
begin
  if new.statut is distinct from old.statut
     and pg_trigger_depth() <= 1
     and exists (select 1 from public.issues where bloc_id = new.id)
  then
    raise exception 'un bloc decoupe ne peut pas etre deplace directement (FR-018)';
  end if;

  return new;
end $$;

create trigger blocs_statut_protege
  before update on public.blocs
  for each row execute function public.bloc_statut_protege();

-- -------------------------------------------------------------- creer_issue
-- Meme mecanisme d'attribution de reference que `creer_bloc` (#29), prolonge
-- et non duplique : increment atomique de `compteur_ref`, jamais `max(ref)`.
-- Un `VM-7` designe donc un bloc OU une issue, jamais les deux.
--
-- `p_chemin` peut etre vide pour la premiere issue d'un bloc encore simple :
-- `bloc_coherent()` le remplacera par le chemin du bloc. Dans tout autre cas
-- il est obligatoire - cette fonction le verifie pour rendre l'erreur lisible
-- plutot que de laisser echouer la contrainte NOT NULL brute.
--
-- Fonction en droits de l'appelant (pas security definer) : c'est la RLS des
-- tables ci-dessus qui autorise ou refuse l'ecriture, pas cette fonction.
create or replace function public.creer_issue(
  p_bloc_id uuid,
  p_titre   text,
  p_chemin  text
) returns public.issues
language plpgsql
set search_path = public, pg_catalog
as $$
declare
  v_repo_id     uuid;
  v_chemin_bloc text;
  v_premiere    boolean;
  v_ref         int;
  v_issue       public.issues;
begin
  if p_titre is null or btrim(p_titre) = '' then
    raise exception 'il faut un titre';
  end if;

  select repo_id, chemin into v_repo_id, v_chemin_bloc
    from public.blocs where id = p_bloc_id;

  if v_repo_id is null then
    raise exception 'bloc introuvable';
  end if;

  v_premiere := not exists (select 1 from public.issues where bloc_id = p_bloc_id);

  if not (v_premiere and v_chemin_bloc is not null)
     and (p_chemin is null or btrim(p_chemin) = '')
  then
    raise exception 'il faut un emplacement';
  end if;

  insert into public.compteur_ref (repo_id, prochain)
  values (v_repo_id, 2)
  on conflict (repo_id) do update
    set prochain = public.compteur_ref.prochain + 1
  returning prochain - 1 into v_ref;

  insert into public.issues (repo_id, bloc_id, ref, titre, chemin)
  values (v_repo_id, p_bloc_id, v_ref, btrim(p_titre), coalesce(btrim(p_chemin), ''))
  returning * into v_issue;

  return v_issue;
end;
$$;

revoke execute on function public.creer_issue(uuid, text, text) from public, anon;
grant execute on function public.creer_issue(uuid, text, text) to authenticated;
