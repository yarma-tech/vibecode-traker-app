-- L'activite colore le plan (issue #4).
--
-- Deux tables et une fonction. Les tables ne portent que des metadonnees :
-- quelle session, quel chemin relatif, quelle nature, a quelle heure. Aucun
-- contenu de fichier, aucun prompt, aucun resultat d'outil ne peut y entrer,
-- faute de colonne pour les accueillir.
--
-- L'etat d'un module n'est jamais stocke : il se calcule a la lecture, sur une
-- fenetre glissante. Un etat stocke serait faux des la seconde suivante.

-- Une session d'agent, vue de la machine qui l'heberge.
-- Les colonnes de jetons et de cout sont posees des maintenant mais restent a
-- zero : c'est l'issue #7 qui les remplira.
create table public.sessions (
  id                    text primary key,
  user_id               uuid not null default auth.uid()
                        references auth.users (id) on delete cascade,
  repo_id               uuid not null references public.repos (id) on delete cascade,
  machine_id            uuid not null references public.machines (id) on delete cascade,
  agent_label           text,
  branch                text,
  worktree_path         text,
  started_at            timestamptz not null default now(),
  last_event_at         timestamptz not null default now(),
  ended_at              timestamptz,
  input_tokens          bigint not null default 0,
  output_tokens         bigint not null default 0,
  cache_read_tokens     bigint not null default 0,
  cache_creation_tokens bigint not null default 0,
  cost_usd              numeric,
  model                 text
);

create index sessions_repo_id_idx on public.sessions (repo_id);
create index sessions_user_id_idx on public.sessions (user_id);

create table public.activity_events (
  id          bigserial primary key,
  user_id     uuid not null default auth.uid()
              references auth.users (id) on delete cascade,
  session_id  text not null references public.sessions (id) on delete cascade,
  repo_id     uuid not null references public.repos (id) on delete cascade,
  -- Dossier contenant le fichier, relatif a la racine du repo. La racine
  -- elle-meme porte la chaine vide.
  module_path text not null,
  file_path   text not null,
  kind        text not null check (kind in ('read', 'write')),
  occurred_at timestamptz not null default now(),
  tool_use_id text not null,
  -- Le hook optionnel et le lecteur de journaux postent le meme appel. Cette
  -- contrainte absorbe le doublon : le premier arrive gagne, l'autre est ignore.
  unique (session_id, tool_use_id)
);

-- La vue interroge toujours « ce repo, depuis n secondes ».
create index activity_events_fenetre_idx
  on public.activity_events (repo_id, occurred_at desc);

alter table public.sessions enable row level security;
alter table public.activity_events enable row level security;

grant select, insert, update, delete on public.sessions to authenticated;
grant select, insert, update, delete on public.activity_events to authenticated;
grant usage, select on sequence public.activity_events_id_seq to authenticated;
grant all on public.sessions to service_role;
grant all on public.activity_events to service_role;
grant all on sequence public.activity_events_id_seq to service_role;

-- L'ecran se rafraichit sur signal : c'est ce qui fait tenir les cinq secondes.
alter publication supabase_realtime add table public.sessions;
alter publication supabase_realtime add table public.activity_events;

