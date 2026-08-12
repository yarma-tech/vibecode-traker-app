# Espace projet Vibe Map : conception

Date : 2026-08-11
État : à relire
Répond à : [`docs/prd/PRD-001-espace-projet-kanban.md`](../../prd/PRD-001-espace-projet-kanban.md)
Remplace : `2026-08-09-espace-projet-kanban-design.md`, écrit sur un modèle à un seul niveau dont presque rien ne survit
S'appuie sur : `2026-08-01-vibe-map-observabilite-web-design.md` (daemon, télémétrie, RLS)

## 1. Ce que ce document décide

Le PRD dit *quoi* et *pourquoi*. Celui-ci dit *comment*, et rien d'autre. Là où une décision produit reste à prendre, il renvoie au PRD au lieu d'inventer.

Cinq chantiers, dans l'ordre où ils se livrent :

| # | Chantier | Livre |
|---|---|---|
| 1 | Identité des dépôts par le distant | un tableau qui survit à un `mv` |
| 2 | Blocs, issues, références | le tableau, saisi à la main, sans automatisme |
| 3 | Ingestion des commits | la fermeture par référence |
| 4 | Lecture des PRD | « À faire » qui se peuple tout seul |
| 5 | Vérification par sous-agent | le rattrapage des références oubliées |

Chaque chantier laisse le produit utilisable. Le 5 est le seul qui ouvre une voie réseau nouvelle ; il a sa section de sécurité (§8) et ne se code pas avant qu'elle soit relue.

## 2. Ce qui existe déjà, et qu'on ne refait pas

Vérifié dans le dépôt au 2026-08-11 :

- `repos` porte **`remote_url` et `remote_owner`** (`20260803000002_repos_modules.sql`) : le distant voyage déjà, le chantier 1 ne pose donc aucune question de confidentialité neuve.
- `activity_events` porte `repo_id`, `module_path`, `file_path`, `kind`, `occurred_at`, avec `unique (session_id, tool_use_id)` : le signal qui **entame** un travail existe, il n'y a rien à ajouter côté daemon pour ça.
- `repo_accessible(repo_id)` et `machine_active(machine_id)` sont écrites et éprouvées : toute nouvelle table s'y raccroche au lieu d'inventer sa RLS.
- Le daemon a trois cadences (`interval_seconds`, `scan_seconds = 300`, `journal_seconds = 2`) dans un `tokio::select!` (`daemon/src/main.rs`), et un helper `git()` dans `plan.rs`.
- **Rien du Kanban n'est écrit** : aucune migration `tasks`, aucun `commits.rs`. C'est un terrain vierge, pas une migration de données.

## 3. Chantier 1 — l'identité d'un dépôt

**Décision du PRD (§10.1).** Un dépôt est le même d'une machine à l'autre et survit à un déplacement de dossier.

`repos.root_hash` reste ce qu'il est — l'empreinte du chemin absolu, propre à un clone. On lui ajoute une identité **logique** :

```sql
alter table public.repos
  add column identity text;                    -- URL distante normalisee, ou 'local:' || root_hash

create unique index repos_identite_par_compte
  on public.repos (user_id, identity);
```

`unique (machine_id, root_hash)` disparaît : deux clones de la même origine ne doivent plus produire deux lignes.

**Normalisation** (dans `plan.rs`, testée sur fixtures) : minuscules, schéma et identifiants retirés, `.git` final retiré, `:` de la forme SSH remplacé par `/`, barre finale retirée.

```
git@github.com:Yarma-Tech/vibecode-traker-app.git  ┐
https://github.com/yarma-tech/vibecode-traker-app  ├─→  github.com/yarma-tech/vibecode-traker-app
https://user@github.com/yarma-tech/vibecode-traker-app.git/ ┘
```

Sans distant : `identity = 'local:' || root_hash`. Si un distant apparaît plus tard, le daemon **rattache** la ligne existante — il met `identity` à jour au lieu d'en créer une seconde. C'est la seule migration de données du lot, et elle est locale à une ligne.

**Ce que ça déplace ailleurs.** `repos.machine_id` cesse d'être une propriété du dépôt : il devient une propriété de l'*observation*. Les colonnes qui décrivent un clone (`current_branch`, `scanned_at`, `loc_total`) suivent la dernière machine qui a scanné, et c'est acceptable — elles n'ont jamais servi qu'à afficher un ordre de grandeur. La machine qui fait foi pour la vérification (§8) se choisit sur l'activité récente, pas sur cette colonne.

