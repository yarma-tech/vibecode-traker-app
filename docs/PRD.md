# PRD — Vibe Code Tracker App

> **Document d'archive.** Il décrit *Vibe Code Tracker*, l'application macOS
> retirée du dépôt le 2026-08-06. Le produit actuel est **Vibe Map** : voir le
> [README](../README.md) et [la spec](superpowers/specs/2026-08-01-vibe-map-observabilite-web-design.md).

> **Statut** : V1 — spec figée pour développement initial
> **Auteur** : Yannick Maillard
> **Plateforme** : macOS 14+ (Sonoma) — Swift / SwiftUI / SwiftData
> **Licence** : MIT
> **Distribution** : Open source sur GitHub, build direct (hors App Store en V1)

---

## 1. Vision

Vibe Code Tracker App est un tableau de bord local pour macOS qui donne aux utilisateurs de Claude Code une visibilité complète et automatique sur tous leurs projets de développement assisté par IA. L'app lit les données déjà présentes sur la machine (sessions Claude Code, repos Git, fichiers backlog) et croise ces données avec l'API Anthropic Console pour produire une vue consolidée de l'activité, des coûts et de l'avancement.

**Problème résolu** : les développeurs qui codent à plusieurs projets en parallèle via Claude Code n'ont aucune vue d'ensemble — sessions éparpillées, tokens consommés invisibles, statut des projets flou, décisions architecturales perdues.

**Promesse** : ouvrir l'app et savoir en 5 secondes où en est chaque projet, ce qui a été fait cette semaine, combien ça a coûté, et ce qui est bloqué.

---

## 2. Personas

### Persona principal — Le solo dev multi-projets
Développeur indépendant ou en reconversion, jongle entre 5–15 projets actifs ou expérimentaux. Utilise Claude Code intensivement. Veut comprendre sa consommation, ses patterns de productivité, et garder une trace pour son portfolio.

### Persona secondaire — Le PM technique
Product Manager qui code via Claude Code pour prototyper. Besoin de tracer les décisions, montrer l'avancement, justifier les coûts à son management.

### Persona tertiaire — Le freelance facturable
Vend des prestations Claude Code à des clients, doit justifier les heures et tokens consommés par projet.

---

## 3. Objectifs V1

### Objectifs fonctionnels
- **F1** : Détecter automatiquement tous les projets ayant eu au moins une session Claude Code sur la machine.
- **F2** : Afficher un dashboard global (Niveau 1) avec KPIs agrégés cross-projets.
- **F3** : Afficher une vue détaillée par projet (Niveau 2) avec timeline et backlog.
- **F4** : Synchroniser les données entre plusieurs Macs du même utilisateur via CloudKit.
- **F5** : Récupérer les tokens et coûts précis via l'API Anthropic Console.

### Objectifs non-fonctionnels
- **NF1** : Lancement de l'app en moins de 2 secondes avec 50+ projets en base.
- **NF2** : Sync incrémental — n'analyse que les nouvelles lignes JSONL depuis le dernier scan.
- **NF3** : Aucune donnée envoyée vers un serveur tiers autre qu'Anthropic (API officielle) et iCloud (CloudKit Apple).
- **NF4** : Clé API stockée exclusivement dans le Keychain macOS.
- **NF5** : Code open source, contributions externes encouragées.

---

## 4. Hors scope V1

Pour cadrer la V1 et permettre une livraison rapide :

- ❌ Captures d'écran de projets (reporté V2)
- ❌ Notifications macOS push
- ❌ Connexion GitHub Issues / Linear / Jira
- ❌ Mode équipe / multi-utilisateurs sur même Mac
- ❌ Export PDF des rapports
- ❌ Catégorisation automatique des sessions via LLM (V1 = manuelle ou inférée par mots-clés simples)
- ❌ Distribution App Store (V1 = build local + DMG signé optionnel)
- ❌ Application iOS / iPad
- ❌ Inférence automatique des bugs corrigés vs nouvelles features par LLM (V1 = heuristique sur commits)

---

