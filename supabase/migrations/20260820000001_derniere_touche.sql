-- La derniere touche d'une zone (issue #75, PRD-002 F8).
--
-- Deux dates par zone : la derniere fois qu'un agent y a ecrit, la derniere
-- fois qu'il s'est contente d'y lire. C'est la seule entorse assumee a « l'etat
-- ne se stocke pas, il se calcule a la lecture » (migration #4). Elle se
-- justifie par un fait, pas par un confort : `purger_activite` emporte les
-- evenements de plus de sept jours (migration #10), donc trente jours
-- d'evenements bruts n'existent pas et n'existeront jamais. Une date, elle,
-- n'est pas un etat : elle ne se perime pas d'elle-meme.
--
-- Trois choix structurent tout ce qui suit.
--
--   1. Une table a part, jamais deux colonnes ajoutees a `modules`. La
--      cartographie efface en bloc les lignes de modules d'un depot avant de
--      les repousser (`daemon/src/lib.rs`, `pousser_plan`), toutes les cinq
--      minutes : deux colonnes posees la seraient perdues au tour suivant
--      (FR-062).
--
--   2. Le cycle de vie de l'agregat est independant de celui des zones. Une
--      zone qui disparait du disque cesse d'etre dessinee, mais sa ligne
--      survit et la retrouve si elle reapparait (FR-064). D'ou l'absence de
--      cle etrangere vers `modules`, et l'absence de droit de suppression : la
--      seule facon d'effacer ces lignes est d'effacer leur depot, en cascade.
--
--   3. L'ecriture est monotone : une date n'y recule jamais (FR-088). Le
--      direct et le depouillement du passe (F9) ecrivent dans le meme agregat,
--      mais le second parcourt du passe. La regle vit ici, du cote de
--      l'agregat, pour qu'aucun appelant n'ait a la connaitre - y compris un
--      envoi rejoue par la file d'attente locale du lecteur.

create table public.dernieres_touches (
  repo_id           uuid not null references public.repos (id) on delete cascade,
  -- Dossier relatif a la racine du depot, comme `activity_events.module_path`.
  -- La racine elle-meme porte la chaine vide. Rien d'autre n'entre ici : ni
  -- chemin absolu, ni nom de fichier, ni session, ni agent - faute de colonne
  -- pour les accueillir (FR-045).
  chemin            text not null,
  derniere_ecriture timestamptz,
  derniere_lecture  timestamptz,
  primary key (repo_id, chemin)
);

alter table public.dernieres_touches enable row level security;

-- Pas de `delete` : les lignes ne s'effacent qu'avec leur depot, par la
-- cascade, qui ne passe ni par les droits ni par la RLS. Ce qui n'est pas
-- accorde ne peut pas etre revoque par erreur plus tard.
grant select, insert, update on public.dernieres_touches to authenticated;
grant all on public.dernieres_touches to service_role;

-- ------------------------------------------------------------------- RLS
-- L'agregat n'a pas de proprietaire propre : il suit son depot, qui suit sa
-- machine. Une machine ne peut donc ecrire que dans l'agregat de ses propres
-- depots, et une machine revoquee n'ecrit plus a la milliseconde.
create policy dernieres_touches_select_own on public.dernieres_touches
  for select using (public.repo_accessible(repo_id));

create policy dernieres_touches_insert_own on public.dernieres_touches
  for insert with check (public.repo_accessible(repo_id));

create policy dernieres_touches_update_own on public.dernieres_touches
  for update using (public.repo_accessible(repo_id))
  with check (public.repo_accessible(repo_id));

-- --------------------------------------------------------------- ecriture
-- Un lot de touches, une requete. Chaque entree ne porte qu'un chemin relatif
-- et jusqu'a deux horodatages : `{"chemin": "web/lib", "ecrit_a": ..., "lu_a":
-- ...}`. Une entree sans `lu_a` ne touche pas la date de lecture (FR-039), et
-- reciproquement (FR-040).
--
-- `greatest` ignore les valeurs nulles : il rend la plus recente des deux dates
-- en presence, et laisse celle qui est en place quand l'entrante est absente.
-- C'est lui, a la fois, qui tient la monotonie et qui garde les deux dates
-- independantes.
--
-- Le regroupement prealable n'est pas un detail de performance : deux entrees
-- du meme lot portant le meme chemin feraient echouer `on conflict do update`,
-- qui refuse de toucher deux fois la meme ligne.
create or replace function public.noter_dernieres_touches(
  p_repo_id uuid,
  p_touches jsonb
)
returns int
language sql
security invoker
set search_path = public, pg_catalog
as $$
  with entrantes as (
    select
      t.chemin,
      max(t.ecrit_a) as ecrit_a,
      max(t.lu_a)    as lu_a
    from jsonb_to_recordset(coalesce(p_touches, '[]'::jsonb))
      as t(chemin text, ecrit_a timestamptz, lu_a timestamptz)
    where t.chemin is not null
      and (t.ecrit_a is not null or t.lu_a is not null)
    group by t.chemin
  ),
  posees as (
    insert into public.dernieres_touches (
      repo_id, chemin, derniere_ecriture, derniere_lecture
    )
    select p_repo_id, e.chemin, e.ecrit_a, e.lu_a
    from entrantes e
    on conflict (repo_id, chemin) do update set
      derniere_ecriture = greatest(
        public.dernieres_touches.derniere_ecriture, excluded.derniere_ecriture
      ),
      derniere_lecture = greatest(
        public.dernieres_touches.derniere_lecture, excluded.derniere_lecture
      )
    returning 1
  )
  select count(*)::int from posees;
$$;

grant execute on function public.noter_dernieres_touches(uuid, jsonb) to authenticated;

-- --------------------------------------------------------------- lecture
-- Les deux dates de chaque parcelle dessinee, heritage compris.
--
-- L'heritage se calcule ICI, par prefixe de chemin, exactement comme
-- `etat_modules` (migration #4) : une touche n'ecrit qu'une ligne, celle de la
-- zone touchee, et aucune ligne d'ancetre n'est tenue a jour (FR-043). Deux
-- lignes ne peuvent donc pas deriver l'une de l'autre.
--
-- La jointure sur `modules` est ce qui fait qu'une zone disparue du depot
-- cesse d'etre rendue sans que sa ligne soit effacee (FR-064).
create or replace function public.touches_modules(p_repo_id uuid)
returns table (
  module_path       text,
  derniere_ecriture timestamptz,
  derniere_lecture  timestamptz
)
language sql
stable
set search_path = public, pg_catalog
as $$
  select
    m.path,
    max(t.derniere_ecriture),
    max(t.derniere_lecture)
  from public.modules m
  join public.dernieres_touches t
    on t.repo_id = m.repo_id
   and case
         -- Les parcelles en « . » portent les fichiers poses directement dans
         -- un dossier : elles ne recoivent rien de ses sous-dossiers.
         when m.path = '.'      then t.chemin = ''
         when m.path like '%/.' then t.chemin = left(m.path, -2)
         else t.chemin = m.path
              -- `starts_with` plutot que `like` : un dossier nomme « src_a »
              -- ferait de son tiret bas un joker et heriterait de « srcXa ».
              or starts_with(t.chemin, m.path || '/')
       end
  where m.repo_id = p_repo_id
  group by m.path;
$$;

grant execute on function public.touches_modules(uuid) to authenticated, anon;
