-- L'ecran d'accueil : tous les repos d'un coup (issue #8).
--
-- Deux choses ici. D'abord la correspondance proprietaire -> compte, reglee une
-- fois par l'utilisateur : c'est elle, croisee au proprietaire du remote git
-- pousse par le daemon, qui donne le badge @perso / @pro. Aucun appel a l'API
-- GitHub nulle part : le proprietaire vient du remote, le classement vient de
-- l'utilisateur, et rien d'autre ne sort de la base.
--
-- Ensuite l'apercu lui-meme : une seule fonction qui rend, par repo, de quoi
-- peindre sa bande d'etat et le trier. Le tri place en tete ce qui demande une
-- decision (un conflit), puis l'ecriture, puis la lecture, puis le silence.

-- ------------------------------------------------- correspondance des comptes
-- Une ligne par proprietaire classe, et par utilisateur. Le meme proprietaire
-- peut etre @pro chez l'un et @perso chez l'autre : la correspondance appartient
-- a chacun, jamais partagee.
create table public.account_mappings (
  user_id    uuid not null default auth.uid()
             references auth.users (id) on delete cascade,
  owner      text not null,
  label      text not null check (label in ('perso', 'pro')),
  updated_at timestamptz not null default now(),
  primary key (user_id, owner)
);

alter table public.account_mappings enable row level security;

grant select, insert, update, delete on public.account_mappings to authenticated;
-- Sans ce grant explicite, les tests d'integration (cle service_role) prendraient
-- un 403 en ecrivant la correspondance.
grant all on public.account_mappings to service_role;

-- Chacun ne voit et ne regle que ses propres correspondances.
create policy account_mappings_all_own on public.account_mappings
  for all using (auth.uid() = user_id) with check (auth.uid() = user_id);

-- --------------------------------------------------------------- l'apercu
-- Pour chaque repo de l'appelant : son etat dominant, ses comptes d'activite,
-- son badge de compte, deja trie. L'ecran lit cette fonction et rien d'autre ;
-- il ne re-trie pas, il ne recompte pas.
--
-- L'etat dominant se lit sur `etat_modules`, exactement comme le plan : un repos
-- ou deux sessions ecrivent dans le meme sous-arbre est en conflit, un ou une
-- seule ecrit est en ecriture, un ou l'on ne fait que lire est en lecture, le
-- reste est muet. Ainsi la bande de l'accueil ne peut pas contredire le plan.
--
-- `security invoker` (par defaut) : la fonction voit ce que la RLS laisse voir a
-- l'appelant, donc ses seuls repos. Le badge ne fuit pas d'un compte a l'autre.
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
  dernier_evenement timestamptz
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
      coalesce(a.agents, 0) as agents,
      -- Le conflit se compte au point de rencontre, comme sur le plan et le
      -- bandeau : jamais une fois par ancetre teinte.
      coalesce(c.conflits, 0) as conflits,
      coalesce(e.ecrits, 0)   as modules_ecrits,
      coalesce(e.lus, 0)      as modules_lus,
      e.dernier_evenement
    from public.repos r
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
    b.dernier_evenement
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