## 5. Architecture technique

### Stack
- **Langage** : Swift 5.9+
- **UI** : SwiftUI (target macOS 14+ pour bénéficier de SwiftData et des dernières API Charts)
- **Persistence** : SwiftData (CloudKit-backed pour sync multi-machine)
- **Graphiques** : Swift Charts (framework natif Apple)
- **File watching** : `DispatchSource.makeFileSystemObjectSource` + scan périodique fallback
- **Git** : shell-out à `/usr/bin/git` via `Process` (libgit2 jugé trop lourd pour V1)
- **Networking** : `URLSession` + `async/await` pour l'API Anthropic
- **Tests** : XCTest pour la logique, Swift Testing pour les nouveaux tests
- **Build** : Xcode 15+, pas de Tuist/XcodeGen en V1 (simplicité contribution OSS)

### Diagramme d'architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Sources de données                         │
├─────────────────────────────────────────────────────────────────┤
│  ~/.claude/projects/<hash>/<session>.jsonl   Sessions          │
│  <project>/.git/                              Commits           │
│  <project>/TODO.md, BACKLOG.md                Backlog           │
│  <project>/package.json, Cargo.toml, …        Stack detection   │
│  Anthropic Console API                        Tokens + coûts    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Couche d'ingestion (Services)                  │
├─────────────────────────────────────────────────────────────────┤
│  ClaudeProjectsScanner    Watch + parse JSONL incrémental       │
│  GitInspector             Lecture commits via git CLI           │
│  BacklogParser            Parse TODO.md / BACKLOG.md            │
│  StackDetector            Identification stack par fichiers     │
│  AnthropicUsageClient     Appels API Console (tokens/coûts)     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   SwiftData store (CloudKit)                    │
│  Project · Session · Commit · BacklogItem · TokenUsageSnapshot  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Couche UI (SwiftUI views)                    │
│  GlobalDashboardView (Niveau 1) · ProjectDetailView (Niveau 2)  │
│  SettingsView · OnboardingView · SessionDetailSheet             │
└─────────────────────────────────────────────────────────────────┘
```

### Organisation du code

```
VibeCodeTrackerApp/
├── VibeCodeTrackerApp.swift           # entry point
├── App/
│   ├── AppState.swift
│   └── Container.swift              # DI léger
├── Models/                          # @Model SwiftData
│   ├── Project.swift
│   ├── Session.swift
│   ├── Commit.swift
│   ├── BacklogItem.swift
│   └── TokenUsageSnapshot.swift
├── Services/
│   ├── ClaudeProjectsScanner.swift
│   ├── JSONLParser.swift
│   ├── GitInspector.swift
│   ├── BacklogParser.swift
│   ├── StackDetector.swift
│   ├── AnthropicUsageClient.swift
│   └── KeychainStore.swift
├── ViewModels/
│   ├── GlobalDashboardViewModel.swift
│   └── ProjectDetailViewModel.swift
├── Views/
│   ├── Global/
│   │   ├── GlobalDashboardView.swift
│   │   ├── KPICard.swift
│   │   ├── ActivityChartView.swift
│   │   └── LatestSessionsList.swift
│   ├── Project/
│   │   ├── ProjectDetailView.swift
│   │   ├── CommitHeatmapView.swift
│   │   ├── SessionsListView.swift
│   │   └── BacklogView.swift
│   ├── Settings/
│   │   └── SettingsView.swift
│   └── Components/
│       └── StatusBadge.swift
├── Resources/
│   └── Assets.xcassets
└── Tests/
    ├── JSONLParserTests.swift
    ├── BacklogParserTests.swift
    ├── StackDetectorTests.swift
    └── Fixtures/
```

---

## 6. Modèle de données

### Entités SwiftData

```swift
@Model final class Project {
    @Attribute(.unique) var id: UUID
    var name: String
    var path: String
    var claudeProjectHash: String      // dossier dans ~/.claude/projects/
    var stack: [String]                 // ["TypeScript", "Mastra", "Supabase"]
    var firstSeenAt: Date
    var lastActivityAt: Date
    var isArchived: Bool
    var isPinned: Bool
    var customColor: String?           // hex pour distinction visuelle

