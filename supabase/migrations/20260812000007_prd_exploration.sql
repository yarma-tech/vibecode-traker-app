-- Le premier bloc qui vient d'un PRD : l'exploration d'un document `draft`
-- (issue #36, F12 du PRD, premiere moitie - FR-036 a FR-038, FR-043, FR-044 ;
-- conception §6).
--
-- La conversion en features (`validé` -> blocs, FR-039 a FR-042) reste hors
-- de cette migration : c'est #37. Le daemon ne presente donc jamais a cette
-- fonction un PRD `validé` ou `abandonné` - elle ne sait faire qu'une chose,
-- poser l'unique bloc d'exploration d'un brouillon.
--
-- Les colonnes `prd_*` existent depuis 20260812000002_blocs.sql, inertes
-- jusqu'ici : c'est leur premiere ecriture.

-- Droits de l'appelant (pas security definer), comme `creer_bloc` et
-- `ingerer_commit` : c'est la RLS de `blocs` et `compteur_ref` qui autorise
-- ou refuse l'ecriture, y compris pour le daemon - son jeton de machine
-- porte le meme `auth.uid()` que le compte qui l'a appairee (§`repos_modules`,
-- `repo_accessible`), exactement comme pour `ingerer_commit`.
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

  -- Idempotence : relire le meme document ne cree rien de neuf. L'unique
  -- (repo_id, prd_cle) le garantirait deja au moment de l'insert ; on la lit
  -- d'abord pour RENDRE la ligne existante plutot que de faire echouer un
  -- appel qui n'a rien fait de mal - la cartographie repasse toutes les
  -- 300 s, ce cas est le cas normal, pas une erreur.
  select * into v_bloc from public.blocs
    where repo_id = p_repo_id and prd_cle = p_prd_cle;

  if found then
    return v_bloc;
  end if;

  -- Meme compteur que `creer_bloc` (§4.2 de la conception) : une exploration
  -- porte une reference VM-n comme n'importe quel autre travail suivi.
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
