-- Declarer une machine sans code d'appairage (issue #65, FR-017 a FR-022, FR-055).
--
-- Le code d'appairage n'existait que pour faire se reconnaitre deux objets
-- separes : une page web d'un cote, un binaire de terminal de l'autre. Ils n'en
-- font plus qu'un. L'appelant presente desormais la SESSION de l'utilisateur,
-- et n'a plus rien a recopier.
--
-- Ce fichier est la soeur d'`appairer_machine` (20260803000001_appairage.sql) :
-- meme mecanique - creer la ligne de machine, signer son jeton -, meme jeton
-- sans expiration adosse a `machines.revoked_at`. Deux differences, et elles
-- portent tout :
--
--   1. la porte d'entree n'est plus un code a usage unique mais une session
--      d'utilisateur, donc `declarer_machine` est SECURITY INVOKER : c'est la
--      RLS de `machines`, deja ecrite, qui tranche qui a le droit de creer une
--      ligne, et non une garde recopiee dans le corps de la fonction.
--      `machines_insert_own` exige `auth.uid() = user_id` ET
--      `machine_du_jeton() is null` : une session ordinaire passe, un jeton de
--      machine ne passe pas ;
--   2. la charge se reduit au nom de la machine et a sa plateforme (FR-022).
--      Aucun chemin, aucune racine surveillee, rien du poste. La liste fermee
--      de ce qui sort de la machine ne gagne aucune ligne.

-- ------------------------------------------------- le jeton d'une machine a soi
-- La signature elle-meme ne peut pas etre invoker : `interne.signer_jwt` n'est
-- appelable par personne, et surtout pas par `authenticated` - sans quoi
-- n'importe qui signerait n'importe quelle charge. Elle est donc isolee ici,
-- en SECURITY DEFINER, avec la seule garde qui compte : on ne signe que pour
-- une machine que la session appelante possede DEJA.
--
-- Cette fonction sert deux fois : a la declaration, juste apres l'insertion, et
-- au lancement d'un poste dont le trousseau a ete vide. Ce second cas est la
-- raison d'etre de FR-055 : l'identifiant conserve localement permet de
-- redemander un jeton pour LA MEME machine, la ou une reinstallation en
-- fabriquerait une seconde a chaque fois.
create or replace function public.jeton_de_machine(p_machine uuid)
returns text
language plpgsql
security definer
set search_path = public, interne, pg_catalog
as $$
declare
  machine public.machines;
begin
  if auth.uid() is null then
    raise exception 'il faut une session ouverte pour obtenir le jeton d''une machine. Connecte-toi dans l''application Vibe Map.';
  end if;

  -- Un jeton de machine ne fabrique pas de jetons. Il ne represente plus
  -- l'utilisateur entier, et lui laisser signer reviendrait a lui rendre les
  -- droits que la RLS resserree lui a retires.
  if public.machine_du_jeton() is not null then
    raise exception 'un jeton de machine ne peut pas en signer un autre : cette demande passe par la session de l''utilisateur.';
  end if;

  select * into machine
  from public.machines
  where id = p_machine and user_id = auth.uid();

  if not found then
    raise exception 'cette machine n''appartient a aucun compte connu. Elle a peut-etre ete supprimee depuis l''application web.';
  end if;

  if machine.revoked_at is not null then
    raise exception 'cette machine a ete revoquee depuis l''application web. Retire la revocation pour la faire repartir.';
  end if;

  return interne.signer_jwt(jsonb_build_object(
    'sub',        machine.user_id,
    'role',       'authenticated',
    'aud',        'authenticated',
    'machine_id', machine.id
  ));
end;
$$;

revoke execute on function public.jeton_de_machine(uuid) from public, anon;
grant execute on function public.jeton_de_machine(uuid) to authenticated;

-- ------------------------------------------------------- declarer une machine
-- SECURITY INVOKER, a dessein : c'est la ligne `insert` ci-dessous qui traverse
-- la RLS, et c'est elle qui refuse une declaration sans session ou faite avec
-- un jeton de machine. Recopier ces regles dans le corps de la fonction les
-- ferait diverger de la policy le jour ou l'une des deux change.
--
-- Le garde-fou sur `auth.uid()` n'est pas cette regle-la : sans session, la
-- colonne `user_id` serait nulle et Postgres se plaindrait d'une contrainte
-- avant meme que la RLS n'ait son mot a dire. On prefere une phrase qui dit
-- quoi faire a un message de contrainte.
create or replace function public.declarer_machine(
  p_label    text,
  p_platform text default null
)
returns jsonb
language plpgsql
set search_path = public, pg_catalog
as $$
declare
  machine public.machines;
begin
  if auth.uid() is null then
    raise exception 'il faut une session ouverte pour declarer cette machine. Connecte-toi dans l''application Vibe Map.';
  end if;

  if p_label is null or btrim(p_label) = '' then
    raise exception 'il faut un nom de machine';
  end if;

  insert into public.machines (user_id, label, platform)
  values (auth.uid(), btrim(p_label), p_platform)
  returning * into machine;

  return jsonb_build_object(
    'machine_id', machine.id,
    'label',      machine.label,
    'token',      public.jeton_de_machine(machine.id)
  );
end;
$$;

revoke execute on function public.declarer_machine(text, text) from public, anon;
grant execute on function public.declarer_machine(text, text) to authenticated;