    @Relationship(deleteRule: .cascade, inverse: \Session.project)
    var sessions: [Session] = []

    @Relationship(deleteRule: .cascade, inverse: \Commit.project)
    var commits: [Commit] = []

    @Relationship(deleteRule: .cascade, inverse: \BacklogItem.project)
    var backlogItems: [BacklogItem] = []
}

@Model final class Session {
    @Attribute(.unique) var sessionId: String
    var startedAt: Date
    var endedAt: Date?
    var durationSeconds: Int
    var model: String                  // "claude-sonnet-4-6-20250514"
    var modelFamily: String            // "Sonnet" | "Opus" | "Haiku"
    var inputTokens: Int
    var outputTokens: Int
    var cacheReadTokens: Int
    var cacheCreationTokens: Int
    var totalCostUSD: Double?
    var topic: String?                 // édité manuellement ou inféré
    var category: String?              // "feature" | "bugfix" | "refactor" | "exploration" | "docs" | "ops"
    var status: String                 // "completed" | "inProgress" | "blocked"
    var filesModified: [String]
    var commitHashes: [String]
    var messageCount: Int
    var firstUserPrompt: String?       // début du premier prompt user (preview)

    var project: Project?
}

@Model final class Commit {
    @Attribute(.unique) var sha: String
    var message: String
    var authorName: String
    var authoredAt: Date
    var filesChanged: Int
    var insertions: Int
    var deletions: Int
    var branch: String?
    var inferredType: String?          // "feat" | "fix" | "refactor" | "docs" | "chore"

    var project: Project?
    @Relationship var session: Session?
}

@Model final class BacklogItem {
    @Attribute(.unique) var id: UUID
    var title: String
    var isDone: Bool
    var sourceFile: String             // "TODO.md" | "BACKLOG.md"
    var lineNumber: Int
    var lastSeenAt: Date
    var priority: String?              // "P0" | "P1" | "P2" (parsé si présent)

    var project: Project?
}

@Model final class TokenUsageSnapshot {
    @Attribute(.unique) var id: UUID
    var fetchedAt: Date
    var periodStart: Date
    var periodEnd: Date
    var model: String
    var inputTokens: Int
    var outputTokens: Int
    var cacheReadTokens: Int
    var cacheCreationTokens: Int
    var costUSD: Double
    var source: String                 // "local-jsonl" | "anthropic-api"
}
```

---

## 7. Spec fonctionnelle — Niveau 1 (Vue globale)

### Layout

Fenêtre principale, sidebar gauche + zone centrale large.

**Sidebar gauche** (240px) :
- Bouton "Global Dashboard" (sélectionné par défaut)
- Section "Projects" avec liste triée par activité récente
- Bouton "Settings" en bas

**Zone centrale** :

```
┌──────────────────────────────────────────────────────────────┐
│  Global Dashboard                            [Refresh] [⚙]   │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐             │
│  │PROJECTS │ │SESSIONS │ │ TOKENS  │ │  COST   │             │
│  │   12    │ │  147    │ │  2.4M   │ │ $87.20  │             │
│  │ active  │ │this week│ │this week│ │this week│             │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐             │
│  │AVG/SESS │ │AVG TIME │ │TOP MODEL│ │ BLOCKED │             │
│  │ 16.3k   │ │ 42 min  │ │Sonnet 4 │ │   3     │             │
│  │tokens   │ │         │ │  68 %   │ │sessions │             │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘             │
│                                                              │
│  Activity — last 30 days                                     │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  bar chart : sessions par jour, coloré par modèle    │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  Model split                                                 │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  donut chart : tokens par famille de modèle          │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  Latest sessions across all projects                         │
│  [filter: All ▼] [period: 7 days ▼]                          │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ 🟢 karata-multi-agent • feature • Sonnet • 24k tok   │    │
│  │    "Agent 3 routing logic"             2h ago        │    │
│  │ 🔴 omicron-content   • bugfix  • Opus   • 18k tok    │    │
│  │    "Remotion render error"             5h ago        │    │
│  │ 🟡 yannick-tech      • refactor • Sonnet • 12k tok   │    │
│  │    "Glass editorial nav"               8h ago        │    │
│  │ ...                                                  │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

