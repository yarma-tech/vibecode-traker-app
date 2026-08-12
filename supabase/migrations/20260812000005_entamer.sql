-- Entamer un travail sans intervention (issue #31, chantier 3 de la
-- conception, §5.4). Le signal existe deja dans `activity_events` (#4) :
-- rien a ajouter cote daemon, tout se joue ici, sur l'insertion d'une ligne
-- `kind = 'write'`.
--
-- Le routage par chemin ne sert qu'a entamer (PRD, "Deux signaux, deux
-- portees") : il ne ferme rien et ne rouvre rien - une issue terminee le
-- reste, quelle que soit l'activite a son emplacement (FR-010). C'est la
-- clause `statut = 'todo'` de la mise a jour, plus bas, qui porte a elle
-- seule cette garantie : une ligne `done` ne matche jamais.

-- ------------------------------------------------------------ entamer_par_ecriture
-- « Le plus profond qui prefixe » se joue en deux temps :
--
--   1. reperer, parmi les travaux VIVANTS du meme depot (blocs simples et
--      issues confondus - ils sont routables de la meme maniere, §5.4), le
--      chemin le plus long qui prefixe le fichier ecrit ;
--   2. entamer TOUS les travaux qui portent exactement ce chemin (FR-011).
--
-- Ces deux temps sont necessairement separes : un bloc et une issue peuvent
-- partager le meme chemin, et rien ne doit dependre de l'ordre dans lequel
-- la premiere requete les trouve.
--
-- Le prefixe est verifie par SEGMENT, jamais par simple prefixe de chaine :
-- `chemin = new.file_path` (le travail vise le fichier lui-meme) ou
-- `starts_with(new.file_path, chemin || '/')` (le fichier vit sous ce
-- dossier). Sans le `/'` de separation, `web/app` prefixerait a tort
-- `web/application/x.ts` - le piege que la tache signale nommement. Le
-- chemin vide est un troisieme cas explicite : il designe la racine du
-- depot et prefixe tout fichier, y compris lui-meme (FR-009).
--
-- Pourquoi le chemin le plus LONG est toujours le plus PROFOND : tous les
-- candidats retenus sont, par construction, des prefixes segmentes du meme
-- `new.file_path`. Deux prefixes d'une meme chaine, de meme longueur, sont
-- forcement la meme sous-chaine depuis la position 0 - deux chemins
-- distincts ne peuvent donc jamais s'egaler en longueur : `order by length
-- desc limit 1` ne laisse aucune ambiguite sur QUEL chemin a gagne, meme
-- avant l'`update` qui en entame tous les porteurs.
--
-- Security definer, comme `bloc_coherent()` et `issues_etat_bloc()` (#30) :
-- ce trigger derive un etat depuis un signal de telemetrie, il ne repond
-- jamais a une demande d'un compte particulier - il n'a donc pas de
-- `grant execute` a accorder ni de garde-fou d'appelant a poser, une
-- fonction trigger ne s'invoque pas par RPC (Postgres refuse tout appel hors
-- contexte de trigger).
create or replace function public.entamer_par_ecriture() returns trigger
language plpgsql security definer set search_path = public, pg_catalog as $$
declare
  v_chemin text;
begin
  if new.kind <> 'write' then
    return new;
  end if;

  select chemin into v_chemin
  from (
    select chemin from public.blocs
     where repo_id = new.repo_id
       and chemin is not null
       and statut in ('todo', 'doing')
    union all
    select chemin from public.issues
     where repo_id = new.repo_id
       and statut in ('todo', 'doing')
  ) travaux_vivants
  where chemin = ''
     or chemin = new.file_path
     or starts_with(new.file_path, chemin || '/')
  order by length(chemin) desc
  limit 1;

  if v_chemin is null then
    return new;
  end if;

  update public.issues
     set statut = 'doing', updated_at = now()
   where repo_id = new.repo_id and chemin = v_chemin and statut = 'todo';

  -- Un bloc decoupe n'a pas de chemin (il vaut null des la premiere issue,
  -- §4.3) : il ne peut donc jamais matcher `chemin = v_chemin` et n'est
  -- jamais route directement, meme si le `not exists` ci-dessous ne se
  -- declenche jamais dans un etat coherent. Il est pose par prudence, dans
  -- le meme esprit defensif que `fermer_par_reference()` (#32) : composer
  -- avec `bloc_statut_protege()` plutot que de compter uniquement sur
  -- l'invariant tenu ailleurs.
  update public.blocs
     set statut = 'doing', updated_at = now()
   where repo_id = new.repo_id and chemin = v_chemin and statut = 'todo'
     and not exists (select 1 from public.issues where bloc_id = blocs.id);

  return new;
end $$;

-- Apres l'insertion, jamais avant : le trigger derive un etat depuis une
-- ligne deja actee, il ne doit jamais pouvoir empecher l'ecriture du signal
-- lui-meme.
create trigger activite_entamer_ecriture
  after insert on public.activity_events
  for each row execute function public.entamer_par_ecriture();
