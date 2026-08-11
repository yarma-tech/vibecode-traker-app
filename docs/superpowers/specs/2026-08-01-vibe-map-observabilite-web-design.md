# Vibe Map, observabilité web : spec de conception

Date : 2026-08-01
État : en revue
Remplace : `docs/PRD-architecture-timeline.md` (ancienne direction macOS, abandonnée)

## 1. Ce qu'on construit

Vibe Map montre l'état vivant des codebases d'un développeur : quels agents travaillent, sur quels dossiers, depuis combien de temps, pour combien de jetons, et où deux agents sont en train de se marcher dessus.

L'écran principal est une carte : un rectangle par dossier, surface proportionnelle aux lignes de code, couleur selon ce que les agents y font.

La v1 **observe**. Elle ne lance rien et ne prédit rien.

Critère de réussite unique : après un coup d'œil de dix secondes, l'utilisateur sait s'il peut lancer un deuxième agent, et où. S'il doit ouvrir un terminal pour vérifier, l'outil a échoué.

Le contexte produit complet (utilisateur, personnalité, anti-références, principes) vit dans `PRODUCT.md` à la racine. La direction visuelle et les maquettes des huit écrans vivent dans l'artefact publié le 2026-08-01.

## 2. Décisions actées

| # | Décision | Motif |
|---|---|---|
| 1 | v1 = observabilité seule | Lancer un agent et prédire les conflits demandent une base de télémétrie qui n'existe pas encore. |
| 2 | Pont machine vers web = daemon local qui pousse vers Supabase | Aucun tunnel à garder ouvert, accessible partout, et la même voie resservira au lancement d'agents en v2. |
| 3 | Un module = un dossier | Agnostique au langage, colle aux chemins de la télémétrie, rien à configurer. |
| 4 | La couleur ne dit que l'état | Quatre états, quatre couleurs. Le worktree passe par la bordure et la trame, canal séparé. |
| 5 | Grille d'ensemble puis plongée | Tous les repos d'abord, le plan d'un repo ensuite. |
| 6 | Recherche : sous-chaîne, nom seul, insensible à la casse | Prévisible, zéro faux positif. Pas de correction de frappe. |
| 7 | L'arbre des dossiers vient du daemon, pas de GitHub | Les clones locaux sont la vérité. Conséquence : les deux comptes GitHub deviennent un badge lu dans le remote git, sans OAuth double. |
| 8 | Connexion GitHub pour le web, code d'appairage pour les machines | Un jeton par machine, révocable machine par machine. |
| 9 | Télémétrie : journaux de session en base, hook en accélérateur optionnel | Tout est déjà dans les `.jsonl`. Le hook ne fait que réduire la latence. |
| 10 | Conflit = deux sessions qui écrivent dans le même sous-arbre | C'est le grain où le merge fait mal. |

## 3. Architecture

Trois pièces. Rien ne joint la machine depuis l'extérieur : le daemon pousse, le web lit.

### 3.1 Daemon local

Un processus par machine, lancé au démarrage, sans interface.

Quatre boucles :

| Boucle | Cadence | Rôle |
|---|---|---|
| Scan des repos | au lancement, puis toutes les 5 min | arbre des dossiers, lignes par dossier, remote git, branche courante |
| Suivi des journaux | continu | sessions, fichiers touchés, jetons, durée |
| Worktrees | toutes les 30 s | `git worktree list --porcelain` par repo |
| Battement de cœur | toutes les 30 s | met à jour `machines.last_seen_at` |

Propriétés attendues :

- **Reprise sans doublon.** Position de lecture de chaque fichier de journal persistée sur disque. Au redémarrage, reprise à l'offset, pas au début.
- **File d'attente locale.** Si le réseau tombe, les événements s'empilent sur disque et repartent au retour. Renvoi avec temporisation croissante.
- **Écritures idempotentes.** Chaque événement porte son `tool_use_id`, contrainte d'unicité côté base. Rejouer un fichier ne crée pas de doublon.
- **Rien du contenu.** Aucun octet de fichier source ne sort. Voir §7.

### 3.2 Supabase

Postgres, RLS par `user_id` sur toutes les tables, Auth GitHub, Realtime sur les tables lues par la carte.

### 3.3 Web

Application responsive, consultable sur mobile. Lit Supabase, s'abonne au Realtime. En v1 elle n'écrit qu'une chose : le code d'appairage d'une nouvelle machine.

### 3.4 Chaîne complète

Un agent édite un fichier, une ligne s'ajoute au journal, le daemon la lit et l'agrège sur une fenêtre d'environ une seconde, il pousse vers Supabase, le Realtime notifie le web, le rectangle passe en ambre.

Cible de bout en bout : moins de 5 secondes entre l'écriture et le changement de couleur sur un autre appareil.

## 4. Données

### 4.1 Tables