### KPIs détaillés

| KPI | Calcul | Source |
|---|---|---|
| Projects active | Projets avec ≥1 session dans les 30 derniers jours | SwiftData |
| Sessions this week | COUNT(Session WHERE startedAt > weekStart) | SwiftData |
| Tokens this week | SUM(input + output + cache) over week | SwiftData (puis recalé sur API si dispo) |
| Cost this week | SUM(totalCostUSD) over week | Anthropic API |
| Avg tokens/session | AVG over rolling 30 days | SwiftData |
| Avg session duration | AVG(durationSeconds) over 30 days | SwiftData |
| Top model | Modèle avec plus de tokens cumulés (30j) | SwiftData |
| Blocked sessions | COUNT(Session WHERE status = "blocked") | SwiftData |

### Statut des sessions (heuristique V1)

- `inProgress` : fichier JSONL modifié il y a < 30 min
- `blocked` : la session contient ≥ 3 messages assistant consécutifs avec mots-clés erreur (`Error`, `failed`, `cannot`, `unable to`) ET aucun commit Git associé
- `completed` : tout le reste

### Interactions

- Clic sur une session dans la liste → ouvre `SessionDetailSheet` (modal)
- Clic sur un projet dans la sidebar → bascule sur `ProjectDetailView`
- Clic sur un KPI → applique le filtre correspondant à la liste

---

## 8. Spec fonctionnelle — Niveau 2 (Détail projet)

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│  ← Back     karata-multi-agent             [Open in Finder]  │
├──────────────────────────────────────────────────────────────┤
│  Path: ~/projects/karata-multi-agent                         │
│  Stack: TypeScript · Mastra · Supabase · Twilio · Inngest    │
│  First seen: Jan 12, 2026 • Last activity: 2h ago            │
│  [Edit metadata]                                             │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐             │
│  │SESSIONS │ │ TOKENS  │ │  COST   │ │AVG TIME │             │
│  │   34    │ │  580k   │ │ $21.40  │ │ 51 min  │             │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘             │
│                                                              │
│  Commit timeline — last 90 days                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  heatmap GitHub-style, intensité = nb commits/jour   │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  Recent commits                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ feat • Agent 3 routing logic            2h ago       │    │
│  │ fix  • Supabase schema migration        5h ago       │    │
│  │ ...                                                  │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌──────────────────────┐ ┌─────────────────────────────┐    │
│  │  Recent sessions     │ │  Backlog                    │    │
│  │                      │ │  source: TODO.md            │    │
│  │ 🟢 Agent 3 routing   │ │ ☐ Agent 4 (escalate)        │    │
│  │   completed • 2h ago │ │ ☐ Twilio webhook retry      │    │
│  │ 🟡 Schema fix        │ │ ☐ Test E2E carnaval         │    │
│  │   in progress 5h ago │ │ ☑ Agent 0 router            │    │
│  │ 🔴 Qonto OAuth       │ │ ☑ Schema pigistes           │    │
│  │   blocked yesterday  │ │                             │    │
│  └──────────────────────┘ └─────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

### Détection automatique de la stack

Le `StackDetector` lit la racine du projet et infère la stack via heuristiques :

| Fichier trouvé | Tags ajoutés |
|---|---|
| `package.json` | "JavaScript" + lecture des deps pour identifier Next.js, React, Mastra, etc. |
| `tsconfig.json` | "TypeScript" |
| `Cargo.toml` | "Rust" |
| `pyproject.toml` ou `requirements.txt` | "Python" |
| `Podfile` ou `*.xcodeproj` | "Swift" + "iOS"/"macOS" selon target |
| `go.mod` | "Go" |
| `supabase/config.toml` | "Supabase" |
| `prisma/schema.prisma` | "Prisma" |
| `.env` avec `TWILIO_*` | "Twilio" |
| `Dockerfile` | "Docker" |

