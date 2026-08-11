# Changelog

Les changements notables du dépôt, au format
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versionnés selon
[SemVer](https://semver.org/spec/v2.0.0.html).

Le dépôt a porté deux produits. **Vibe Map** est l'actuel, et c'est lui que
numérotent les versions ci-dessous. *Vibe Code Tracker*, l'application macOS
retirée le 2026-08-06, garde son journal en bas de page, sous son propre titre :
son `0.1.0` n'a rien à voir avec celui de Vibe Map.

## [Non publié]

Rien pour l'instant.

## [0.1.0] — 2026-08-10

Première release de Vibe Map. Le daemon `vibemap` est distribué en binaire
compilé pour macOS arm64 et Linux x86_64, chaque archive verrouillée par une
somme de contrôle sha256.

### Ajouté

- **Daemon.** Binaire Rust qui cartographie les repos, lit les journaux de
  Claude Code et pousse des métadonnées : battement de cœur toutes les 30 s,
  cartographie toutes les 5 min, lecture des journaux toutes les 2 s. Le choix
  du langage est motivé dans [l'ADR 0001](docs/adr/0001-daemon-en-rust.md). (#1, #2)
- **Appairage.** Une machine se relie à un compte par un code demandé depuis le
  web ; le jeton va dans le trousseau du système, la configuration dans
  `~/.config/vibemap/config.toml`. (#9)
- **Le plan.** Un dossier par parcelle, la surface proportionnelle aux lignes,
  affiché en gris au repos. (#3)
- **L'activité en direct.** Bleu quand un agent lit, ambre quand il écrit. (#4)
- **Le conflit.** Deux sessions qui écrivent dans le même sous-arbre passent le
  module en rouge et inscrivent une ligne au journal. (#5)
- **Worktrees.** Affichés en surimpression du repo parent. (#6)
- **Le relevé.** Jetons, coût et temps par repo. (#7)
- **Écran d'accueil.** Tous les repos d'un coup. (#8)
- **États vides.** Premier lancement, chargement, repo sans activité. (#12)
- **Distribution.** Workflow de release déclenché par un tag `v*` (compilation
  croisée, sommes de contrôle, publication), formule Homebrew et plist launchd
  pour un démarrage à l'ouverture de session. (#14)
- **Hook `PostToolUse` facultatif**, qui fait gagner environ une seconde. Sans
  lui, tout fonctionne. Voir `daemon/hooks/README.md`.

### Robustesse

- Le daemon survit aux coupures réseau et aux redémarrages : aucun événement
  perdu, aucun doublon. Rejouer entièrement un fichier de journal n'en crée pas
  non plus. (#10)
- Une machine injoignable fait passer l'écran en état gelé, horodaté, en moins
  de 90 secondes. Le battement de cœur est la seule source de vérité de l'état
  d'une machine. (#11)

### Confidentialité

- Le code ne quitte pas la machine : seuls des noms de dossiers relatifs, des
  compteurs et des horodatages sont transmis. Un chemin absolu devient une
  empreinte, et aucune table n'a de colonne où un contenu de fichier ou un
  prompt pourrait entrer.
- Toute la logique d'accès est en RLS ; révoquer une machine coupe ses écritures
  à la milliseconde.

### Interface

- Revue visuelle des écrans, les deux thèmes passant le contraste AA, étiquettes
  en chasse fixe comprises. (#13)

### Périmètre

La v1 ne fait qu'observer : elle ne lance rien, n'arrête rien, ne corrige rien.

### Connu

- La formule Homebrew vit à deux endroits — la source dans ce dépôt, la copie
  servie dans le tap `yarma-tech/homebrew-vibemap`. Chaque release demande de
  recopier le fichier à la main.
- Les tests d'intégration du daemon parlent à une vraie pile Supabase locale et
  ne tournent donc pas en intégration continue ; ils se lancent à la main avant
  chaque fusion.

[Non publié]: https://github.com/yarma-tech/vibecode-traker-app/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yarma-tech/vibecode-traker-app/releases/tag/v0.1.0

---

# Vibe Code Tracker — application macOS (retirée)

> Journal d'archive. Cette application SwiftData lisait les mêmes journaux, hors
> ligne. Elle a été retirée du dépôt le 2026-08-06 au profit de Vibe Map ; son
> code reste dans l'historique git, sous le commit `f435888`. Rien de ce qui
> suit ne décrit le produit actuel, et sa numérotation lui est propre.

### Non publié au moment du retrait

#### Changed
- Costs are always shown as an on-device **estimate** (token counts × published
  Anthropic list prices). No API key or account is required.

#### Removed
- The Anthropic Admin API key / usage-sync feature (`AnthropicUsageClient`,
  `UsageSyncService`, `KeychainStore`, `TokenUsageSnapshot`, `Session.totalCostUSD`,
  and the Settings "Anthropic API" section). It fetched data that was never
  displayed and only duplicated the estimate; the app now makes no network calls.
  See `DECISIONS.md` (2026-06-08).

### 0.1.0 — 2026-06-07

Initial release.

#### Added (post-initial)
- Project filters: Claude Code worktrees are detected and hidden from the sidebar
  and KPIs by default (with a "Show worktrees" toggle), and projects are categorized
  by GitHub remote vs local-only via a sidebar filter menu (All / On GitHub / Local only).

#### Fixed
- Crash when opening a project that has Git commits: the commit heatmap used a
  descending `chartYScale` domain (`6.5...(-0.5)`), which traps at runtime. Now uses
  an ascending domain with the weekday row inverted, plus a render regression test.

#### Added
- Automatic detection of Claude Code projects from `~/.claude/projects/`.
- Schema-tolerant JSONL session parser (tokens, primary model, duration, message
  count, first prompt) with file-level incremental scanning.
- Global dashboard with eight KPIs and a cross-project latest-sessions list.
- Per-project detail view: local KPIs, commit contribution heatmap (Swift Charts),
  recent commits, sessions, backlog, and detected stack.
- Git integration via the `git` CLI with commit-type inference.
- Backlog parser for `TODO.md` / `BACKLOG.md` with P0–P2 priorities.
- Automatic tech-stack detection from marker files and `package.json` dependencies.
- Session status heuristic (in progress / blocked / completed).
- Settings with Keychain-stored Anthropic Admin API key, refresh frequency, and an
  iCloud sync toggle.
- Anthropic usage API client (mock-tested) with token snapshots and estimated costs.
- App icon, dark-mode support, loading/empty states, and an animated refresh.

#### Known limitations
- CloudKit sync is scaffolded but inactive (requires an Apple Developer setup and
  removal of unique constraints). See `BLOCKERS.md`.
- Cost figures are estimated from public pricing; the exact usage API schema needs
  validation with a real key. See `BLOCKERS.md`.