```
machines
  id uuid pk, user_id uuid, label text, platform text,
  paired_at timestamptz, last_seen_at timestamptz, revoked_at timestamptz

repos
  id uuid pk, user_id uuid, machine_id uuid fk,
  name text, root_hash text, remote_owner text, remote_url text,
  account_label text, default_branch text, current_branch text,
  loc_total int, file_count int, scanned_at timestamptz
  unique (machine_id, root_hash)

modules
  id uuid pk, repo_id uuid fk,
  path text, parent_path text, depth int,
  loc int, file_count int, updated_at timestamptz
  unique (repo_id, path)

sessions
  id text pk, user_id uuid, repo_id uuid fk, machine_id uuid fk,
  agent_label text, branch text, worktree_path text,
  started_at timestamptz, last_event_at timestamptz, ended_at timestamptz,
  input_tokens bigint, output_tokens bigint,
  cache_read_tokens bigint, cache_creation_tokens bigint,
  cost_usd numeric, model text

activity_events
  id bigserial pk, user_id uuid, session_id text fk, repo_id uuid fk,
  module_path text, file_path text, kind text check (kind in ('read','write')),
  occurred_at timestamptz, tool_use_id text
  unique (session_id, tool_use_id)

worktrees
  id uuid pk, repo_id uuid fk, path text, branch text,
  detected_at timestamptz, closed_at timestamptz
```

### 4.2 État d'un module

Jamais stocké. Calculé à la lecture par une vue, sur une fenêtre glissante de 10 minutes :

1. aucun événement : **inactif**
2. uniquement des `read` : **lu**
3. au moins un `write` : **écrit**
4. au moins deux `session_id` distincts avec un `write` dans le sous-arbre : **conflit**

Le conflit se propage vers le haut : si deux agents écrivent dans `src/core/auth` et `src/core/db`, c'est `src/core` qui devient rouge à la profondeur affichée.

Le worktree est une jointure séparée, jamais mélangée à l'état ci-dessus. Un module peut être `écrit` et porter un worktree.

### 4.3 Profondeur affichée

Par défaut, le plan montre les dossiers de **profondeur 1** sous la racine du repo. Cliquer une parcelle descend d'un niveau.

