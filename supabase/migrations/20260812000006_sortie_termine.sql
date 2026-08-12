-- Sortir de « Termine », sans jamais y entrer a la main (issue #33, F8 du
-- PRD, §10 de la conception).
--
-- Fermer sur un commit local ferme parfois trop tot (PRD, risques : « une
-- branche abandonnee ferme quand meme ») : il faut une sortie. Mais une
-- seule, dans un seul sens - sinon « Termine » ne veut plus rien dire
-- (FR-025 a FR-027). Deux verrous, pas un (§10) : l'interface ne propose pas
-- le geste vers Termine (web/app/repo/[id]/projet/tableau.tsx), le serveur
-- le refuse aussi - ici.

-- ------------------------------------------------------- pourquoi une policy, pas un trigger
--
-- `bloc_statut_protege()` (#30) distingue une ecriture directe d'une
-- ecriture derivee par la PROFONDEUR de trigger (`pg_trigger_depth()`) :
-- l'ecriture derivee y arrive TOUJOURS depuis un trigger imbrique
-- (`issues_etat_bloc_maj` -> `etat_bloc()`), donc a une profondeur > 1.
--
-- Ce raisonnement ne tient pas ici, et le recopier serait un piege :
-- `fermer_par_reference()` (#32) n'est PAS appelee depuis un trigger, c'est
-- un appel RPC ordinaire. Son propre UPDATE sur `issues`/`blocs` s'execute
-- donc a la MEME profondeur (1) qu'un PATCH direct du web - la profondeur ne
-- distingue plus rien entre les deux.
--
-- Ce qui les distingue vraiment, c'est le ROLE Postgres sous lequel chaque
-- ecriture s'execute :
--   - un PATCH du web s'execute toujours en `anon` ou `authenticated` ;
--   - `fermer_par_reference()` et `etat_bloc()` sont `security definer` :
--     elles s'executent sous le role proprietaire de la fonction
--     (`postgres`, verifie localement via pg_proc/pg_roles) ;
--   - le daemon et les tests parlent avec la cle `service_role`.
--
-- `postgres` et `service_role` portent tous deux `rolbypassrls` (verifie :
-- `select rolname, rolbypassrls from pg_roles`) ; `anon` et `authenticated`
-- non. C'est exactement la ligne de partage que CONTRIBUTING.md demande
-- (« les regles d'acces vivent dans la RLS, pas dans le client ») : une
-- policy `with check` suffit, sans verrou supplementaire cote fonction ni
-- code applicatif a faire confiance.

-- ------------------------------------------------------- service_role et 'done' : decision
--
-- `service_role` continue, deliberement, de pouvoir ecrire `done`
-- directement. Cette cle n'est JAMAIS transmise au navigateur - elle vit
-- uniquement dans le daemon et dans les tests d'integration
-- (`daemon/tests/common/mod.rs`), qui l'emploient deja pour poser des faits
-- qu'un automatisme produirait (`poser_statut_issue`, dont depend par
-- exemple `entamer.rs` pour eprouver FR-010). La lui retirer ne fermerait
-- aucune faille reelle - un navigateur ne detient jamais cette cle, et un
-- compte qui la detient a deja tous les droits sur toutes les tables de ce
-- schema (`grant all ... to service_role`, partout) - et casserait cet usage
-- etabli. FR-026 vise explicitement « une ecriture ... venue du web » : ce
-- sont precisement `anon` et `authenticated` qu'elle nomme, jamais la cle de
-- service.

-- ------------------------------------------------------- la regle elle-meme
--
-- `statut <> 'done'` sur la ligne resultante suffit, sans lire l'ancien
-- statut : FR-026 n'interdit qu'une seule chose, ARRIVER a 'done' depuis le
-- web - jamais le fait d'en repartir (FR-025 : 'done' -> 'doing' est un
-- `statut` different de 'done', donc toujours accepte). Un PATCH qui
-- laisserait une carte DEJA 'done' inchangee sur ce point serait refuse lui
-- aussi ; aucun geste du produit n'en a besoin aujourd'hui (le tableau ne
-- sait pas encore editer une carte terminee autrement qu'en la sortant), et
-- mieux vaut une regle simple et sure qu'une regle qui viserait exactement
-- le mouvement et rien d'autre.
--
-- La policy s'applique a TOUTE la ligne, quelles que soient les autres
-- colonnes touchees par le meme PATCH : un update qui glisse `statut =
-- 'done'` a cote d'un changement de titre est refuse comme n'importe quel
-- autre, la RLS Postgres annule le UPDATE en entier des qu'une seule ligne
-- resultante echoue le WITH CHECK - jamais une reussite partielle.
drop policy blocs_update_own on public.blocs;
create policy blocs_update_own on public.blocs
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id) and statut <> 'done');

drop policy issues_update_own on public.issues;
create policy issues_update_own on public.issues
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id) and statut <> 'done');

-- La meme regle doit couvrir l'INSERT, pas seulement l'UPDATE : trouve en
-- essayant de contourner cette migration (voir le rapport de l'issue #33).
-- `blocs_insert_own`/`issues_insert_own` ne verifiaient que
-- `repo_accessible(repo_id)`, jamais `statut` - un POST direct sur
-- `/rest/v1/blocs`, hors de `creer_bloc()`, pouvait donc creer une ligne DEJA
-- 'done', sans jamais passer par une seule UPDATE que la policy ci-dessus
-- aurait pu intercepter. `creer_bloc()` et `creer_issue()` ne fixent jamais
-- `statut` eux-memes (la colonne retombe sur son defaut `'todo'`) : leur
-- retirer la possibilite d'un `statut` different ne leur enleve donc rien.
drop policy blocs_insert_own on public.blocs;
create policy blocs_insert_own on public.blocs
  for insert with check (public.repo_accessible(repo_id) and statut <> 'done');

drop policy issues_insert_own on public.issues;
create policy issues_insert_own on public.issues
  for insert with check (public.repo_accessible(repo_id) and statut <> 'done');

-- ------------------------------------------------------- FR-027, par omission
--
-- Aucune colonne n'est ajoutee par cette migration. Un epinglage ou un gel
-- exigerait une colonne pour le porter (par exemple `fige_le` ou `epingle`) :
-- ne pas l'ajouter ferme la porte ici, plutot que de compter sur une regle
-- applicative pour ne jamais l'ouvrir plus tard.