L'utilisateur peut éditer manuellement la stack via "Edit metadata".

### Timeline de commits

Composant `CommitHeatmapView` — réplique du graphe de contribution GitHub :
- Grille 7 lignes (jours de la semaine) × ~13 colonnes (semaines)
- Couleur graduée selon nombre de commits dans la journée
- Tooltip au survol : date + nombre de commits + messages courts
- Implémenté avec Swift Charts (`Chart` + `RectangleMark`)

---

## 9. Sources de données — détails techniques

### 9.1 Parser JSONL Claude Code

**Localisation** : `~/.claude/projects/<encoded-cwd>/<session-uuid>.jsonl`

L'encodage du cwd dans le nom du dossier est : remplacement des `/` par `-`. Exemple : `/Users/yannick/projects/karata-multi-agent` → `-Users-yannick-projects-karata-multi-agent`.

**Schéma observé d'une ligne** :
```json
{
  "type": "user" | "assistant" | "tool_use" | "tool_result" | "system",
  "timestamp": "2026-06-06T14:32:11.000Z",
  "sessionId": "abc-123-def",
  "cwd": "/Users/yannick/projects/karata-multi-agent",
  "message": {
    "id": "msg_...",
    "role": "assistant",
    "model": "claude-sonnet-4-6-20250514",
    "content": [...],
    "usage": {
      "input_tokens": 1234,
      "output_tokens": 567,
      "cache_read_input_tokens": 8900,
      "cache_creation_input_tokens": 200
    }
  }
}
```

**Stratégie de parsing** :
1. Au lancement : full scan de `~/.claude/projects/`
2. Pour chaque fichier `.jsonl` : tracker offset dans une table `FileSync(path, lastOffset, lastModified)` (en SwiftData)
3. Watch avec `DispatchSource.makeFileSystemObjectSource` sur le dossier parent
4. À chaque modification détectée : lire à partir de `lastOffset`, parser les nouvelles lignes JSON
5. Agréger par `sessionId` : `durationSeconds = lastTimestamp - firstTimestamp`, tokens = sum des `usage`
6. Le `cwd` du JSONL → matching avec `Project.path` (création si projet inconnu)

**Gestion d'erreurs** :
- Ligne JSON malformée : log + skip, ne pas crasher
- Fichier verrouillé : retry après 1s, max 3 fois
- JSONL > 100MB : parsing en stream, pas en mémoire

### 9.2 Git Inspector

Shell-out à `git` via `Process` (un binaire universel, dispo partout sur Mac via Xcode CLT).

**Commandes utilisées** :
```bash
git -C <project-path> log --since="90 days ago" --pretty=format:"%H|%an|%ai|%s" --shortstat
git -C <project-path> branch --show-current
```

**Matching session ↔ commit** :
- Si un commit a été fait dans la fenêtre [session.startedAt, session.endedAt + 5 min], rattacher à la session.
- Heuristique simple en V1, raffinable en V2.

**Inférence du type de commit** :
- Préfixe conventional commits (`feat:`, `fix:`, `refactor:`, etc.) → utilisé directement
- Sinon mots-clés dans le message (`add`, `fix`, `update`, `remove`) → mapping
- Sinon → `chore`

### 9.3 Backlog Parser

Cherche `TODO.md` et `BACKLOG.md` à la racine du projet.

**Format markdown supporté** :
```markdown
- [ ] Tâche non faite
- [x] Tâche faite
- [ ] **P0** — Priorité haute (extrait via regex)
- [ ] [P1] Autre format de priorité
```

**Algorithme** :
1. Regex `^[\s-*]*\[([ x])\]\s+(.+)$` pour extraire items
2. Extraction priorité via regex `\b(P[0-2])\b` ou `\*\*(P[0-2])\*\*`
3. Diff avec items déjà en base : ajout/suppression/changement de statut
4. Re-parse à chaque session-end et à chaque ouverture de l'app