-- ------------------------------------------------------------------- RLS
-- Les sessions et les evenements suivent leur repo, qui suit sa machine :
-- revoquer une machine coupe ses ecritures a la milliseconde.
create policy sessions_select_own on public.sessions
  for select using (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy sessions_insert_own on public.sessions
  for insert with check (
    auth.uid() = user_id
    and public.repo_accessible(repo_id)
    and public.machine_active(machine_id)
  );

create policy sessions_update_own on public.sessions
  for update using (auth.uid() = user_id and public.repo_accessible(repo_id))
  with check (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy sessions_delete_own on public.sessions
  for delete using (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy activite_select_own on public.activity_events
  for select using (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy activite_insert_own on public.activity_events
  for insert with check (auth.uid() = user_id and public.repo_accessible(repo_id));

create policy activite_delete_own on public.activity_events
  for delete using (auth.uid() = user_id and public.repo_accessible(repo_id));

-- ------------------------------------------------------ fenetre glissante
-- La duree de la fenetre est un reglage, pas une constante enfouie. Elle se
-- change sans toucher au code :
--
--   alter database postgres set vibemap.fenetre_secondes = 900;
--
-- et chaque appelant peut passer la sienne.
create or replace function public.fenetre_activite_secondes()
returns int
language sql
stable
set search_path = public, pg_catalog
as $$
  select coalesce(
    nullif(current_setting('vibemap.fenetre_secondes', true), '')::int,
    600
  );
$$;

-- ------------------------------------------------------- etat des modules
-- Quatre etats, calcules a la lecture (spec, section 4.2) :
--
--   aucun evenement                                   inactif (aucune ligne)
--   uniquement des lectures                           lu
--   au moins une ecriture                             ecrit
--   deux sessions ou plus ecrivent dans le sous-arbre conflit
--
-- Un module herite de tout ce qui se passe sous lui : c'est ce qui fait
-- remonter le conflit vers le parent affiche.
create or replace function public.etat_modules(
  p_repo_id uuid,
  p_fenetre_secondes int default public.fenetre_activite_secondes()
)
returns table (
  module_path       text,
  etat              text,
  lectures          bigint,
  ecritures         bigint,
  sessions_ecrivant int,
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
    max(r.occurred_at)
  from public.modules m
  join recents r
    on case
         -- Les parcelles en « . » portent les fichiers poses directement dans
         -- un dossier : elles ne recoivent rien de ses sous-dossiers.
         when m.path = '.'      then r.module_path = ''
         when m.path like '%/.' then r.module_path = left(m.path, -2)
         else r.module_path = m.path
              -- `starts_with` plutot que `like` : un dossier nomme « src_a »
              -- ferait de son tiret bas un joker et allumerait « srcXa ».
              or starts_with(r.module_path, m.path || '/')
       end
  where m.repo_id = p_repo_id
  group by m.path;
$$;

grant execute on function public.fenetre_activite_secondes() to authenticated, anon;
grant execute on function public.etat_modules(uuid, int) to authenticated, anon;

-- --------------------------------------------------------------- sessions
-- Un envoi ne connait que les evenements qu'il transporte : il ne peut pas
-- savoir si la session avait deja commence avant. `least` et `greatest` font
-- que l'ordre d'arrivee des lots n'a aucune importance.
create or replace function public.noter_session(
  p_id            text,
  p_repo_id       uuid,
  p_machine_id    uuid,
  p_branch        text,
  p_started_at    timestamptz,
  p_last_event_at timestamptz
)
returns void
language sql
security invoker
set search_path = public, pg_catalog
as $$
  insert into public.sessions (id, repo_id, machine_id, branch, started_at, last_event_at)
  values (p_id, p_repo_id, p_machine_id, p_branch, p_started_at, p_last_event_at)
  on conflict (id) do update set
    branch        = coalesce(excluded.branch, public.sessions.branch),
    started_at    = least(public.sessions.started_at, excluded.started_at),
    last_event_at = greatest(public.sessions.last_event_at, excluded.last_event_at);
$$;

grant execute on function
  public.noter_session(text, uuid, uuid, text, timestamptz, timestamptz)
  to authenticated;

-- --------------------------------------------------------------- retention
-- Les evenements ne vivent que sept jours (spec, section 4.4). Les agregats de
-- `sessions` survivent : c'est la memoire longue « combien m'a coute ce repo ».
-- La tache planifiee qui appelle ceci arrive avec l'issue #10.
create or replace function public.purger_activite()
returns bigint
language sql
security definer
set search_path = public, pg_catalog
as $$
  with efface as (
    delete from public.activity_events
    where occurred_at < now() - interval '7 days'
    returning 1
  )
  select count(*) from efface;
$$;
