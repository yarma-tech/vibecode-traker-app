-- Sortir de « Termine », sans jamais y entrer a la main (issue #33, F8 du
-- PRD, §10 de la conception).
--
-- Fermer sur un commit local ferme parfois trop tot (PRD, risques : « une
-- branche abandonnee ferme quand meme ») : il faut une sortie. Mais une
-- seule, dans un seul sens - sinon « Termine » ne veut plus rien dire
-- (FR-025 a FR-027). Deux verrous, pas un (§10) : l'interface ne propose pas
-- le geste vers Termine (web/app/repo/[id]/projet/tableau.tsx), le serveur
-- le refuse aussi - ici.
--
-- FR-026 interdit une TRANSITION (arriver a 'done' depuis le web), jamais un
-- ETAT ('done' tel quel). C'est important pour #34 (FR-032) : changer le
-- type d'un travail doit rester possible « a tout moment, sans effet sur son
-- etat » - y compris sur une carte deja terminee, la seule qu'on relit
-- vraiment une semaine plus tard (PRD F10). Une premiere version de cette
-- migration confondait les deux et bloquait donc aussi ce retypage ; corrige
-- ici, avant fusion.

-- ------------------------------------------------------- INSERT : une policy suffit
--
-- Naitre deja 'done' est TOUJOURS une arrivee illegitime : il n'existe pas
-- de ligne anterieure a comparer, l'etat EST la transition. Une simple
-- policy `with check` le tranche donc entierement, sans qu'aucun trigger ne
-- soit necessaire ici.
--
-- Trouve en essayant de contourner cette meme migration (voir le rapport de
-- l'issue #33) : `blocs_insert_own`/`issues_insert_own` ne verifiaient que
-- `repo_accessible(repo_id)`, jamais `statut` - un POST direct sur
-- `/rest/v1/blocs`, hors de `creer_bloc()`, pouvait donc creer une ligne DEJA
-- 'done'. `creer_bloc()` et `creer_issue()` ne fixent jamais `statut`
-- eux-memes (la colonne retombe sur son defaut `'todo'`) : leur retirer la
-- possibilite d'un `statut` different ne leur enleve donc rien.
drop policy blocs_insert_own on public.blocs;
create policy blocs_insert_own on public.blocs
  for insert with check (public.repo_accessible(repo_id) and statut <> 'done');

drop policy issues_insert_own on public.issues;
create policy issues_insert_own on public.issues
  for insert with check (public.repo_accessible(repo_id) and statut <> 'done');

-- ------------------------------------------------------- UPDATE : il faut un trigger
--
-- Ici, en revanche, une policy ne peut pas trancher : `with check` ne voit
-- que la ligne resultante (NEW), jamais l'ancienne (OLD) dans la meme
-- expression - impossible d'y ecrire « statut passe A done », seulement
-- « statut EST done ». C'est cette confusion qui faisait, dans la premiere
-- version, refuser jusqu'a un simple changement de titre sur une carte deja
-- terminee : `statut <> 'done'` sur le UPDATE rejetait toute ligne qui
-- RESTAIT a 'done', pas seulement celles qui y ARRIVAIENT.
--
-- Un trigger `before update`, lui, recoit OLD et NEW : c'est le seul outil
-- qui peut viser la transition et rien d'autre. Les policies `update` de
-- #29/#30 retrouvent donc leur forme d'origine (`repo_accessible` seul) :
-- ce trigger porte desormais, et seul, la regle de FR-026 sur l'update.
--
-- Meme ligne de partage que l'analyse initiale, reprise ici pour l'update :
-- `fermer_par_reference()` (#32) n'est pas appelee depuis un trigger, c'est
-- un appel RPC ordinaire - la PROFONDEUR de trigger (`pg_trigger_depth()`,
-- le mecanisme de `bloc_statut_protege()`, #30) ne distingue donc rien ici,
-- son propre UPDATE s'executant a la meme profondeur qu'un PATCH direct du
-- web. Ce qui distingue vraiment les ecritures, c'est le ROLE Postgres sous
-- lequel chacune s'execute : un PATCH du web tourne toujours en `anon` ou
-- `authenticated` (`rolbypassrls = false`, verifie localement via
-- `select rolbypassrls from pg_roles`) ; `fermer_par_reference()` et
-- `etat_bloc()`, `security definer`, tournent sous leur proprietaire
-- (`postgres`, `rolbypassrls = true`) ; le daemon et les tests parlent avec
-- `service_role` (`rolbypassrls = true` egalement). Ce trigger relit donc
-- `pg_roles` pour l'appelant courant plutot que de recopier le raisonnement
-- par profondeur de #30, qui ne tiendrait pas ici.
--
-- `service_role` continue, deliberement, de pouvoir ecrire `done`
-- directement - meme decision qu'a la premiere version de cette migration.
-- Cette cle n'est JAMAIS transmise au navigateur, elle vit uniquement dans
-- le daemon et les tests d'integration (`daemon/tests/common/mod.rs`), qui
-- l'emploient deja pour poser des faits qu'un automatisme produirait
-- (`poser_statut_issue`, dont depend par exemple `entamer.rs`). La lui
-- retirer ne fermerait aucune faille reelle et casserait cet usage etabli.
-- FR-026 vise explicitement « une ecriture ... venue du web » : precisement
-- `anon` et `authenticated`, jamais la cle de service.
create or replace function public.statut_done_protege() returns trigger
language plpgsql set search_path = public, pg_catalog as $$
declare
  v_contourne_rls boolean;
begin
  if new.statut = 'done' and old.statut is distinct from new.statut then
    select rolbypassrls into v_contourne_rls from pg_roles where rolname = current_user;

    if not coalesce(v_contourne_rls, false) then
      raise exception 'un role sans bypassrls ne peut pas ecrire done directement (FR-026)';
    end if;
  end if;

  return new;
end $$;

create trigger blocs_statut_done_protege
  before update on public.blocs
  for each row execute function public.statut_done_protege();

create trigger issues_statut_done_protege
  before update on public.issues
  for each row execute function public.statut_done_protege();

-- ------------------------------------------------------- FR-027, par omission
--
-- Aucune colonne n'est ajoutee par cette migration. Un epinglage ou un gel
-- exigerait une colonne pour le porter (par exemple `fige_le` ou `epingle`) :
-- ne pas l'ajouter ferme la porte ici, plutot que de compter sur une regle
-- applicative pour ne jamais l'ouvrir plus tard.