## 4. Chantier 2 — le modèle

### 4.1 `blocs`

```sql
create table public.blocs (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null default auth.uid()
                references auth.users (id) on delete cascade,
  repo_id       uuid not null references public.repos (id) on delete cascade,
  ref           int  not null,                    -- VM-7 : le 7
  type          text not null default 'feature'
                check (type in ('feature','correction','technique','exploration')),
  titre         text not null,
  statut        text not null default 'todo'
                check (statut in ('todo','doing','done')),
  version       int  not null default 1,
  -- Emplacement du bloc simple. Null des qu'il est decoupe (§4.3).
  chemin        text,
  -- Provenance. Null = saisi a la main.
  prd_cle       text,                             -- '2026-08-10/PRD-004/F1'
  prd_priorite  text,                             -- 'P1'
  prd_a_clarifier boolean not null default false,
  prd_statut    text,
  prd_maj       date,
  prd_valide_le date,
  prd_absent    boolean not null default false,   -- retiree du PRD, carte conservee (regle 7)
  position      int  not null default 0,
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now(),
  unique (repo_id, ref),
  unique (repo_id, prd_cle)
);
```

`unique (repo_id, prd_cle)` est ce qui rend la relecture d'un PRD idempotente : relire le même document ne crée rien de neuf.

### 4.2 `issues`

```sql
create table public.issues (
  id         uuid primary key default gen_random_uuid(),
  user_id    uuid not null default auth.uid()
             references auth.users (id) on delete cascade,
  repo_id    uuid not null references public.repos (id) on delete cascade,
  bloc_id    uuid not null references public.blocs (id) on delete cascade,
  ref        int  not null,
  titre      text not null,
  chemin     text not null,                       -- relatif racine ; '' = racine
  statut     text not null default 'todo'
             check (statut in ('todo','doing','done')),
  version    int  not null default 1,
  position   int  not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (repo_id, ref)
);

create index issues_bloc_idx  on public.issues (bloc_id);
create index issues_route_idx on public.issues (repo_id, chemin);
```

**Les deux `ref` puisent dans le même compteur.** Un `VM-7` désigne un bloc *ou* une issue, jamais les deux : c'est ce qui permet à un message de commit de nommer l'un ou l'autre sans que le lecteur ait à savoir lequel.

```sql
create table public.compteur_ref (
  repo_id  uuid primary key references public.repos (id) on delete cascade,
  prochain int not null default 1
);
```

Attribution par `update … set prochain = prochain + 1 returning prochain - 1`, jamais par `max(ref) + 1` : **une référence supprimée n'est pas réattribuée** (PRD §6.3), sinon un vieux message de commit pourrait fermer une carte neuve.

### 4.3 Les invariants, tenus par la base

Ce que le modèle interdit, plutôt que ce que l'application promet :

```sql
-- Un bloc a un chemin, ou des issues, jamais les deux (PRD A3).
create or replace function public.bloc_coherent() returns trigger
language plpgsql security definer set search_path = public, pg_catalog as $$
begin
  if exists (select 1 from public.issues i where i.bloc_id = new.bloc_id)
     and (select b.chemin from public.blocs b where b.id = new.bloc_id) is not null
  then
    update public.blocs set chemin = null where id = new.bloc_id;
  end if;
  return new;
end $$;
```

La première issue insérée fait donc **descendre** le chemin du bloc, au lieu de refuser l'insertion. C'est le comportement décrit par le PRD : on découpe après coup et rien ne se perd.

**Pas d'index d'unicité sur `(repo_id, chemin)`.** Six issues peuvent partager un dossier : c'était la contrainte du spec précédent, elle n'a plus d'objet depuis que la fermeture passe par la référence.

### 4.4 L'état d'un bloc découpé est dérivé

```sql
create or replace function public.etat_bloc(p_bloc_id uuid) returns void
language plpgsql security definer set search_path = public, pg_catalog as $$
declare v_total int; v_done int; v_doing int;
begin
  select count(*), count(*) filter (where statut = 'done'),
         count(*) filter (where statut = 'doing')
    into v_total, v_done, v_doing
    from public.issues where bloc_id = p_bloc_id;

  if v_total = 0 then return; end if;             -- bloc simple : son statut lui appartient

  update public.blocs set
    statut = case when v_done = v_total then 'done'
                  when v_doing > 0 or v_done > 0 then 'doing'
                  else 'todo' end,
    updated_at = now()
  where id = p_bloc_id;
end $$;
```

