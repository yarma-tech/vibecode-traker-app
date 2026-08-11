# Espace projet Vibe Map (Kanban lié au git) : spec de conception

Date : 2026-08-09
État : **remplacé** le 2026-08-11 par [`2026-08-11-espace-projet-kanban-conception.md`](2026-08-11-espace-projet-kanban-conception.md)

> Ce document décrivait un modèle à un seul niveau : une tâche, un chemin, une
> fermeture déduite des fichiers touchés par un commit. Le PRD écrit ensuite
> ([`docs/PRD-espace-projet-kanban.md`](../../PRD-espace-projet-kanban.md)) a
> retenu deux niveaux (bloc et issue), une fermeture par référence explicite, et
> une identité de dépôt fondée sur le distant. Presque rien de ce qui suit ne
> survit. Conservé pour l'historique du raisonnement, pas pour être implémenté.
S'appuie sur : `docs/superpowers/specs/2026-08-01-vibe-map-observabilite-web-design.md` (télémétrie, daemon, RLS)

## 1. Ce qu'on construit

Un espace de gestion de projet greffé sur Vibe Map. On choisit un dépôt, on y suit ses fonctionnalités sur un tableau Kanban à trois colonnes — **À faire · En cours · Terminé** — et le tableau se met à jour tout seul à partir de ce que font déjà les agents et de ce qui est committé.

Le principe : une fonctionnalité prévue (issue d'un PRD ou d'un backlog) attend en **À faire**. Quand un agent se met à écrire dessus, elle passe en **En cours** — on réutilise le système d'activité existant. Quand un commit la réalise, elle passe en **Terminé**. Une fonctionnalité qui itère (landing page V1 committée, puis reprise en V2) se **versionne** sur la même carte : elle ne revient jamais en « À faire ».

Ce n'est pas un gestionnaire de tickets généraliste. C'est un miroir git pour un développeur solo qui lance des agents : le tableau reflète l'état réel du code, il ne le pilote pas. Pas de dates, pas d'assignation, pas de sous-tâches.

