-- La conversion d'un PRD `validé` en ses features (issue #37, F12 du PRD,
-- seconde moitie - FR-039 a FR-042 ; conception §6, "la conversion de
-- l'exploration").
--
-- Deux fonctions, un seul principe commun a #36 : on lit d'abord, on decide
-- ensuite, jamais l'inverse - relire le meme document ne doit rien creer de
-- neuf (`unique (repo_id, prd_cle)`).
--
-- `convertir_prd_valide` fait trois choses dans UNE seule transaction (une
-- fonction plpgsql s'execute entierement dans la transaction de son appelant,
-- sans COMMIT intermediaire) : c'est ce qui rend la conversion atomique -
-- une erreur en cours de route (feature malformee, connexion coupee cote
-- client apres l'appel) annule tout, l'exploration comme les features deja
-- inserees, plutot que de laisser un etat a moitie converti (couvert par
-- `daemon/tests/prd.rs`, `une_conversion_qui_echoue_a_mi_chemin_ne_laisse_rien`).

-- ------------------------------------------------------------- prd_converti
-- Le troisieme etat d'un bloc d'exploration, a cote de sa suppression : il
-- « porte quelque chose » (une issue ajoutee a la main, une fermeture) et
-- doit donc survivre a la conversion (PRD §5.1 - "on ne detruit pas du
-- travail, meme pour respecter une regle d'affichage"). Ce marqueur est ce
-- qui distingue, cote web, une exploration ordinaire (encore vierge, en
-- attente de validation) d'une exploration devenue un vestige apres coup.
alter table public.blocs add column prd_converti boolean not null default false;

-- ------------------------------------------------------- prd_champs_proteges
-- FR-042 : la priorite lue dans le document s'affiche et ne se modifie pas
-- depuis le tableau - et « ne se modifie pas » ne peut pas etre seulement une
-- omission de l'interface web (#33 l'a deja demontre pour FR-026 : un POST
-- direct, hors de tout bouton, contourne une regle qui ne vivrait que cote
-- client). Meme mecanisme que `statut_done_protege` (#33) : un role qui
-- n'echappe pas a la RLS (`rolbypassrls`) ne peut modifier aucun des champs
-- issus d'un PRD une fois la ligne posee. `convertir_prd_valide` et
-- `marquer_prd_absent`, plus bas, passent donc en `security definer` pour
-- continuer d'ecrire ces memes colonnes - avec, en echange, leur propre
-- verification de `repo_accessible` puisque la RLS ne les protege plus
-- (meme compromis que `fermer_par_reference`, #32). `creer_bloc_exploration_prd`
-- n'a pas besoin de ce changement : il ne fait jamais d'UPDATE sur ces
-- colonnes-la, seulement un INSERT (une ligne neuve) ou un retour anticipe.
create or replace function public.prd_champs_proteges() returns trigger
language plpgsql set search_path = public, pg_catalog as $$
declare
  v_contourne_rls boolean;
  v_change boolean;
begin
  v_change := new.prd_cle is distinct from old.prd_cle
    or new.prd_priorite is distinct from old.prd_priorite
    or new.prd_a_clarifier is distinct from old.prd_a_clarifier
    or new.prd_statut is distinct from old.prd_statut
    or new.prd_maj is distinct from old.prd_maj
    or new.prd_valide_le is distinct from old.prd_valide_le
    or new.prd_absent is distinct from old.prd_absent
    or new.prd_converti is distinct from old.prd_converti;

  if v_change then
    select rolbypassrls into v_contourne_rls from pg_roles where rolname = current_user;

    if not coalesce(v_contourne_rls, false) then
      raise exception 'les champs issus d''un PRD ne se modifient pas depuis le tableau (FR-042)';
    end if;
  end if;

  return new;
end $$;

create trigger blocs_prd_champs_proteges
  before update on public.blocs
  for each row execute function public.prd_champs_proteges();

-- --------------------------------------------------------- convertir_prd_valide
-- Security definer + verification manuelle de `repo_accessible` (voir le
-- commentaire de `prd_champs_proteges` ci-dessus pour le pourquoi) : cette
-- fonction ecrit des colonnes que la RLS interdit desormais a un role
-- ordinaire de toucher, elle doit donc s'executer sous son proprietaire
-- (postgres, `rolbypassrls = true`) et reprendre elle-meme la verification
-- que la RLS ne fait plus. Un appel sur un depot qui n'appartient pas a
-- l'appelant rend le meme resultat neutre qu'un document sans feature -
-- silencieusement, comme `fermer_par_reference` (#32) : une exception
-- confirmerait que ce `repo_id` existe, ce qui est deja une fuite.
create or replace function public.convertir_prd_valide(
  p_repo_id       uuid,
  p_doc_prd_cle   text,   -- '2026-08-10/PRD-001' : la cle du DOCUMENT, celle de son eventuelle exploration
  p_prd_statut    text,
  p_prd_maj       date,
  p_prd_valide_le date,
  p_features      jsonb   -- [{"cle","titre","priorite","a_clarifier"}, ...] - jamais vide (l'appelant ne convertit pas un document sans feature reconnue, FR-044)
) returns jsonb
language plpgsql
security definer
set search_path = public, pg_catalog
as $$
declare
  v_exploration public.blocs;
  v_vierge      boolean;
  v_convertie   boolean := false;
  v_supprimee   boolean := false;
  v_feature     jsonb;
  v_cle         text;
  v_ref         int;
  v_creees      int := 0;
  v_absentes    int := 0;
  v_cles        text[] := array[]::text[];
  v_neutre      jsonb := jsonb_build_object(
    'features_creees', 0, 'features_absentes', 0,
    'exploration_convertie', false, 'exploration_supprimee', false
  );
begin
  if p_doc_prd_cle is null or btrim(p_doc_prd_cle) = '' then
    raise exception 'il faut la cle du document';
  end if;

  if not public.repo_accessible(p_repo_id) then
    return v_neutre;
  end if;

  -- Un appel direct (hors de `traiter`, qui garde deja ce garde-fou cote
  -- daemon) avec zero feature ne doit RIEN faire : ni supprimer l'exploration,
  -- ni marquer quoi que ce soit absent - un document temporairement illisible
  -- (edition en cours) ne doit jamais faire disparaitre ce qui existe deja.
  if p_features is null or jsonb_array_length(p_features) = 0 then
    return v_neutre;
  end if;

  -- 1. L'exploration cede la place aux features (PRD §5.1). Vierge = jamais
  -- entamee a la main et jamais fermee : `statut` reste celui pose a la
  -- creation (todo/doing, jamais 'done' - une exploration ne se ferme que par
  -- fermer_par_reference, qui insere alors dans `fermetures`), aucune issue,
  -- aucune fermeture. Elle est alors supprimee ; sinon elle est conservee et
  -- marquee convertie, jamais detruite.
  select * into v_exploration from public.blocs
    where repo_id = p_repo_id and prd_cle = p_doc_prd_cle and type = 'exploration';

  if found then
    v_vierge := v_exploration.statut in ('todo', 'doing')
      and not exists (select 1 from public.issues i where i.bloc_id = v_exploration.id)
      and not exists (select 1 from public.fermetures f where f.bloc_id = v_exploration.id);

    if v_vierge then
      delete from public.blocs where id = v_exploration.id;
      v_supprimee := true;
    else
      update public.blocs
         set prd_converti = true, prd_statut = p_prd_statut,
             prd_maj = p_prd_maj, prd_valide_le = p_prd_valide_le,
             updated_at = now()
       where id = v_exploration.id;
      v_convertie := true;
    end if;
  end if;

  -- 2. Chaque feature : upsert par (repo_id, prd_cle), JAMAIS par titre
  -- (FR-040) - un titre qui change ne doit jamais creer de doublon, il met a
  -- jour le bloc que sa cle designe deja. On lit d'abord (UPDATE, qui rend le
  -- nombre de lignes touchees) plutot que de tenter un INSERT et rattraper le
  -- conflit : c'est la seule maniere de savoir si une NOUVELLE reference doit
  -- etre tiree du compteur, sans quoi une mise a jour ordinaire en gaspillerait
  -- une a chaque relecture.
  for v_feature in select * from jsonb_array_elements(p_features)
  loop
    v_cle := v_feature->>'cle';
    v_cles := array_append(v_cles, v_cle);

    update public.blocs set
      titre           = btrim(v_feature->>'titre'),
      prd_priorite    = nullif(btrim(v_feature->>'priorite'), ''),
      prd_a_clarifier = coalesce((v_feature->>'a_clarifier')::boolean, false),
      prd_statut      = p_prd_statut,
      prd_maj         = p_prd_maj,
      prd_valide_le   = p_prd_valide_le,
      prd_absent      = false,
      updated_at      = now()
    where repo_id = p_repo_id and prd_cle = v_cle;

    if found then
      continue;
    end if;

    insert into public.compteur_ref (repo_id, prochain)
    values (p_repo_id, 2)
    on conflict (repo_id) do update
      set prochain = public.compteur_ref.prochain + 1
    returning prochain - 1 into v_ref;

    insert into public.blocs (
      repo_id, ref, type, titre, statut,
      prd_cle, prd_priorite, prd_a_clarifier, prd_statut, prd_maj, prd_valide_le
    ) values (
      p_repo_id, v_ref, 'feature', btrim(v_feature->>'titre'), 'todo',
      v_cle, nullif(btrim(v_feature->>'priorite'), ''),
      coalesce((v_feature->>'a_clarifier')::boolean, false),
      p_prd_statut, p_prd_maj, p_prd_valide_le
    );

    v_creees := v_creees + 1;
  end loop;

  -- 3. Une feature disparue du document : jamais de delete, `prd_absent`
  -- (FR-041). "Disparue" = un bloc DEJA rattache a ce document
  -- (`prd_cle` commence par `<doc>/F`) dont la cle n'est plus dans ce
  -- qu'on vient de lire. `starts_with`, pas `like` : la cle d'un document
  -- peut porter `_` ou `%` (un `id` de PRD n'est contraint a rien), qui
  -- seraient sinon lus comme des jokers.
  update public.blocs
     set prd_absent = true, updated_at = now()
   where repo_id = p_repo_id
     and starts_with(prd_cle, p_doc_prd_cle || '/F')
     and not (prd_cle = any(v_cles))
     and prd_absent = false;
  get diagnostics v_absentes = row_count;

  return jsonb_build_object(
    'features_creees', v_creees,
    'features_absentes', v_absentes,
    'exploration_convertie', v_convertie,
    'exploration_supprimee', v_supprimee
  );
end;
$$;

revoke execute on function public.convertir_prd_valide(uuid, text, text, date, date, jsonb) from public, anon;
grant execute on function public.convertir_prd_valide(uuid, text, text, date, date, jsonb) to authenticated;

-- ---------------------------------------------------------- marquer_prd_absent
-- `statut: abandonné` : plus de creation, les blocs deja issus de ce document
-- (l'exploration si elle n'a jamais ete convertie, chaque feature) passent
-- `prd_absent = true` - jamais de suppression, meme regle que #3 ci-dessus.
-- Security definer + `repo_accessible` manuel, meme raison que
-- `convertir_prd_valide` : `prd_absent` et `prd_statut` sont desormais geles
-- pour tout role qui n'echappe pas a la RLS (`prd_champs_proteges`).
create or replace function public.marquer_prd_absent(
  p_repo_id     uuid,
  p_doc_prd_cle text,
  p_prd_statut  text,
  p_prd_maj     date
) returns int
language plpgsql
security definer
set search_path = public, pg_catalog
as $$
declare v_touchees int;
begin
  if p_doc_prd_cle is null or btrim(p_doc_prd_cle) = '' then
    raise exception 'il faut la cle du document';
  end if;

  if not public.repo_accessible(p_repo_id) then
    return 0;
  end if;

  update public.blocs
     set prd_absent = true, prd_statut = p_prd_statut, prd_maj = p_prd_maj, updated_at = now()
   where repo_id = p_repo_id
     and (prd_cle = p_doc_prd_cle or starts_with(prd_cle, p_doc_prd_cle || '/F'))
     and (prd_absent is distinct from true
          or prd_statut is distinct from p_prd_statut
          or prd_maj is distinct from p_prd_maj);

  get diagnostics v_touchees = row_count;
  return v_touchees;
end;
$$;

revoke execute on function public.marquer_prd_absent(uuid, text, text, date) from public, anon;
grant execute on function public.marquer_prd_absent(uuid, text, text, date) to authenticated;

-- ------------------------------------------------------ regression valide -> draft
-- Une regression de statut (le document repasse de `validé` a `draft`, une
-- correction manuelle malheureuse ou un chantier rouvert) ne doit pas
-- ressusciter une exploration a cote de features deja converties : elles
-- restent la source de verite, et un troisieme etat ("exploration ET
-- features actives en meme temps pour le meme document") ne correspond a
-- rien dans le vocabulaire du produit (PRD, "Les quatre types"). Si la
-- conversion a deja produit des features pour cette cle de document, on ne
-- cree rien - silencieusement, comme l'idempotence deux lignes plus haut.
create or replace function public.creer_bloc_exploration_prd(
  p_repo_id       uuid,
  p_titre         text,
  p_chemin        text,
  p_prd_cle       text,
  p_prd_statut    text,
  p_prd_maj       date,
  p_prd_valide_le date
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

  if p_prd_cle is null or btrim(p_prd_cle) = '' then
    raise exception 'il faut une cle de PRD';
  end if;

  select * into v_bloc from public.blocs
    where repo_id = p_repo_id and prd_cle = p_prd_cle;

  if found then
    return v_bloc;
  end if;

  if exists (
    select 1 from public.blocs
    where repo_id = p_repo_id and starts_with(prd_cle, p_prd_cle || '/F')
  ) then
    return null;
  end if;

  insert into public.compteur_ref (repo_id, prochain)
  values (p_repo_id, 2)
  on conflict (repo_id) do update
    set prochain = public.compteur_ref.prochain + 1
  returning prochain - 1 into v_ref;

  insert into public.blocs (
    repo_id, ref, type, titre, chemin,
    prd_cle, prd_statut, prd_maj, prd_valide_le
  )
  values (
    p_repo_id, v_ref, 'exploration', btrim(p_titre), btrim(p_chemin),
    p_prd_cle, p_prd_statut, p_prd_maj, p_prd_valide_le
  )
  returning * into v_bloc;

  return v_bloc;
end;
$$;

revoke execute on function public.creer_bloc_exploration_prd(uuid, text, text, text, text, date, date) from public, anon;
grant execute on function public.creer_bloc_exploration_prd(uuid, text, text, text, text, date, date) to authenticated;