Appelée par trigger après tout changement d'une issue. Un bloc découpé n'est donc jamais écrit directement — ni par le web, ni par la machine à états.

## 5. Chantier 3 — les commits

### 5.1 Tables

```sql
create table public.commits (
  id          uuid primary key default gen_random_uuid(),
  repo_id     uuid not null references public.repos (id) on delete cascade,
  sha         text not null,
  message     text not null,
  branch      text,
  authored_at timestamptz not null,
  created_at  timestamptz not null default now(),
  unique (repo_id, sha)
);

create table public.fermetures (
  bloc_id   uuid references public.blocs (id)  on delete cascade,
  issue_id  uuid references public.issues (id) on delete cascade,
  commit_id uuid not null references public.commits (id) on delete cascade,
  version   int  not null,
  ferme_le  timestamptz not null default now(),
  check (num_nonnulls(bloc_id, issue_id) = 1)
);
```

Pas de `commit_paths` : les chemins d'un commit ne servaient qu'à router la fermeture, et la fermeture ne route plus par chemin.

### 5.2 Le module `daemon/src/commits.rs`

Cadence propre — **30 s**, entre les journaux (2 s) et la cartographie (300 s). Le PRD promet la fermeture en moins d'une minute (NF1) ; la boucle de cartographie ne pouvait pas la tenir.

Par dépôt :

