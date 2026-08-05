-- Le conflit apparait sur le plan (issue #5).
--
-- Le calcul de l'etat existe depuis l'issue #4 : cette migration ne change pas
-- la regle, elle ajoute de quoi la raconter. Nommer les agents en cause demande
-- de connaitre leurs sessions, d'ou la colonne ajoutee a `etat_modules`.
--
-- Un conflit ne se compte qu'une fois. Deux agents sous `src/core` mettent en
-- rouge `src/core` ET `src`, parce qu'un module herite de tout ce qui se passe
-- sous lui. Mais il n'y a qu'un seul conflit, la ou les deux agents se
-- rencontrent : le module en conflit le plus profond. Sans cette regle, un
-- conflit unique compterait autant de fois que sa profondeur, et le compte du
-- bandeau ne collerait plus a ce que le plan montre.

-- Le type de retour change : PostgreSQL ne sait pas remplacer une fonction
-- sur place dans ce cas.
drop function if exists public.etat_modules(uuid, int);

create function public.etat_modules(
  p_repo_id uuid,
  p_fenetre_secondes int default public.fenetre_activite_secondes()
)
returns table (
  module_path       text,
  etat              text,
  lectures          bigint,
  ecritures         bigint,
  sessions_ecrivant int,
  sessions          text[],
  dernier_evenement timestamptz
)
language sql
stable
set search_path = public, pg_catalog
as $$
  with recents as (
    select e.session_id, e.module_path, e.kind, e.occurred_at
    from public.activity_events e
    where e.repo_id = p_repo_id
      and e.occurred_at > now() - make_interval(secs => p_fenetre_secondes)
  )
  select
    m.path,
    case
      when count(*) filter (where r.kind = 'write') = 0 then 'lu'
      when count(distinct r.session_id) filter (where r.kind = 'write') > 1 then 'conflit'
      else 'ecrit'
    end,
    count(*) filter (where r.kind = 'read'),
    count(*) filter (where r.kind = 'write'),
    count(distinct r.session_id) filter (where r.kind = 'write')::int,
    -- Les sessions qui ecrivent, pour que le journal puisse les nommer.
    coalesce(
      array_agg(distinct r.session_id) filter (where r.kind = 'write'),
      '{}'::text[]
    ),
    max(r.occurred_at)
  from public.modules m
  join recents r
    on case
         when m.path = '.'      then r.module_path = ''
         when m.path like '%/.' then r.module_path = left(m.path, -2)
         else r.module_path = m.path
              or starts_with(r.module_path, m.path || '/')
       end
  where m.repo_id = p_repo_id
  group by m.path;
$$;

grant execute on function public.etat_modules(uuid, int) to authenticated, anon;

-- ------------------------------------------------------------- les conflits
-- Un repo, ou tous ceux que l'appelant a le droit de voir. Le bandeau du plan
-- et la liste des repos lisent la meme fonction : leurs comptes ne peuvent
-- donc pas diverger.
create or replace function public.conflits(
  p_repo_id uuid default null,
  p_fenetre_secondes int default public.fenetre_activite_secondes()
)
returns table (
  repo_id           uuid,
  module_path       text,
  sessions_ecrivant int,
  sessions          text[],
  dernier_evenement timestamptz
)
language sql
stable
set search_path = public, pg_catalog
as $$
  with tous as (
    select r.id as repo_id, e.*
    from public.repos r
    cross join lateral public.etat_modules(r.id, p_fenetre_secondes) e
    where (p_repo_id is null or r.id = p_repo_id)
      and e.etat = 'conflit'
  )
  select t.repo_id, t.module_path, t.sessions_ecrivant, t.sessions, t.dernier_evenement
  from tous t
  -- On ne garde que le point de rencontre : si un descendant est lui aussi en
  -- conflit, c'est lui qui porte le conflit, pas son parent.
  where not exists (
    select 1
    from tous plus_bas
    where plus_bas.repo_id = t.repo_id
      and starts_with(plus_bas.module_path, t.module_path || '/')
  )
  order by t.dernier_evenement desc;
$$;

grant execute on function public.conflits(uuid, int) to authenticated, anon;

-- ----------------------------------------------------------- agents presents
-- Le bandeau annonce combien d'agents travaillent sur le repo. Le compte se
-- fait ici, avec la meme fenetre que les couleurs : l'ecran n'a pas a manier
-- une horloge pour dire la meme chose que le plan.
create or replace function public.agents_actifs(
  p_repo_id uuid,
  p_fenetre_secondes int default public.fenetre_activite_secondes()
)
returns int
language sql
stable
set search_path = public, pg_catalog
as $$
  select count(*)::int
  from public.sessions s
  where s.repo_id = p_repo_id
    and s.last_event_at > now() - make_interval(secs => p_fenetre_secondes);
$$;

grant execute on function public.agents_actifs(uuid, int) to authenticated, anon;