### 9.4 Anthropic Usage API

**Endpoint** : `https://api.anthropic.com/v1/organizations/usage_report/messages`

**Configuration requise** :
- Clé API Admin (différente de la clé API standard) — stockée Keychain
- Headers : `x-api-key`, `anthropic-version: 2023-06-01`

**Fréquence** :
- Au lancement de l'app
- Toutes les heures en background (timer)
- Manuel via bouton Refresh

**Mapping vers SwiftData** :
- Crée un `TokenUsageSnapshot` par appel
- Réconcilie avec les sessions locales par date + modèle
- Le coût USD est l'autorité — les tokens locaux servent uniquement de fallback si l'API échoue

**Fallback si pas de clé API** :
- L'app fonctionne en mode "tokens-only" sans coûts
- Bannière non-intrusive dans Settings : "Add your Anthropic Admin API key to see costs"

---

## 10. Multi-machine — Sync CloudKit

### Stratégie

SwiftData supporte CloudKit nativement via la configuration `ModelConfiguration(cloudKitDatabase: .private("iCloud.tech.yannick.vibecodetracker"))`.

**Ce qui sync** :
- Tous les modèles `Project`, `Session`, `Commit`, `BacklogItem`, `TokenUsageSnapshot`
- Préférences utilisateur (clé API API _exclue_ — reste Keychain local)

**Ce qui ne sync PAS** :
- Les fichiers JSONL bruts (trop volumineux, et propres à chaque machine)
- Les credentials
- L'index `FileSync` (state local au filesystem de chaque machine)

**Conflits** :
- Stratégie last-write-wins par défaut SwiftData
- Pour les `Session`, identifier unique = `sessionId` Claude Code donc pas de doublon
- Pour les `Project`, identifier = `path` normalisé OU `claudeProjectHash` (préférable car même path sur 2 Macs ≠ même projet forcément)

**Activation côté utilisateur** :
- Onboarding : checkbox "Sync via iCloud" (par défaut activée si iCloud connecté)
- Settings : toggle pour activer/désactiver
- Status indicator dans la sidebar : ☁️ "Synced 2 min ago"

### Prérequis Apple Developer

- Apple Developer account ($99/an) — bloqueur Yannick
- App ID enregistré avec capability CloudKit
- Container CloudKit créé : `iCloud.tech.yannick.vibecodetracker`
- Provisioning profile

**Si l'utilisateur n'a pas Apple Developer** : l'app fonctionne sans sync, en local pur. Le code détecte l'absence de capability et désactive gracieusement la fonctionnalité.

---

## 11. Open source — structure du repo

```
vibecode-traker-app/
├── README.md                          # description, screenshots, install
├── LICENSE                            # MIT
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── CHANGELOG.md
├── .gitignore
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                     # build + tests sur PR
│   │   └── release.yml                # build DMG signé sur tag
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
├── VibeCodeTrackerApp.xcodeproj
├── VibeCodeTrackerApp/                  # code source app
├── VibeCodeTrackerAppTests/             # tests
├── docs/
│   ├── PRD.md                         # ce document
│   ├── ARCHITECTURE.md
│   ├── DEVELOPMENT.md
│   └── screenshots/
└── scripts/
    ├── build-release.sh
    └── dev-bootstrap.sh
```

### README structure cible
1. Logo + tagline
2. Screenshots (2-3 captures)
3. Features
4. Installation (Homebrew cask futur, DMG release maintenant)
5. Setup (Anthropic API key optionnelle)
6. Privacy (tout reste local sauf API Anthropic + iCloud opt-in)
7. Contributing
8. License

### Licence
**MIT** — permissive, standard pour outils dev macOS open source.

