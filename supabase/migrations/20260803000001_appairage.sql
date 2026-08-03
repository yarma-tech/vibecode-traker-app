-- Appairage d'une machine (issue #9).
--
-- Le web cree un code court. Le daemon l'echange contre un jeton porteur de
-- son identifiant de machine. Tout se passe en base : aucun service
-- supplementaire, et le daemon continue d'ecrire directement dans PostgREST.
--
-- Le jeton n'expire pas. C'est un choix assume : la protection ne vient pas
-- de l'expiration mais de `machines.revoked_at`, verifie a chaque ecriture par
-- la RLS. La revocation coupe donc l'acces immediatement.

-- ---------------------------------------------------------------- schema prive
-- Ce qui vit ici n'est PAS expose par PostgREST : la signature des jetons ne
-- doit etre appelable par personne, seulement par nos fonctions.
create schema if not exists interne;
revoke all on schema interne from anon, authenticated;

-- --------------------------------------------------------------- signature JWT
create or replace function interne.signer_jwt(charge jsonb)
returns text
language sql
stable
set search_path = interne, extensions, pg_catalog
as $$
  with parties as (
    select
      translate(
        encode(convert_to('{"alg":"HS256","typ":"JWT"}', 'utf8'), 'base64'),
        E'+/=\n', '-_'
      ) as entete,
      translate(
        encode(convert_to(charge::text, 'utf8'), 'base64'),
        E'+/=\n', '-_'
      ) as corps
  )
  select
    entete || '.' || corps || '.' || translate(
      encode(
        extensions.hmac(
          entete || '.' || corps,
          current_setting('app.settings.jwt_secret'),
          'sha256'
        ),
        'base64'
      ),
      E'+/=\n', '-_'
    )
  from parties;
$$;

-- ------------------------------------------------------------ codes d'appairage
create table public.pairing_codes (
  code        text primary key,
  user_id     uuid not null references auth.users (id) on delete cascade,
  created_at  timestamptz not null default now(),
  expires_at  timestamptz not null default now() + interval '15 minutes',
  consumed_at timestamptz,
  machine_id  uuid references public.machines (id) on delete set null
);

create index pairing_codes_user_id_idx on public.pairing_codes (user_id);

alter table public.pairing_codes enable row level security;

-- L'utilisateur suit son propre code depuis la page d'appairage. Personne
-- n'ecrit dans cette table a la main : tout passe par les deux fonctions.
grant select on public.pairing_codes to authenticated;
grant all on public.pairing_codes to service_role;

create policy pairing_codes_select_own on public.pairing_codes
  for select using (auth.uid() = user_id);

alter publication supabase_realtime add table public.pairing_codes;

-- ---------------------------------------------------------- creation d'un code
-- Alphabet sans O/0 ni I/1 : le code se lit a voix haute et se retape sans
-- ambiguite. Sept caracteres sur trente-deux, soit environ 34 milliards de
-- combinaisons, valables quinze minutes et utilisables une seule fois.
create or replace function public.creer_code_appairage()
returns public.pairing_codes
language plpgsql
security definer
set search_path = public, pg_catalog
as $$
declare
  alphabet constant text := '23456789ABCDEFGHJKLMNPQRSTUVWXYZ';
  brut     text := '';
  i        int;
  ligne    public.pairing_codes;
begin
  if auth.uid() is null then
    raise exception 'il faut etre connecte pour creer un code d''appairage';
  end if;

  for i in 1..7 loop
    brut := brut || substr(alphabet, 1 + floor(random() * length(alphabet))::int, 1);
  end loop;

  insert into public.pairing_codes (code, user_id)
  values (substr(brut, 1, 3) || '-' || substr(brut, 4), auth.uid())
  returning * into ligne;

  return ligne;
end;
$$;

revoke execute on function public.creer_code_appairage() from public, anon;
grant execute on function public.creer_code_appairage() to authenticated;

-- -------------------------------------------------------- echange code / jeton
-- Appelee par le daemon, qui n'a aucune session : elle est donc ouverte a
-- `anon`. Sa seule porte d'entree est un code court, a usage unique et de
-- duree limitee.
create or replace function public.appairer_machine(
  p_code     text,
  p_label    text,
  p_platform text default null
)
returns jsonb
language plpgsql
security definer
set search_path = public, interne, pg_catalog
as $$
declare
  demande public.pairing_codes;
  machine public.machines;
  normalise text;
begin
  if p_label is null or btrim(p_label) = '' then
    raise exception 'il faut un nom de machine';
  end if;

  -- On accepte le code avec ou sans tiret, en minuscules ou en majuscules.
  normalise := upper(replace(btrim(coalesce(p_code, '')), '-', ''));
  normalise := substr(normalise, 1, 3) || '-' || substr(normalise, 4);

  select * into demande
  from public.pairing_codes
  where code = normalise
  for update;

  if not found then
    raise exception 'code d''appairage inconnu. Verifie la saisie, ou demande un nouveau code depuis l''application web.';
  end if;

  if demande.consumed_at is not null then
    raise exception 'ce code a deja servi a relier une machine. Demande un nouveau code depuis l''application web.';
  end if;

  if demande.expires_at < now() then
    raise exception 'ce code a expire. Les codes valent quinze minutes : demandes-en un nouveau depuis l''application web.';
  end if;

  insert into public.machines (user_id, label, platform)
  values (demande.user_id, btrim(p_label), p_platform)
  returning * into machine;

  update public.pairing_codes
     set consumed_at = now(),
         machine_id  = machine.id
   where code = demande.code;

  return jsonb_build_object(
    'machine_id', machine.id,
    'label',      machine.label,
    'token',      interne.signer_jwt(jsonb_build_object(
                    'sub',        demande.user_id,
                    'role',       'authenticated',
                    'aud',        'authenticated',
                    'machine_id', machine.id
                  ))
  );
end;
$$;

revoke execute on function public.appairer_machine(text, text, text) from public;
grant execute on function public.appairer_machine(text, text, text) to anon, authenticated;

-- ------------------------------------------------------------- RLS resserree
-- Un jeton porteur d'un `machine_id` ne represente plus l'utilisateur entier :
-- il ne peut voir et modifier QUE sa propre ligne, et seulement tant qu'elle
-- n'est pas revoquee. Une session web ordinaire, elle, garde tous ses droits.
create or replace function public.machine_du_jeton()
returns uuid
language sql
stable
set search_path = public, pg_catalog
as $$
  select nullif(auth.jwt() ->> 'machine_id', '')::uuid;
$$;

drop policy machines_select_own on public.machines;
drop policy machines_insert_own on public.machines;
drop policy machines_update_own on public.machines;
drop policy machines_delete_own on public.machines;

create policy machines_select_own on public.machines
  for select using (
    auth.uid() = user_id
    and (public.machine_du_jeton() is null or public.machine_du_jeton() = id)
  );

-- Creer une machine passe par l'appairage, jamais par un jeton de machine.
create policy machines_insert_own on public.machines
  for insert with check (
    auth.uid() = user_id and public.machine_du_jeton() is null
  );

create policy machines_update_own on public.machines
  for update using (
    auth.uid() = user_id
    and (
      public.machine_du_jeton() is null
      or (public.machine_du_jeton() = id and revoked_at is null)
    )
  ) with check (
    auth.uid() = user_id
    and (
      public.machine_du_jeton() is null
      or (public.machine_du_jeton() = id and revoked_at is null)
    )
  );

-- Supprimer ou revoquer une machine reste une action humaine.
create policy machines_delete_own on public.machines
  for delete using (
    auth.uid() = user_id and public.machine_du_jeton() is null
  );
