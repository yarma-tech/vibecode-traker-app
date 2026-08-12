-- L'identite logique d'un depot (issue #28, chantier 1 de la conception).
--
-- `repos.root_hash` reste l'empreinte du chemin absolu : elle est propre a un
-- clone et ne dit rien de deux clones du meme depot. On lui ajoute une
-- identite qui, elle, survit a un `mv` et se retrouve d'une machine a
-- l'autre : l'URL du distant normalisee, ou `local:<root_hash>` en son
-- absence. La normalisation vit dans `daemon/src/plan.rs`, testee sur
-- fixtures ; elle ne se rejoue pas ici.

alter table public.repos
  add column identity text;

-- Defensif : une base deja peuplee sans cette colonne se retrouve avec des
-- depots locaux coherents des la migration jouee. Un depot avec distant sera
-- rattache par le daemon a son prochain scan (voir plus bas), pas ici : la
-- normalisation d'URL est une regle applicative, pas une regle SQL.
update public.repos
   set identity = 'local:' || root_hash
 where identity is null;

create unique index repos_identite_par_compte
  on public.repos (user_id, identity);

-- Deux clones de la meme origine ne doivent plus produire deux lignes : cette
-- contrainte forcait l'inverse en couplant l'identite a la machine.
alter table public.repos
  drop constraint repos_machine_id_root_hash_key;

-- ------------------------------------------------------------- RLS resserree
-- `repos.machine_id` cesse d'etre une propriete du depot : il devient une
-- propriete de l'*observation*, et suit desormais la derniere machine qui a
-- scanne (comme `current_branch`, `scanned_at`, `loc_total`). Un depot peut
-- donc etre ecrit par n'importe laquelle des machines de son proprietaire, pas
-- seulement celle qui a cree la ligne : la restriction precedente
-- (`machine_active(machine_id)`, qui exigeait l'egalite stricte avec la
-- machine du jeton) interdirait a une seconde machine de rattacher le meme
-- depot, ce que le chantier 1 exige justement de permettre.
--
-- Ce qui reste garanti : un jeton de machine revoquee n'ecrit plus rien, et
-- un compte ne voit jamais les depots d'un autre.
create or replace function public.machine_du_jeton_valide()
returns boolean
language sql
stable
set search_path = public, pg_catalog
as $$
  select
    public.machine_du_jeton() is null
    or exists (
      select 1 from public.machines m
      where m.id = public.machine_du_jeton() and m.revoked_at is null
    );
$$;

drop policy repos_select_own on public.repos;
drop policy repos_insert_own on public.repos;
drop policy repos_update_own on public.repos;
drop policy repos_delete_own on public.repos;

create policy repos_select_own on public.repos
  for select using (
    auth.uid() = user_id and public.machine_du_jeton_valide()
  );

create policy repos_insert_own on public.repos
  for insert with check (
    auth.uid() = user_id and public.machine_du_jeton_valide()
  );

create policy repos_update_own on public.repos
  for update using (
    auth.uid() = user_id and public.machine_du_jeton_valide()
  ) with check (
    auth.uid() = user_id and public.machine_du_jeton_valide()
  );

create policy repos_delete_own on public.repos
  for delete using (
    auth.uid() = user_id and public.machine_du_jeton_valide()
  );

-- `modules`, `sessions`, `worktrees` et `activity_events` suivent tous leur
-- depot via cette fonction : la faire pivoter sur la meme regle leur evite de
-- devoir chacune leur propre migration.
create or replace function public.repo_accessible(p_repo_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public, pg_catalog
as $$
  select exists (
    select 1
    from public.repos r
    where r.id = p_repo_id
      and r.user_id = auth.uid()
      and public.machine_du_jeton_valide()
  );
$$;
