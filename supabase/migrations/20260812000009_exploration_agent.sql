-- Constater une exploration ecrite par un agent (issue #38, F13 du PRD -
-- FR-045 a FR-048 ; conception, §5.4 et §6).
--
-- Un document de conception ecrit par un agent (une decision d'ADR, une
-- spec, un plan) laisse une trace, pour se souvenir qu'on y a reflechi -
-- « un memo, pas une charge de travail » (PRD, "Les quatre types"). Comme
-- `entamer_par_ecriture()` (#31), le signal existe deja dans
-- `activity_events` : rien a ajouter cote daemon, un second trigger suffit
-- sur la meme insertion `kind = 'write'`.

-- ------------------------------------------------- couverture : reprise ou ecart ?
-- Le trigger d'entamer (#31) a deja une notion de « prefixe le plus profond »
-- pour ROUTER une ecriture vers le travail qu'elle avance. Cette migration
-- reprend la MEME logique de prefixe segmente (`chemin = fichier` ou
-- `starts_with(fichier, chemin || '/')`, le chemin vide couvrant tout) pour
-- une question differente : pas « quel est le travail le plus profond a
-- entamer » (il n'y a rien a departager, un simple EXISTS suffit), mais
-- « ce chemin porte-t-il DEJA une carte, laquelle qu'elle soit ».
--
-- Un ecart assume par rapport a `entamer_par_ecriture()` : la couverture ne
-- filtre PAS sur le statut. `entamer` se limite aux travaux VIVANTS
-- (todo/doing) parce que sa question est « faut-il reprendre ce travail » et
-- qu'un travail termine ne se rouvre jamais sur la seule foi d'une ecriture
-- (FR-010). Ici la question est toute autre : « ce fichier a-t-il deja une
-- carte, peu importe son etat ». Un bloc Termine a cette adresse a DEJA
-- rempli ce role - il continue de couvrir. C'est aussi ce qui rend le
-- mecanisme stable a la reecriture d'un fichier existant : sans ce choix, un
-- agent qui reouvre puis resauvegarde une ADR deja livree ferait apparaitre
-- une exploration en double a cote d'une carte Terminee qui dit deja tout.
create or replace function public.creer_exploration_par_ecriture() returns trigger
language plpgsql security definer set search_path = public, pg_catalog as $$
declare
  v_couvert boolean;
  v_titre   text;
  v_ref     int;
begin
  if new.kind <> 'write' then
    return new;
  end if;

  -- Perimetre de FR-045 : seuls ces trois dossiers, en toute profondeur
  -- ("sous"). Un prefixe LITTERAL, `/` final compris, ecarte deja de
  -- lui-meme le piege du faux dossier voisin (`docs/adr-old/` ne commence
  -- pas par `docs/adr/`) sans avoir besoin de la logique de segment
  -- ci-dessous, qui ne sert qu'a la couverture plus bas.
  if not (
    starts_with(new.file_path, 'docs/adr/')
    or starts_with(new.file_path, 'docs/superpowers/specs/')
    or starts_with(new.file_path, 'docs/superpowers/plans/')
  ) then
    return new;
  end if;

  select exists (
    select 1 from (
      select chemin from public.blocs where repo_id = new.repo_id and chemin is not null
      union all
      select chemin from public.issues where repo_id = new.repo_id
    ) travaux
    where chemin = ''
       or chemin = new.file_path
       or starts_with(new.file_path, chemin || '/')
  ) into v_couvert;

  if v_couvert then
    return new;
  end if;

  -- FR-046 : le titre est le seul nom de fichier (dernier segment du
  -- chemin), jamais le chemin complet ni une ligne lue dans le document -
  -- ce trigger n'ouvre jamais le fichier, il ne connait que `file_path`,
  -- une colonne deja posee par `activity_events` (#4). C'est cette absence
  -- de lecture qui garde F13 hors de la liste fermee de ce qui quitte la
  -- machine (CONTRIBUTING.md) : un chemin relatif voyage deja pour toute
  -- activite, un nom de fichier n'ajoute rien de neuf.
  v_titre := substring(new.file_path from '[^/]+$');

  insert into public.compteur_ref (repo_id, prochain)
  values (new.repo_id, 2)
  on conflict (repo_id) do update
    set prochain = public.compteur_ref.prochain + 1
  returning prochain - 1 into v_ref;

  -- FR-045 : directement « En cours », jamais « A faire » - a la difference
  -- d'un bloc saisi a la main (F1), une exploration n'est jamais une
  -- intention a valider, elle CONSTATE un travail deja en train de se faire.
  --
  -- `on conflict ... do nothing` sur l'index unique partiel plus bas : deux
  -- agents qui ecrivent le meme fichier au meme instant s'executent chacun
  -- dans sa PROPRE transaction, l'un ne voit donc pas encore la ligne de
  -- l'autre au moment du EXISTS ci-dessus (READ COMMITTED) - seul l'index,
  -- verifie a l'ecriture elle-meme, empeche vraiment le doublon. La
  -- reference tiree par la transaction perdante n'est alors pas reutilisee
  -- (elle est simplement sautee) : un trou dans la numerotation est deja
  -- accepte ailleurs dans ce schema (une reference n'est jamais reutilisee,
  -- PRD FR-004), jamais un doublon de carte.
  insert into public.blocs (repo_id, ref, type, titre, statut, chemin)
  values (new.repo_id, v_ref, 'exploration', v_titre, 'doing', new.file_path)
  on conflict (repo_id, chemin) where (type = 'exploration' and chemin is not null) do nothing;

  return new;
end $$;

-- Meme raisonnement de securite que `entamer_par_ecriture()` (#31) : ce
-- trigger derive un etat depuis un signal de telemetrie, il ne repond jamais
-- a une demande d'un compte particulier - pas de `grant execute` a accorder,
-- une fonction trigger ne s'invoque pas par RPC.
create trigger activite_creer_exploration
  after insert on public.activity_events
  for each row execute function public.creer_exploration_par_ecriture();

-- ------------------------------------------------------- l'index qui protege
-- Un bloc d'exploration ne peut anchrer qu'UNE fois un chemin donne, dans un
-- depot donne : c'est ce qui rend `on conflict ... do nothing` ci-dessus
-- reellement sur sous ecriture concurrente (un NOT EXISTS seul, sans cet
-- index, ne le serait pas), et c'est aussi ce qui force
-- `creer_bloc_exploration_prd()` (#36/#37, ci-dessous) a ADOPTER une ligne
-- deja posee par ce trigger plutot que d'en inserer une seconde en
-- violation de l'index.
--
-- Scope au SEUL `type = 'exploration'` : un bloc de tout autre type peut
-- legitimement partager son chemin avec une exploration si l'utilisateur a
-- retype l'un des deux (FR-032/FR-048, "a tout moment") - cette migration ne
-- doit alors bloquer ni le retypage, ni une exploration neuve qui apparaitrait
-- ensuite pour ce meme fichier.
create unique index blocs_exploration_chemin_unique
  on public.blocs (repo_id, chemin)
  where type = 'exploration' and chemin is not null;

-- --------------------------------------------- collision avec #36/#37 (le PRD)
-- Un fichier sous `docs/adr/`, `docs/superpowers/specs/` ou
-- `docs/superpowers/plans/` peut AUSSI porter un en-tete PRD complet (rien
-- ne l'empeche : #36 lit tout `*.md` du depot, sans restriction de dossier).
-- Les deux mecanismes visent alors le MEME fichier, et ne doivent pas se
-- marcher dessus dans les deux sens :
--
--  - PRD pose d'abord, agent ecrit ensuite : deja couvert par la logique de
--    couverture ci-dessus (n'importe quel bloc au meme chemin compte, quelle
--    que soit son origine) - rien a changer ici.
--  - agent pose d'abord (le cas courant : l'activite arrive en direct, la
--    cartographie du PRD ne repasse que toutes les 300 s), PRD lu ensuite :
--    SANS ce qui suit, `creer_bloc_exploration_prd()` chercherait une ligne
--    par `prd_cle` (absente, cette ligne vient de l'ecriture d'un agent),
--    ne la trouverait pas, et tenterait d'en INSERER une seconde - qui
--    violerait desormais `blocs_exploration_chemin_unique` ci-dessus.
--
-- La ligne deja posee par un agent est donc ADOPTEE (mise a jour en place)
-- plutot que dedoublee ou qu'on la laisse faire echouer l'insertion. Ce
-- n'est possible que si la fonction peut ecrire les colonnes `prd_*` -
-- protegees par `prd_champs_proteges()` (#37) pour tout role qui n'echappe
-- pas a la RLS - elle devient donc `security definer` avec sa propre
-- verification de `repo_accessible`, exactement le compromis deja pris par
-- `convertir_prd_valide()` et `marquer_prd_absent()` pour la meme raison.
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
security definer
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

  -- Meme resultat neutre qu'un repo inaccessible chez `convertir_prd_valide`
  -- (#37) : une exception confirmerait que ce `repo_id` existe, deja une
  -- fuite. Necessaire uniquement parce que cette fonction devient security
  -- definer par la ligne ci-dessus - avant #38, la RLS de `blocs` et
  -- `compteur_ref` s'en chargeait seule.
  if not public.repo_accessible(p_repo_id) then
    return null;
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

  -- #38 : adopter une exploration deja posee par un agent sur ce MEME
  -- fichier, jamais en dedoubler une - voir le commentaire au-dessus de
  -- l'index `blocs_exploration_chemin_unique` pour le raisonnement complet.
  -- `prd_cle is null` ecarte deliberement toute ligne deja rattachee a un
  -- AUTRE document : cette situation prealable n'est pas nouvelle (deux PRD
  -- differents ne peuvent normalement pas partager un chemin de fichier) et
  -- reste hors du perimetre de cette adoption.
  select * into v_bloc from public.blocs
    where repo_id = p_repo_id and chemin = p_chemin
      and type = 'exploration' and prd_cle is null;

  if found then
    update public.blocs set
      titre           = btrim(p_titre),
      prd_cle         = p_prd_cle,
      prd_statut      = p_prd_statut,
      prd_maj         = p_prd_maj,
      prd_valide_le   = p_prd_valide_le,
      updated_at      = now()
    where id = v_bloc.id
    returning * into v_bloc;

    return v_bloc;
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