### Considération trademark
Le nom "Vibe Code Tracker" évite l'usage direct de la marque "Claude" (qui appartient à Anthropic). L'app peut mentionner "compatible with Claude Code" dans la description sans problème selon les [Anthropic Trademark Guidelines](https://www.anthropic.com/trademark). Pas de blocage prévu.

---

## 12. Phases de développement — slices verticaux

Chaque slice est end-to-end : data + logique + UI + tests + commit. À la fin de chaque slice, l'app reste fonctionnelle.

| # | Slice | Livre une app qui... | Estimation |
|---|---|---|---|
| 1 | Bootstrap | Se lance, affiche un splash, vide mais sans crash | 1h |
| 2 | Détection projets | Liste tous les projets détectés depuis ~/.claude/projects/ | 3h |
| 3 | Sessions parser | Pour chaque projet, liste les sessions avec model + tokens | 4h |
| 4 | Dashboard global | Affiche les 8 KPIs principaux | 3h |
| 5 | Détail projet | Vue projet avec timeline et sessions récentes | 4h |
| 6 | Git integration | Affiche commits et heatmap | 3h |
| 7 | Backlog parser | Liste les items TODO.md / BACKLOG.md | 2h |
| 8 | Stack detection | Détecte et affiche la stack par projet | 2h |
| 9 | Status sessions | Marque inProgress / blocked / completed | 2h |
| 10 | Settings + Keychain | Stocke clé API, gère préférences | 2h |
| 11 | Anthropic API | Sync tokens et coûts via API Console | 4h |
| 12 | CloudKit sync | Multi-machine sync (bloqué sans Apple Dev) | 3h |
| 13 | Polish UI | Animations, états vides, dark mode | 3h |
| 14 | Docs + README | Documentation OSS-ready | 2h |

**Approche nuit** : Claude Code attaque slice 1 → 14 dans l'ordre. Si bloqué sur un slice, le note dans `BLOCKERS.md` et continue avec le suivant.

---

## 13. Critères de succès V1

L'app V1 est considérée réussie si, sur la machine de Yannick :

- ✅ Au lancement, tous ses projets ayant utilisé Claude Code apparaissent
- ✅ Le dashboard global affiche des chiffres cohérents (tokens, sessions, coût)
- ✅ La vue projet montre l'historique correct des sessions et commits
- ✅ Les backlogs sont lus depuis ses TODO.md
- ✅ Aucune donnée n'est envoyée à un service tiers non-déclaré
- ✅ L'app compile sans warnings et passe les tests
- ✅ Le repo GitHub est public, propre, avec README et LICENSE
- ✅ Un utilisateur tiers peut cloner et compiler en moins de 5 minutes

---

## 14. Annexes

### A. Glossaire

- **JSONL** : JSON Lines, format où chaque ligne est un objet JSON
- **CloudKit** : service Apple de sync entre devices d'un même utilisateur iCloud
- **SwiftData** : framework de persistance Apple, successeur moderne de Core Data
- **Keychain** : coffre-fort sécurisé macOS pour secrets
- **FSEvents** : API macOS de file system events

### B. Décisions architecturales

- **Pourquoi SwiftData et pas Core Data ?** Plus moderne, syntaxe `@Model`, sync CloudKit intégrée, target macOS 14+ est acceptable pour la cible utilisateur (devs sur Mac récent).
- **Pourquoi shell-out git plutôt que libgit2 ?** Pas de dépendance C à compiler, git est installé sur toute machine de dev, performances suffisantes pour le scan.
- **Pourquoi pas d'API call automatique pour catégoriser les sessions ?** Coût + complexité de configuration. V1 = heuristique simple. V2 = option opt-in.
- **Pourquoi MIT et pas Apache 2.0 ?** Plus simple, plus standard pour outils dev macOS. Apache 2.0 si Yannick veut une protection brevet explicite.

### C. Risques identifiés

| Risque | Mitigation |
|---|---|
| Schéma JSONL Claude Code change | Parser tolérant, schéma versionné, tests fixtures |
| API Anthropic Console change | Versionner les calls, fallback gracieux |
| Utilisateur n'a pas Apple Dev account | App fonctionne sans CloudKit |
| Trademark "Claude" contesté | Plan de renommage prêt |
| Volume JSONL > 1GB | Parsing en stream, pagination dans la BDD |