Le contexte produit (utilisateur, personnalité, anti-références, principes de couleur et d'accessibilité) vit dans `PRODUCT.md`. La direction visuelle des états reprend les trois teintes du diagramme de séquence publié le 2026-08-09.

Critère de réussite : après un commit sur une machine, la carte correspondante glisse en « Terminé » sur un autre appareil sans que personne ait touché au tableau.

## 2. Décisions actées

| # | Décision | Motif |
|---|---|---|
| 1 | Une tâche est ancrée à un **chemin exact** (dossier ou fichier), relatif à la racine | Réutilise le grain des `modules` et de la télémétrie ; permet le routage « sous-arbre exact ». |
| 2 | « Terminé » se peuple des **commits** ; « En cours » de l'**activité** d'agent | Les deux signaux existent déjà (ou presque) dans la télémétrie ; rien à saisir à la main pour avancer. |
| 3 | État **stocké**, transitions par **triggers Postgres** | Le verrou « En cours » et le versionnage ont besoin d'un point stocké ; les triggers font tomber les transitions même sans fenêtre web ouverte, multi-machines. |
| 4 | Transitions **en avant seulement** en auto : `todo→doing`, `→done`, `done→doing v+1` | Traduit la règle métier « une itération ne revient jamais en À faire ». |
| 5 | Routage d'un commit vers la tâche à l'**ancre la plus profonde** qui matche | Deux tâches sous un même dossier parent restent distinctes tant que leurs ancres diffèrent. |
| 6 | Au plus **une tâche vivante (non-`done`) par ancre exacte** | Lève l'ambiguïté « quelle tâche ce commit ferme-t-il » sans demander à l'utilisateur. |
| 7 | Le daemon poste les commits, **HEAD-seulement** au premier passage | Pas de bruit rétroactif : « Terminé » se peuple à partir des commits futurs. |
| 8 | Le daemon ne poste que **sha, message, branche, date, chemins relatifs** | Cohérent avec la doctrine `root_hash` : ni diff, ni contenu, ni chemin absolu ne quittent la machine. |
| 9 | **Drag manuel libre**, les triggers n'avancent que | Souplesse pour le solo ; un recul manuel peut être ré-avancé par la prochaine activité, les deux cohabitent. |
| 10 | Espace **dédié par repo** (`/repo/[id]/projet`), pas un widget dans la carte | « La carte est le produit » (PRODUCT §5) ; le Kanban est un espace voisin. |
| 11 | Ancre choisie par **autocomplétion sur `modules`** | Chemins connus du repo, saisie clavier rapide, registre Linear. |

## 3. Modèle de données

Trois nouvelles tables porteuses, une table de liaison, une colonne curseur. RLS calquée sur `modules` : les lignes n'ont pas de propriétaire propre, elles suivent leur repo via `repo_accessible(repo_id)`.

### 3.1 `tasks` — les cartes

```
id           uuid  pk  default gen_random_uuid()
user_id      uuid  not null default auth.uid() references auth.users(id) on delete cascade
repo_id      uuid  not null references repos(id) on delete cascade
anchor_path  text  not null              -- chemin exact, relatif racine ('' = racine)
title        text  not null
description  text  null
status       text  not null default 'todo' check (status in ('todo','doing','done'))
version      int   not null default 1
position     int   not null default 0    -- ordre dans la colonne (drag manuel)
created_at   timestamptz not null default now()
updated_at   timestamptz not null default now()
```

Index : `(repo_id, status)` pour le rendu par colonne, `(repo_id, anchor_path)` pour le routage.

Contrainte d'unicité vivante :
```
create unique index tasks_ancre_vivante
  on tasks (repo_id, anchor_path) where status <> 'done';
```
Au plus une tâche non-`done` par ancre exacte dans un repo. Deux tâches sur exactement le même chemin ne coexistent que si l'une est `done` (une V précédente figée, une V suivante en cours — voir §5).

### 3.2 `commits` — faits git bruts (postés par le daemon)

```
id           uuid pk default gen_random_uuid()
repo_id      uuid not null references repos(id) on delete cascade
sha          text not null
message      text not null
branch       text null
authored_at  timestamptz not null
created_at   timestamptz not null default now()
unique (repo_id, sha)
```

### 3.3 `commit_paths` — chemins touchés (pour le routage)

```
commit_id    uuid not null references commits(id) on delete cascade
path         text not null              -- relatif racine ; dossiers touchés ET fichiers exacts
primary key (commit_id, path)
```

On stocke à la fois les fichiers exacts touchés et leurs dossiers parents repliés, pour que le routage « ancre la plus profonde » puisse matcher une ancre dossier *ou* une ancre fichier.

### 3.4 `task_commits` — lien tâche ↔ commit qui l'a terminée

```
task_id      uuid not null references tasks(id) on delete cascade
commit_id    uuid not null references commits(id) on delete cascade
version      int  not null              -- la version de la tâche au moment de la fermeture
closed_at    timestamptz not null default now()
primary key (task_id, commit_id)
```

Une tâche versionnée porte plusieurs lignes : V1 → commit A, V2 → commit B, même `task_id`. C'est l'historique des versions.

### 3.5 Curseur d'ingestion

`repos.last_commit_sha text null` — jusqu'où le daemon a lu pour ce repo. `null` = jamais lu.

## 4. Ingestion des commits (daemon Rust)

Nouveau module `daemon/src/commits.rs`, appelé dans la boucle de scan (au lancement puis toutes les ~5 min), une passe par repo cartographié.

Par repo :

1. Lit `repos.last_commit_sha`.
   - **Curseur `null` (premier passage)** : ne poste aucun commit, pose `last_commit_sha = HEAD`. « Terminé » ne se peuplera qu'à partir des commits suivants.
   - **Curseur présent** : `git log <last_sha>..HEAD --no-merges` → liste des nouveaux commits, du plus ancien au plus récent (sha, message, branche courante, date d'auteur).
2. Pour chaque nouveau commit : `git diff-tree --no-commit-id --name-only -r <sha>` → fichiers touchés. On les replie en chemins relatifs à la racine (mêmes conventions que `plan.rs` : racine = chaîne vide). L'ensemble posté = { fichiers exacts } ∪ { tous les dossiers parents de chaque fichier, jusqu'à la racine }, dédupliqué.
3. **Un commit à la fois, du plus ancien au plus récent.** Pour chaque nouveau commit, en une transaction : `insert commits ... on conflict (repo_id, sha) do nothing` (un seul commit), puis `insert commit_paths` de ce commit **en un seul statement**, puis appel de la fonction de fermeture `select fermer_taches_du_commit(<commit_id>)` (§5.3). Une fois tous les commits traités : `update repos.last_commit_sha = HEAD`.

Cette granularité — un commit par transaction, chemins d'un commit en un statement, ordre chronologique, fermeture appelée explicitement par `commit_id` — est **contractuelle** : elle est ce qui rend la fermeture (§5.3) déterministe et attribuable à un commit unique. Le daemon ne batche pas plusieurs commits dans une même insertion de `commit_paths`.

Propriétés :

- **Idempotent.** `unique (repo_id, sha)` + `on conflict do nothing` : rejouer une passe ne crée pas de doublon (même doctrine que le `tool_use_id` du journal). Si l'insertion du commit ne crée rien (déjà présent), le daemon n'appelle pas la fermeture pour ce sha.
- **Ordre chronologique.** Les commits sont traités du plus ancien au plus récent. Au sein d'une même passe, la version d'une tâche ne change pas d'un commit à l'autre (les incréments de version ne viennent que de l'activité, Trigger 3) ; l'ordre ne détermine donc qu'une chose : quel commit est **crédité** de la fermeture si deux commits d'une même passe touchent la même tâche vivante — le plus ancien la ferme, le suivant ne trouve plus de tâche non-`done` à cette ancre.
- **Rien de sensible.** Ni diff, ni contenu, ni chemin absolu. Voir la doctrine `root_hash` du spec du 2026-08-01.
- **Voie authentifiée existante.** Écrit sous jeton machine → RLS `repo_accessible`. Aucune nouvelle voie réseau.
- **Curseur par repo.** Pas d'état global ; un repo sans nouveau commit ne poste rien.

Cas limites : rebase/amend réécrivant des sha déjà postés → les nouveaux sha sont postés comme des commits distincts (on ne réconcilie pas l'historique réécrit ; acceptable pour un miroir d'avancement). Un repo sans commit (HEAD absent) → passe ignorée.

## 5. Machine à états (triggers Postgres)

Toute la logique de transition vit en base. Fonctions `security definer`, `search_path` verrouillé, comme les fonctions existantes (`repo_accessible`, `etat_modules`).

### 5.1 Routage — quelle tâche pour un chemin

Une ancre A **préfixe** un chemin P si `A = P` ou `P` commence par `A || '/'`, avec `A = ''` (racine) qui matche tout chemin du repo — une tâche ancrée à la racine est donc un fourre-tout qui capte toute activité et tout commit du repo (sharp edge assumé).

Deux fonctions de routage, car activité et commit ne cherchent pas la même chose :

- `tache_pour_event(p_repo_id, p_path) returns uuid` — pour l'activité. Rend la tâche, **tous statuts confondus**, dont l'ancre préfixe `p_path`, la plus profonde d'abord (`length(anchor_path)` décroissant). L'activité doit pouvoir viser une tâche `done` (pour la rouvrir en V2, Trigger 3). Départage à profondeur égale : la tâche non-`done` la plus récemment mise à jour, sinon la `done` la plus récente. (À profondeur égale il ne peut y avoir qu'**une** non-`done` — contrainte §3.1 — donc pas d'ambiguïté entre deux tâches vivantes.)
- `tache_a_fermer(p_repo_id, p_path) returns uuid` — pour le commit. Rend la tâche **non-`done`** dont l'ancre préfixe `p_path`, la plus profonde d'abord. Un commit ferme ; une tâche `done` est déjà fermée et ne se rouvre jamais sur un commit. On cherche donc la tâche *fermable* la plus spécifique.

Ces deux routages **diffèrent volontairement** : c'est le point de §5.5. Cette différence n'est réelle que dans une configuration où une tâche `done` à une ancre profonde coexiste avec une tâche vivante à une ancre plus courte sur les mêmes fichiers ; alors une activité rouvre la `done` profonde (V2) tandis qu'un commit ferme la vivante plus courte. Comportement voulu : l'activité vise le plus spécifique, le commit ferme le plus spécifique *encore ouvert*.

### 5.2 Trigger 1 — `todo → doing` (sur insert `activity_events`)

À chaque event `kind = 'write'` : on route `file_path` (le chemin le plus précis dont on dispose) via `tache_pour_event`. Si la tâche trouvée est `todo` → `status = 'doing'`, `updated_at = now()`. Si elle est `doing`, rien. Si elle est `done`, c'est le Trigger 3. Si aucune tâche ne matche, rien : l'activité colore le plan par ailleurs, indépendamment du Kanban.

### 5.3 Fermeture — `→ done` (fonction `fermer_taches_du_commit`, appelée par le daemon)

Le mécanisme est **figé** : pas de trigger implicite sur `commit_paths` (dont le timing dépendrait du batching du daemon). C'est une fonction explicite `fermer_taches_du_commit(p_commit_id uuid)`, `security definer`, que le daemon appelle une fois par commit juste après avoir inséré ses chemins (§4 step 3). Elle est donc invoquée avec les chemins déjà en base, pour un `commit_id` unique et sans équivoque. Elle :

1. Collecte les `commit_paths` du commit.
2. Route chacun via `tache_a_fermer` (§5.1) — qui, parce que `commit_paths` contient les **dossiers parents repliés** (§4 step 2), atteint aussi bien une tâche ancrée à un fichier qu'une tâche ancrée à un dossier parent. Retient la tâche non-`done` à l'ancre la plus profonde parmi tous les chemins.
3. Si une telle tâche existe : `status = 'done'`, `updated_at = now()`, et `insert task_commits(task_id, p_commit_id, version = tasks.version)`.

Le saut `todo → done` direct est autorisé (agent qui committe sans qu'on ait observé d'event `write`). Un commit qui ne route vers aucune tâche non-`done` ne ferme rien (il reste visible comme fait git, mais ne bouge aucune carte).

### 5.4 Trigger 3 — `done → doing` version suivante (sur insert `activity_events`)

Toujours au sein du Trigger 1 (même event, même appel à `tache_pour_event`) : si la tâche routée est `done` → `version = version + 1`, `status = 'doing'`, `updated_at = now()`. C'est l'itération V2. Jamais `→ todo`. Comme `tache_pour_event` a déjà rendu la tâche la plus profonde tous statuts confondus, il n'y a pas de garde supplémentaire à poser : si une tâche non-`done` plus profonde existait sur ce chemin, `tache_pour_event` l'aurait rendue à sa place et on serait dans le Trigger 1.

### 5.5 Priorité des règles

Pour un **event d'activité** : `tache_pour_event` rend la tâche à l'ancre la plus profonde, tous statuts confondus ; son état courant décide de la transition (`todo→doing` Trigger 1, `done→doing v+1` Trigger 3, `doing` rien). Un event ne touche qu'une tâche.

Pour un **commit** : `tache_a_fermer` rend la tâche non-`done` à l'ancre la plus profonde ; un commit ne ferme qu'une tâche.

Les deux routages ne sont pas identiques (§5.1) : l'activité considère tous les statuts (elle peut rouvrir une `done`), le commit ne considère que les tâches fermables. Dans la configuration ordinaire — au plus une tâche vivante par ancre, pas de `done` masquée sous une vivante — les deux rendent la même tâche et la distinction est invisible. Elle ne compte que dans le cas croisé décrit en §5.1.

### 5.6 Drag manuel

Le web écrit directement `tasks.status` (et `position`) lors d'un drag, dans n'importe quelle direction. Les triggers, eux, n'avancent que. Conséquence assumée : une carte tirée en arrière peut être ré-avancée par la prochaine activité ou le prochain commit ; c'est voulu (décision #9).

## 6. RLS et sécurité

`tasks`, `commits`, `commit_paths`, `task_commits` : RLS activée, `grant` à `authenticated` et `service_role`, ajout à `supabase_realtime`.

- `tasks` : policies select/insert/update/delete gouvernées par `repo_accessible(repo_id)` — mêmes quatre policies que `modules`. Éditées depuis le web sous le compte user.
- `commits` / `commit_paths` : insert/select sous `repo_accessible`. Insérées par le daemon (jeton machine, via `machine_active`), lues par le web.
- `task_commits` : select sous `repo_accessible` via jointure ; insert réservé aux triggers (`security definer`).

Aucune table ne porte de contenu de fichier, de prompt, de diff ni de chemin absolu (§4, doctrine §7 du spec 2026-08-01).

## 7. L'espace projet (web)

Framework : ce dépôt utilise une version de Next.js avec des ruptures d'API — lire `node_modules/next/dist/docs/` avant d'écrire du code d'app (cf. `web/AGENTS.md`).

### 7.1 Emplacement et navigation

Route `/repo/[id]/projet`. Lien depuis `/repo/[id]` (en-tête, à côté de « ← tous les repos ») et une entrée depuis l'accueil sur chaque ligne de repo. La carte du plan (`/repo/[id]`) reste inchangée.

### 7.2 Écran

Trois colonnes — **À faire · En cours · Terminé**. Registre Linear : densité maîtrisée, neutres à peine teintés, la couleur ne dit que l'état (les trois teintes de l'artefact). Abonnement Realtime Supabase sur `tasks` (et `task_commits`) comme `direct.tsx` : une transition en base fait glisser la carte sans rechargement. Cible de bout en bout cohérente avec le spec précédent (< 5 s écriture → glissement sur un autre appareil).

### 7.3 Carte

- Titre + ancre (`app/landing/hero`, en mono discret).
- Pastille d'état : **texte + teinte, jamais la couleur seule** (doctrine accessibilité, PRODUCT). Coupée sous `prefers-reduced-motion` pour le mouvement de glissement.
- Colonne **Terminé** : le(s) commit(s) liés via `task_commits` — sha court + message ; badge `v2`, `v3`… si versionnée.
- Colonne **En cours** : indicateur léger si un agent est actif *maintenant* sous l'ancre (réutilise le signal temps réel d'activité déjà présent).

### 7.4 Créer / éditer une tâche

Bouton « Nouvelle tâche » → titre, description optionnelle, et **ancre** via un champ à autocomplétion sur les `modules` du repo (chemins connus) + saisie libre d'un fichier. À la validation :

- Si l'ancre exacte est déjà prise par une tâche non-`done` → refus lisible : « déjà suivie par la tâche “X” » (traduit la contrainte §3.1, vérifiée aussi côté serveur).
- Sinon insertion en `todo`.

Éditer le titre/description/ancre d'une carte ; supprimer une carte. Pas de sous-tâches, pas d'assignation, pas de dates (YAGNI solo).

## 8. Périmètre exclu (YAGNI)

- Pas d'import automatique d'un PRD en cartes (l'utilisateur crée les tâches).
- Pas de réconciliation d'historique réécrit (rebase/amend : nouveaux sha = nouveaux commits).
- Pas de backfill d'historique (HEAD-seulement, décision #7).
- Pas de dates d'échéance, d'assignés, de priorités, de sous-tâches, de tags.
- Pas de multi-utilisateur : un développeur solo, RLS par `user_id` comme le reste.

## 9. Plan de tests

- **Daemon** : premier passage curseur `null` → aucun commit posté, `last_commit_sha = HEAD`. Passage suivant avec N nouveaux commits → N lignes `commits`, chemins repliés corrects (fichiers + dossiers parents). Rejeu d'une passe → aucun doublon. Rebase → nouveaux sha traités comme neufs.
- **Triggers** : `todo→doing` sur write sous l'ancre ; pas de mouvement si aucune tâche ne matche. Commit → `done` + `task_commits(version)` ; `todo→done` direct. Nouvelle activité sur `done` → `doing`, `version+1`. Deux tâches sous un même dossier parent, ancres distinctes → un commit ne ferme que la plus profonde. Départage sur profondeur égale.
- **Contrainte** : deux tâches non-`done` sur la même ancre exacte → la seconde insertion échoue (index partiel) ; côté web, message lisible.
- **RLS** : un user ne voit ni n'édite les tâches/commits d'un repo qui n'est pas le sien ; le daemon ne peut insérer que pour une machine active non révoquée.
- **Web** : création avec autocomplétion ; drag manuel dans les deux sens ; glissement Realtime après un commit simulé ; rendu accessible (état non porté par la seule couleur), mouvement coupé sous `prefers-reduced-motion`.