1. Lire `repos.last_commit_sha`. Absent → poser `HEAD` et ne rien poster (le passé n'est pas rejoué, PRD règle 10).
2. `git log <sha>..HEAD --no-merges --format=…`, du plus ancien au plus récent.
3. Pour chaque commit : `insert … on conflict (repo_id, sha) do nothing`, puis `select fermer_par_reference(<commit_id>)`.
4. `update repos.last_commit_sha = HEAD`.

Un commit par transaction, dans l'ordre chronologique. Idempotent par `unique (repo_id, sha)` ; si l'insertion ne crée rien, la fermeture n'est pas appelée.

**Le message part en clair** — il le fait déjà pour les commits affichés ailleurs, et c'est lui qui porte la référence. C'est une ligne de la liste fermée (PRD §7.1).

### 5.3 Fermer

```sql
create or replace function public.fermer_par_reference(p_commit_id uuid) returns void
language plpgsql security definer set search_path = public, pg_catalog as $$
declare v_repo uuid; v_message text; v_ref int;
begin
  select repo_id, message into v_repo, v_message
    from public.commits where id = p_commit_id;

  for v_ref in
    select distinct (regexp_matches(v_message, 'VM-([0-9]+)', 'g'))[1]::int
  loop
    update public.issues
       set statut = case when statut = 'done' then 'doing' else 'done' end,
           version = case when statut = 'done' then version + 1 else version end,
           updated_at = now()
     where repo_id = v_repo and ref = v_ref;
    -- meme traitement sur blocs sans issues, puis insert dans fermetures
  end loop;
end $$;
```

Trois comportements que la fonction doit tenir, tous décidés par le PRD :

- une référence qui nomme un travail **déjà terminé** le rouvre en version suivante ;
- une référence inconnue est ignorée sans bruit ;
- un commit qui ne nomme rien ne ferme rien, quels que soient les fichiers qu'il touche.

Le motif est `VM-([0-9]+)`, **jamais `#([0-9]+)`** : les messages de ce dépôt contiennent déjà `feat(#7):` et `Merge pull request #17`, qui désignent des issues GitHub.

### 5.4 Entamer

Sur insertion dans `activity_events`, `kind = 'write'` : route `file_path` vers l'issue vivante à l'emplacement le plus profond qui le préfixe, et la passe en `doing` si elle est en `todo`.

Le routage par chemin **ne sert qu'à entamer**. Il ne ferme pas, il ne rouvre pas une issue terminée : écrire dans un dossier n'est pas reprendre un travail livré (PRD §6.3).

## 6. Chantier 4 — lire les PRD

`daemon/src/prd.rs`, dans la boucle de cartographie (300 s : un PRD ne change pas toutes les trente secondes).

1. Repérer les `*.md` dont l'en-tête YAML porte `id`, `statut`, `date`, `repo`. Pas d'en-tête → ignoré.
2. `repo` différent du dépôt courant → ignoré, le document dit pour qui il est écrit.
3. Selon `statut` :
   - `draft` → **un** bloc `type = 'exploration'`, ancré au fichier du PRD, titré « cadrage <titre> ». Aucune feature.
   - `validé` et au-delà → l'exploration cède la place aux features (voir « la conversion » ci-dessous).
   - `abandonné` → plus de création ; les blocs existants passent `prd_absent = true`.
4. Pour chaque `### Fn — titre (Priorité : Pn)` : upsert sur `(repo_id, prd_cle)` avec `prd_cle = <date>/<id>/Fn`.
5. Une feature disparue du document : `prd_absent = true`, jamais de `delete` (PRD règle 7).

**La conversion de l'exploration.** Le PRD (§5.1) veut qu'elle *devienne* les features sans laisser de carte. Traduction : à la première lecture d'un PRD passé en `validé`, la fonction supprime le bloc d'exploration **s'il est encore vierge** — statut `todo` ou `doing`, aucune issue, aucune fermeture — et crée les features dans la même transaction. S'il porte quelque chose (une issue ajoutée à la main, une fermeture), il est conservé et marqué converti : on ne détruit pas du travail, même pour respecter une règle d'affichage.

**Ce qui sort de la machine** : `(clé, titre, priorité, à-clarifier, statut, valide_le, maj)`. Ni user story, ni exigences, ni critères d'acceptation. Le parseur les traverse sans les transmettre.

**Le parseur est déterministe et testé sur fixtures.** C'est la raison d'être du choix (PRD §6.2) : un modèle reformulerait à chaque passage et fabriquerait des doublons.

## 7. La fraîcheur

`repos.scanned_at` existe ; `machines.last_seen_at` aussi. L'espace projet en dérive un état affichable :

- **frais** — battement de la machine il y a moins de deux minutes ;
- **muet depuis N** — sinon, avec la durée.

Rien de neuf en base. C'est ce qui permet à §6.6 du PRD de distinguer « rien à faire » de « rien ne remonte », et donc de n'afficher son clin d'œil que quand il est vrai.

Même mécanisme pour F16 : un travail dont le `chemin` n'existe plus dans `modules` après une cartographie est signalé — une jointure, pas une colonne.

## 8. Chantier 5 — la vérification (voie retour)

**Cette section est la seule qui ouvre une porte nouvelle. Elle se relit avant de se coder.**

```sql
create table public.verifications (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid not null default auth.uid() references auth.users (id) on delete cascade,
  repo_id     uuid not null references public.repos (id) on delete cascade,
  cible_bloc  uuid references public.blocs (id)  on delete cascade,
  cible_issue uuid references public.issues (id) on delete cascade,
  machine_id  uuid references public.machines (id) on delete set null,
  etat        text not null default 'demandee'
              check (etat in ('demandee','en_cours','rendue','abandonnee')),
  verdict     text check (verdict in ('implemente','partiel','introuvable')),
  confiance   text check (confiance in ('haute','moyenne','basse')),
  chemins     text[],
  phrase      text,
  jetons      bigint,
  demandee_le timestamptz not null default now(),
  rendue_le   timestamptz,
  check (num_nonnulls(cible_bloc, cible_issue) = 1)
);
```

**Aucune colonne de prompt.** C'est la garantie structurelle, la même que celle d'`activity_events` : une ligne ne peut pas transporter d'instruction parce qu'il n'y a pas de colonne pour l'accueillir. Le prompt du sous-agent est **écrit dans le binaire du daemon**.

Ce que le daemon fait en voyant une ligne `demandee` qui le désigne :

1. la passe `en_cours` (verrou : une seule par dépôt à la fois) ;
2. lance un sous-agent **en lecture seule**, sur le dépôt local, avec son prompt à lui et la seule variable qu'il accepte du serveur : quel travail vérifier (titre, chemin) ;
3. écrit `verdict`, `confiance`, `chemins`, `phrase` (plafonnée), `jetons` ;
4. au-delà du délai maximal, la ligne passe `abandonnee` et le dit à l'écran.

**Choix de la machine** : celle dont une session a écrit le plus récemment sur ce dépôt. Si elle ne rend rien dans le délai, l'écran propose la suivante — il ne bascule pas tout seul.

**Le verdict ne ferme jamais.** Il fait apparaître un bouton *Fermer*, qui est le seul du produit (PRD §6.3).

**RLS** : `insert` réservé au compte utilisateur via `repo_accessible`, `update` du verdict réservé à la machine désignée via `machine_active`. Une machine révoquée ne peut donc ni prendre ni rendre une vérification.

## 9. RLS

Identique en tout point à `modules` : les cinq tables suivent leur dépôt via `repo_accessible(repo_id)`, quatre policies chacune, `grant` à `authenticated` et `service_role`, ajout à `supabase_realtime`.

Deux exceptions :

- `fermetures` : `select` seulement ; les lignes n'y entrent que par `fermer_par_reference`, en `security definer` ;
- `verifications` : voir §8.

## 10. Le web

Route `/repo/[id]/projet`, lien depuis `/repo/[id]` et depuis chaque ligne de l'accueil.

**Lire `node_modules/next/dist/docs/` avant d'écrire quoi que ce soit** — cette version de Next a des ruptures d'API (`web/AGENTS.md`), et rien de ce qui suit ne dispense de cette lecture.

- Abonnement Realtime sur `blocs`, `issues`, `fermetures`, comme `direct.tsx`.
- Trois colonnes, cartes selon §6.5 du PRD : hiérarchie, pas empilement. Titre et avancement de loin ; origine et priorité de près ; référence au moment de lancer un agent.
- **Pas de glisser vers « Terminé ».** L'interface ne propose pas le geste ; le serveur le refuse aussi — une policy `update` qui interdit `statut = 'done'` depuis le web sauf après un verdict rendu.
- Clavier d'abord : créer, déplacer, ouvrir, copier une référence sans souris.
- Trois états vides distincts (PRD §6.6), dont le clin d'œil n'est montré **que** si la fraîcheur de §7 le permet.

## 11. Ordre de livraison

| Étape | Contenu | Le produit après |
|---|---|---|
| 1 | §3 identité | rien de visible, mais les tableaux qui suivront survivront à un `mv` |
| 2 | §4 + §9 + §10 sans automatisme | un Kanban manuel, utilisable, sans mensonge possible |
| 3 | §5 commits | les cartes se ferment toutes seules |
| 4 | §6 PRD | « À faire » se peuple tout seul |
| 5 | §7 fraîcheur | le tableau cesse de pouvoir mentir en silence |
| 6 | §8 vérification | le rattrapage des références oubliées |

L'étape 1 est en premier parce qu'elle est la seule qui coûte cher **après** : changer l'identité d'un dépôt quand des cartes existent demande de deviner quelle ancienne ligne correspond à quelle nouvelle.

L'étape 4 emporte la réécriture de `README.md` (l. 12-15) : c'est le commit qui rend la promesse actuelle fausse, c'est donc celui qui doit la corriger (PRD §7.1).

## 12. Tests

- **Normalisation d'URL** : les quatre formes du §3 donnent une seule identité ; un dépôt sans distant reste isolé ; l'ajout d'un distant rattache au lieu de dupliquer.
- **Références** : jamais réattribuées après suppression ; blocs et issues puisent au même compteur ; `VM-7` ferme, `#7` ne ferme pas — fixture avec les vrais messages du dépôt (`feat(#7):`, `Merge pull request #17`).
- **Fermeture** : un commit nommant six références en ferme six ; un commit sans référence ne ferme rien même s'il touche l'emplacement ; une référence sur un travail terminé rouvre en `version + 1` ; référence inconnue ignorée.
- **Entamer** : une écriture passe l'issue en `doing` ; elle ne rouvre pas une issue terminée ; six issues d'un même dossier passent toutes en `doing`.
- **Dérivation** : un bloc découpé suit ses issues ; la première issue fait descendre le chemin du bloc ; un bloc simple garde son statut propre.
- **PRD** : `draft` → un bloc exploration ; `validé` → conversion et N features ; relecture idempotente ; feature retirée → `prd_absent`, jamais de suppression ; `repo` étranger ignoré ; document sans en-tête ignoré.
- **RLS** : un compte ne voit rien d'un autre ; une machine révoquée n'insère plus ; le web ne peut pas écrire `statut = 'done'` sans verdict.
- **Vérification** : une seule en cours par dépôt ; délai dépassé → `abandonnee` ; la table ne peut pas transporter de prompt (test de schéma, pas de comportement).
- **Web** : glisser vers « Terminé » impossible ; sortie de « Terminé » possible ; clin d'œil absent quand la fraîcheur manque ; parcours clavier complet.

## 13. Ce que ce document ne tranche pas

- Le dessin des trois états vides et l'illustration du tableau vidé (PRD §6.6).
- La hiérarchie visuelle exacte des marques d'une carte (PRD §6.5).
- Le prompt du sous-agent de vérification, qui vit dans le daemon et se règle à l'usage.