Les dossiers dont le nom figure dans `.gitignore`, ainsi que `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, `vendor`, sont exclus du calcul des lignes et du plan.

*Point à confirmer : la profondeur 1 convient à un repo classique, moins à un monorepo où tout vit sous `packages/`. Solution retenue si le cas se pose : si un dossier de profondeur 1 contient plus de 70 % des lignes du repo, on affiche ses enfants à sa place.*

### 4.4 Rétention

`activity_events` purgés au-delà de 7 jours par une tâche planifiée. Les agrégats de `sessions` (jetons, coût, durée) sont conservés sans limite : c'est la mémoire longue « combien m'a coûté ce repo ».

## 5. Télémétrie

### 5.1 Source principale

Le daemon suit `~/.claude/projects/**/*.jsonl`. Vérifié sur les journaux réels : `file_path` par appel d'outil, bloc `usage` avec les jetons d'entrée, de sortie, de cache lu et de cache créé, plus `cwd` et `gitBranch`.

Correspondances :

| Donnée | Origine |
|---|---|
| session | `sessionId` |
| repo | `cwd`, remonté à la racine git puis apparié à un repo scanné |
| module | premier segment du chemin relatif à la racine du repo |
| nature | `Read`, `Grep`, `Glob` donnent `read` ; `Edit`, `Write`, `NotebookEdit` donnent `write` |
| jetons et coût | bloc `usage`, tarifé par la table de prix |
| durée | `last_event_at` moins `started_at` |

Les appels `Bash` ne produisent pas d'événement : la commande peut toucher n'importe quoi, deviner serait pire que se taire.

### 5.2 Accélérateur optionnel

Un hook `PostToolUse` peut poster le même événement directement. Il porte le même `tool_use_id`, la contrainte d'unicité absorbe le doublon. Le premier arrivé gagne, l'autre est ignoré.

Le hook n'est jamais requis. Une installation sans hook perd environ une seconde de latence, rien d'autre.

### 5.3 Coût

La logique de tarification et le lecteur de journaux existent dans l'application macOS abandonnée : `ModelPricing.swift` (46 lignes) et `JSONLParser.swift` (235 lignes), soit 281 lignes réellement réutilisables sur 78 fichiers Swift. Le reste est de l'interface macOS, jetée.

Les jetons de cache lu et de cache créé se tarifent séparément des jetons d'entrée. Ne pas les confondre fausse le coût d'un facteur trois sur une session longue.

## 6. Comptes et appairage

### 6.1 Web

Connexion GitHub via Supabase Auth. Un seul utilisateur par instance en v1. RLS `user_id = auth.uid()` sur toutes les tables.

### 6.2 Machines

1. Le web affiche un code court, valide 15 minutes.
2. L'utilisateur lance `vibemap pair 7K4-M2Q` sur la machine.
3. Le daemon échange le code contre un jeton machine et le range dans le trousseau du système.
4. Le web se remplit dès la première poussée.

Chaque machine a son jeton, révocable seule depuis les réglages. Perdre une machine ne compromet pas les autres.

### 6.3 Les deux comptes GitHub

Aucun OAuth double. Le daemon lit le remote git de chaque repo, en extrait le propriétaire, et le web l'affiche en badge `@perso` ou `@pro`. La correspondance propriétaire vers badge se règle une fois dans les réglages.

## 7. Confidentialité

> **Dépassée sur un point, à la date du 2026-08-11.** Le PRD de l'espace projet
> ([docs/PRD-espace-projet-kanban.md](../../PRD-espace-projet-kanban.md), §7.1)
> remplace la règle absolue ci-dessous par une **liste fermée et publiée** : deux
> lectures de contenu y sont admises — les titres de features d'un PRD, et le
> verdict d'une vérification par sous-agent. Le plancher, lui, ne bouge pas :
> ni code, ni diff, ni prompt, ni chemin absolu, ni secret. Cette section reste
> exacte pour le produit tel qu'il tourne aujourd'hui ; elle cessera de l'être
> le jour où l'espace projet sera livré, et le README devra suivre au même
> moment.

Ce qui sort de la machine : noms de repos, chemins **relatifs à la racine du repo**, nombres de lignes, noms de branches, horodatages, compteurs de jetons.

Ce qui ne sort jamais : le contenu des fichiers, les prompts, les réponses des agents, les chemins absolus. La racine d'un repo est identifiée par une empreinte, pas par son chemin, qui contient le nom de l'utilisateur.

Critère vérifiable : après une journée d'usage, inspecter les tables ne doit révéler aucune ligne de code source ni aucun nom d'utilisateur système.

## 8. Interface

Écrans spécifiés dans l'artefact de direction visuelle :

1. Tous les repos, tableau trié par activité, recherche par sous-chaîne
2. Un repo, plan vivant, bandeau de relevé, journal
3. Premier lancement, aucune machine appairée
4. Chargement, squelette aux proportions réelles
5. Repo cartographié sans activité
6. Machine injoignable, dernier état connu désaturé et horodaté
7. Recherche sans résultat, qui accuse le filtre actif et nomme le repo écarté

Règles de fond : clair par défaut, sombre traité, jamais la couleur seule, mouvement réservé au changement d'état réel, aucune animation d'ambiance.

## 9. Hors périmètre v1

Lancer un agent depuis la carte. Conflit prédictif avant lancement. Graphe de dépendances. Découpe de modules configurable par fichier. Recherche floue. Repos jamais clonés localement. Plusieurs utilisateurs. Couplage à First Mate.

## 10. Stack

Tranché le 2026-08-01, voir `docs/adr/0001-daemon-en-rust.md`.

**Daemon en Rust**, web en Next.js. Tokio pour les quatre boucles, notify pour la surveillance des fichiers, reqwest et serde pour Supabase, appel au binaire `git` en sortie `--porcelain`, keyring pour le jeton machine, launchd et systemd pour le démarrage automatique.

Contrepartie assumée : deux langages à tenir d'accord sur la forme des données. Les six mesures qui limitent ce risque sont listées dans l'ADR et ne sont pas facultatives. La principale : les migrations SQL sont l'unique source de vérité, les structures Rust et les types TypeScript en descendent tous les deux, et un test d'intégration vérifie le contrat en envoyant une charge réelle vers une base locale.

Conséquence sur l'ordre des travaux : le schéma se fige avant que le gros du code Rust soit écrit.

## 11. Critères d'acceptation

1. Un agent écrit un fichier : le module correspondant change de couleur sur un autre appareil en moins de 5 secondes.
2. Deux sessions écrivent dans le même sous-arbre : le module passe en conflit et une ligne apparaît dans le journal.
3. Le daemon est arrêté : l'écran passe en état gelé, horodaté, en moins de 90 secondes.
4. Le daemon redémarre après une coupure réseau de 10 minutes : aucun événement perdu, aucun doublon.
5. Rejouer entièrement un fichier de journal ne crée aucun doublon.
6. Chercher « ad » retourne `admin.conto`, `ad-server` et `payments-adapter`.
7. Le coût affiché pour une session correspond au calcul manuel à partir du bloc `usage`, cache compris.
8. Les tables ne contiennent ni contenu de fichier, ni chemin absolu.
9. Les sept écrans du §8 existent et sont atteignables.
10. Les deux thèmes passent le contraste AA, y compris les étiquettes en chasse fixe.

## 12. Risques

| Risque | Parade |
|---|---|
| Le format des journaux de Claude Code change | Lecture tolérante : un champ inconnu est ignoré, un champ manquant dégrade l'événement au lieu de le rejeter. Test sur des journaux réels versionnés. |
| Un très gros repo rend le scan coûteux | Scan incrémental sur la date de modification des dossiers, plafond de fichiers par passe. |
| La fenêtre de 10 minutes est mal calibrée | Valeur en configuration dès le départ, ajustée à l'usage. |
| Le plan de profondeur 1 est illisible sur un monorepo | Règle des 70 % décrite au §4.3. |
| Le daemon meurt en silence | Le battement de cœur est la seule source de vérité de l'état d'une machine, et l'écran gelé du §8 rend la panne visible. |
