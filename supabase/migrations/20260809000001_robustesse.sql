-- Le daemon survit aux coupures (issue #10).
--
-- Deux garanties cote base, la ou le daemon ne peut pas les tenir seul :
--
--   1. L'agregation des jetons devient idempotente. Jusqu'ici `noter_session`
--      AJOUTAIT les jetons du lot au total : rejouer un fichier de journal, ou
--      redemarrer avant d'avoir persiste la position de lecture, doublait la
--      consommation. Une cle par lot, gardee dans `session_usage`, fait que le
--      meme lot rejoue n'est compte qu'une fois. Les evenements, eux, etaient
--      deja proteges par `unique(session_id, tool_use_id)`.
--
--   2. La purge des evenements de plus de sept jours tourne toute seule, via
--      pg_cron. Les agregats de `sessions` ne sont jamais touches.

-- --------------------------------------------------- registre d'idempotence
-- Une cle par lot de consommation deja compte pour une session. La cle est une
-- empreinte des lignes du lot, calculee par le daemon : deux envois du meme lot
-- portent la meme cle, un lot vraiment nouveau en porte une autre.
create table public.session_usage (
  session_id text not null references public.sessions (id) on delete cascade,
  usage_key  text not null,
  applied_at timestamptz not null default now(),
  primary key (session_id, usage_key)
);

alter table public.session_usage enable row level security;

grant select, insert on public.session_usage to authenticated;
grant all on public.session_usage to service_role;

-- Le registre suit sa session, qui suit son repo, qui suit sa machine : on ne
-- peut ecrire une cle que pour une session qu'on possede deja.
create policy session_usage_insert_own on public.session_usage
  for insert with check (
    exists (
      select 1 from public.sessions s
      where s.id = session_usage.session_id and s.user_id = auth.uid()
    )
  );

create policy session_usage_select_own on public.session_usage
  for select using (
    exists (
      select 1 from public.sessions s
      where s.id = session_usage.session_id and s.user_id = auth.uid()
    )
  );

-- ------------------------------------------- noter_session, rendue idempotente
-- Changer la signature (un parametre de plus) exige de supprimer d'abord :
-- `create or replace` refuse un nouveau jeu de parametres. On passe aussi de SQL
-- a PL/pgSQL pour pouvoir n'ajouter les jetons que si la cle est neuve.
drop function if exists public.noter_session(
  text, uuid, uuid, text, timestamptz, timestamptz,
  bigint, bigint, bigint, bigint, text
);

-- Le chemin des evenements appelle sans jetons ni cle : les jetons valent alors
-- zero et la cle est nulle, donc rien ne change pour lui. Le chemin du cout
-- passe une cle : l'ajout des jetons ne se fait qu'a la premiere apparition de
-- cette cle. `least`/`greatest` gardent l'ordre des lots sans importance.
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
  p_model          text default null,
  p_usage_key      text default null
)
returns void
language plpgsql
security invoker
set search_path = public, pg_catalog
as $$
begin
  -- Les metadonnees d'abord : la session doit exister avant qu'on note sa cle
  -- (contrainte de cle etrangere), et les jetons se posent a zero ici, jamais
  -- ajoutes deux fois.
  insert into public.sessions (
    id, repo_id, machine_id, branch, started_at, last_event_at, model
  )
  values (
    p_id, p_repo_id, p_machine_id, p_branch, p_started_at, p_last_event_at, p_model
  )
  on conflict (id) do update set
    branch        = coalesce(excluded.branch, public.sessions.branch),
    started_at    = least(public.sessions.started_at, excluded.started_at),
    last_event_at = greatest(public.sessions.last_event_at, excluded.last_event_at),
    model         = coalesce(excluded.model, public.sessions.model);

  -- Une cle deja vue : le lot a deja ete compte, on ne rajoute rien.
  if p_usage_key is not null then
    insert into public.session_usage (session_id, usage_key)
    values (p_id, p_usage_key)
    on conflict (session_id, usage_key) do nothing;
    if not found then
      return;
    end if;
  end if;

  -- Cle neuve (ou pas de cle : ancien comportement additif) : on ajoute.
  update public.sessions set
    input_tokens          = input_tokens + p_input,
    output_tokens         = output_tokens + p_output,
    cache_read_tokens     = cache_read_tokens + p_cache_read,
    cache_creation_tokens = cache_creation_tokens + p_cache_creation
  where id = p_id;
end;
$$;

grant execute on function public.noter_session(
  text, uuid, uuid, text, timestamptz, timestamptz,
  bigint, bigint, bigint, bigint, text, text
) to authenticated;

-- ------------------------------------------------------- purge planifiee
-- La purge tourne toute seule, cote base : une fois par jour, elle emporte les
-- evenements de plus de sept jours. `purger_activite` (migration #4) ne touche
-- jamais `sessions` : les agregats de cout survivent. On rend la fonction
-- appelable et on la programme.
grant execute on function public.purger_activite() to service_role;

create extension if not exists pg_cron;

-- Reprogrammer sans doublon : on efface un eventuel ancien job de meme nom.
do $$
begin
  perform cron.unschedule('purger-activite')
  where exists (select 1 from cron.job where jobname = 'purger-activite');
end;
$$;

select cron.schedule(
  'purger-activite',
  '17 3 * * *',
  $$ select public.purger_activite(); $$
);

-- De quoi verifier, depuis un test, que la purge est bien programmee, sans
-- exposer tout le schema `cron`.
create or replace function public.purge_planifiee()
returns boolean
language sql
security definer
set search_path = public, pg_catalog, cron
as $$
  select exists (select 1 from cron.job where jobname = 'purger-activite');
$$;

grant execute on function public.purge_planifiee() to authenticated, anon, service_role;
