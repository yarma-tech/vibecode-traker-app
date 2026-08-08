-- Jetons, cout et temps par repo (issue #7).
--
-- Le daemon agrege la consommation de chaque session dans `sessions`. Le cout ne
-- s'y stocke pas : il se calcule a la lecture, a partir du modele et d'une table
-- de prix. Ainsi un changement de prix ne demande qu'une migration, il recalcule
-- l'historique, et un modele inconnu montre ses jetons sans cout faux.

-- --------------------------------------------------------------- table de prix
-- Prix publics indicatifs d'Anthropic, en USD par million de jetons. La verite
-- reste la console ; ces chiffres sont une estimation, faciles a corriger ici.
-- Les jetons de cache lu et de cache cree se tarifent a part : les confondre
-- fausse le cout d'un facteur trois sur une longue session.
create table public.model_prices (
  famille        text primary key,
  input          numeric not null,
  output         numeric not null,
  cache_read     numeric not null,
  cache_creation numeric not null
);

insert into public.model_prices (famille, input, output, cache_read, cache_creation) values
  ('Opus',   15.0, 75.0, 1.50, 18.75),
  ('Sonnet',  3.0, 15.0, 0.30,  3.75),
  ('Haiku',   0.80, 4.0, 0.08,  1.0);

-- Reference partagee, sans utilisateur : lisible par tous, ecrite par personne.
alter table public.model_prices enable row level security;
grant select on public.model_prices to authenticated, anon;
grant all on public.model_prices to service_role;

create policy model_prices_lisible on public.model_prices
  for select using (true);

-- ------------------------------------------------------------ modele -> famille
-- Le journal ecrit un identifiant complet (« claude-opus-4-8-… ») ; la table de
-- prix ne connait que la famille. Un modele hors des trois familles n'a pas de
-- prix : la fonction rend NULL, et le cout suivra.
create or replace function public.famille_modele(p_model text)
returns text
language sql
immutable
as $$
  select case
    when p_model ilike '%opus%'   then 'Opus'
    when p_model ilike '%sonnet%' then 'Sonnet'
    when p_model ilike '%haiku%'  then 'Haiku'
    else null
  end;
$$;

-- ------------------------------------------------------------------- le cout
-- NULL si le modele est inconnu : montrer des jetons sans cout vaut mieux qu'un
-- cout faux. Le cache compte a son propre tarif.
create or replace function public.cout_session(
  p_model          text,
  p_input          bigint,
  p_output         bigint,
  p_cache_read     bigint,
  p_cache_creation bigint
)
returns numeric
language sql
stable
set search_path = public, pg_catalog
as $$
  select case
    when pr.famille is null then null
    else (
      p_input          * pr.input
      + p_output         * pr.output
      + p_cache_read     * pr.cache_read
      + p_cache_creation * pr.cache_creation
    ) / 1000000.0
  end
  from (select public.famille_modele(p_model) as fam) f
  left join public.model_prices pr on pr.famille = f.fam;
$$;

grant execute on function public.famille_modele(text) to authenticated, anon;
grant execute on function
  public.cout_session(text, bigint, bigint, bigint, bigint)
  to authenticated, anon;

-- ----------------------------------------------------- noter_session, etendue
-- La signature gagne les jetons et le modele. Changer le type de retour ou la
-- signature d'une fonction exige de la supprimer d'abord : `create or replace`
-- refuse.
drop function if exists public.noter_session(
  text, uuid, uuid, text, timestamptz, timestamptz
);

-- Les jetons s'AJOUTENT : un envoi ne transporte que ce qu'il a vu, jamais le
-- total. `least`/`greatest` rendent l'ordre des lots sans importance ; le modele
-- retenu est le dernier annonce. Le chemin des evenements appelle sans jetons :
-- leurs valeurs par defaut ajoutent zero et ne touchent pas le modele.
create function public.noter_session(
  p_id             text,
  p_repo_id        uuid,
  p_machine_id     uuid,
  p_branch         text,
  p_started_at     timestamptz,
  p_last_event_at  timestamptz,
  p_input          bigint default 0,
  p_output         bigint default 0,
  p_cache_read     bigint default 0,
  p_cache_creation bigint default 0,
  p_model          text default null
)
returns void
language sql
security invoker
set search_path = public, pg_catalog
as $$
  insert into public.sessions (
    id, repo_id, machine_id, branch, started_at, last_event_at,
    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, model
  )
  values (
    p_id, p_repo_id, p_machine_id, p_branch, p_started_at, p_last_event_at,
    p_input, p_output, p_cache_read, p_cache_creation, p_model
  )
  on conflict (id) do update set
    branch                = coalesce(excluded.branch, public.sessions.branch),
    started_at            = least(public.sessions.started_at, excluded.started_at),
    last_event_at         = greatest(public.sessions.last_event_at, excluded.last_event_at),
    input_tokens          = public.sessions.input_tokens + excluded.input_tokens,
    output_tokens         = public.sessions.output_tokens + excluded.output_tokens,
    cache_read_tokens     = public.sessions.cache_read_tokens + excluded.cache_read_tokens,
    cache_creation_tokens = public.sessions.cache_creation_tokens + excluded.cache_creation_tokens,
    model                 = coalesce(excluded.model, public.sessions.model);
$$;

grant execute on function public.noter_session(
  text, uuid, uuid, text, timestamptz, timestamptz,
  bigint, bigint, bigint, bigint, text
) to authenticated;

-- ---------------------------------------------------------- releve d'un repo
-- La memoire longue « combien m'a coute ce repo » : la somme de ses sessions,
-- qui survit a la purge des evenements. Le cout total ignore les sessions sans
-- prix (sum saute les NULL) ; `sessions_tarifees` dit combien ont compte, pour
-- que le bandeau puisse avouer ce qui manque.
create or replace function public.releve_repo(p_repo_id uuid)
returns table (
  input_tokens          bigint,
  output_tokens         bigint,
  cache_read_tokens     bigint,
  cache_creation_tokens bigint,
  cout_usd              numeric,
  sessions              int,
  sessions_tarifees     int
)
language sql
stable
security invoker
set search_path = public, pg_catalog
as $$
  select
    coalesce(sum(s.input_tokens), 0)::bigint,
    coalesce(sum(s.output_tokens), 0)::bigint,
    coalesce(sum(s.cache_read_tokens), 0)::bigint,
    coalesce(sum(s.cache_creation_tokens), 0)::bigint,
    sum(public.cout_session(
      s.model, s.input_tokens, s.output_tokens,
      s.cache_read_tokens, s.cache_creation_tokens
    )),
    count(*)::int,
    count(*) filter (where public.famille_modele(s.model) is not null)::int
  from public.sessions s
  where s.repo_id = p_repo_id;
$$;

grant execute on function public.releve_repo(uuid) to authenticated, anon;
