-- Machine injoignable, état gelé (issue #11).
--
-- L'écran ne ment pas quand une machine ne répond plus. Au-delà de 90 s sans
-- battement de cœur, la machine est déclarée injoignable : le dernier état
-- connu reste affiché, mais désaturé et daté. Cette bascule se décide côté
-- écran, à partir de l'âge du dernier battement — car « depuis combien de
-- temps » dépend de l'instant present, que seule l'horloge du navigateur tient.
--
-- La base, elle, n'a qu'un devoir : porter cet âge jusqu'à l'écran. L'aperçu de
-- l'accueil rend déjà, par repo, de quoi peindre sa bande d'état ; on lui ajoute
-- la dernière présence de sa machine (`machines.last_seen_at`). Le rail compare
-- alors cette heure à maintenant pour signaler une machine muette, et l'écran
-- d'un repo fait de même à partir de la présence de sa propre machine.
--
-- Le type de retour change : PostgREST refuse un `create or replace` qui touche
-- à la signature. On dépose donc la fonction avant de la recréer.
drop function if exists public.apercu_repos(int);

create function public.apercu_repos(
  p_fenetre_secondes int default public.fenetre_activite_secondes()
)
returns table (
  id                uuid,
  name              text,
  remote_owner      text,
  compte            text,
  loc_total         int,
  file_count        int,
  agents            int,
  conflits          int,
  modules_ecrits    int,
  modules_lus       int,
  etat              text,
  dernier_evenement timestamptz,
  -- Dernier battement de la machine du repo. `null` tant qu'elle n'a jamais
  -- battu : « jamais vue » n'est pas « vue à l'instant ». L'écran en déduit,
  -- au-delà de 90 s, une machine muette — jamais la base.
  derniere_presence timestamptz
)
language sql
stable
set search_path = public, pg_catalog
as $$
  with base as (
    select
      r.id,
      r.name,
      r.remote_owner,
      am.label as compte,
      r.loc_total,
      r.file_count,
      r.user_id,
      -- La présence suit la machine du repo, pas son activité : une machine
      -- muette peut porter un repo au repos comme un repo qui travaillait.
      mac.last_seen_at as derniere_presence,
      coalesce(a.agents, 0) as agents,
      -- Le conflit se compte au point de rencontre, comme sur le plan et le
      -- bandeau : jamais une fois par ancetre teinte.
      coalesce(c.conflits, 0) as conflits,
      coalesce(e.ecrits, 0)   as modules_ecrits,
      coalesce(e.lus, 0)      as modules_lus,
      e.dernier_evenement
    from public.repos r
    left join public.machines mac on mac.id = r.machine_id
    left join public.account_mappings am
      on am.user_id = r.user_id and am.owner = r.remote_owner
    left join lateral (
      select
        count(*) filter (where m.etat = 'ecrit')::int as ecrits,
        count(*) filter (where m.etat = 'lu')::int    as lus,
        max(m.dernier_evenement) as dernier_evenement
      from public.etat_modules(r.id, p_fenetre_secondes) m
    ) e on true
    left join lateral (
      select count(*)::int as conflits
      from public.conflits(r.id, p_fenetre_secondes)
    ) c on true
    left join lateral (
      select count(*)::int as agents
      from public.sessions s
      where s.repo_id = r.id
        and s.last_event_at > now() - make_interval(secs => p_fenetre_secondes)
    ) a on true
  )
  select
    b.id,
    b.name,
    b.remote_owner,
    b.compte,
    b.loc_total,
    b.file_count,
    b.agents,
    b.conflits,
    b.modules_ecrits,
    b.modules_lus,
    case
      when b.conflits > 0       then 'conflit'
      when b.modules_ecrits > 0 then 'ecrit'
      when b.modules_lus > 0    then 'lu'
      else 'inactif'
    end as etat,
    b.dernier_evenement,
    b.derniere_presence
  from base b
  order by
    -- Ce qui demande une decision d'abord, le silence en dernier.
    case
      when b.conflits > 0       then 0
      when b.modules_ecrits > 0 then 1
      when b.modules_lus > 0    then 2
      else 3
    end,
    b.dernier_evenement desc nulls last,
    b.loc_total desc,
    b.name;
$$;

grant execute on function public.apercu_repos(int) to authenticated, anon;
