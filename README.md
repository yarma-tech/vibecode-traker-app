# Vibe Map

Une carte de tes codebases qui s'allume quand un agent y travaille.

> Dix secondes sur un onglet, entre deux autres, pour décider si tu lances un
> deuxième agent. Puis tu refermes.

Un dossier par parcelle, la surface proportionnelle aux lignes. Bleu quand on
lit, ambre quand on écrit, rouge quand deux agents se marchent dessus. La v1 ne
fait qu'observer : elle ne lance rien, n'arrête rien, ne corrige rien.

**Ton code ne quitte pas ta machine.** Seuls des noms de dossiers relatifs, des
compteurs et des horodatages voyagent. Le chemin absolu d'un repo devient une
empreinte ; aucune table n'a de colonne où un contenu de fichier ou un prompt
pourrait entrer.

## Les trois morceaux

| | |
|---|---|
| `daemon/` | Un binaire Rust sur ton poste. Il cartographie tes repos, lit les journaux de Claude Code, et pousse des métadonnées. Voir [l'ADR 0001](docs/adr/0001-daemon-en-rust.md). |
| `supabase/` | Le schéma : machines, repos, modules, sessions, activité. Toute la logique d'accès est en RLS ; révoquer une machine coupe ses écritures à la milliseconde. |
| `web/` | Next.js. La carte, le journal, le relevé. |

La conception complète est dans
[la spec](docs/superpowers/specs/2026-08-01-vibe-map-observabilite-web-design.md),
la personnalité du produit dans [PRODUCT.md](PRODUCT.md).

## Démarrer

Il faut Rust, Node 22, et la [CLI Supabase](https://supabase.com/docs/guides/local-development)
avec un moteur Docker (Colima fait l'affaire).

```sh
supabase start                 # la pile locale
cd web && npm install && npm run dev
```

Puis, dans un autre terminal :

```sh
cd daemon && cargo build
```

Ouvre <http://localhost:3000>, connecte-toi, demande un code d'appairage, et
relie la machine :

```sh
VIBEMAP_SUPABASE_URL=http://127.0.0.1:54321 \
VIBEMAP_SUPABASE_ANON_KEY=<cle anon de `supabase status`> \
  ./daemon/target/debug/vibemap pair ABC-DEFG
```

Le jeton va dans le trousseau du système, la configuration dans
`~/.config/vibemap/config.toml`. Ensuite :

```sh
./daemon/target/debug/vibemap
```

Il bat toutes les 30 s, cartographie toutes les 5 min, et lit les journaux
toutes les 2 s.

Un hook `PostToolUse` facultatif fait gagner environ une seconde — voir
[daemon/hooks/README.md](daemon/hooks/README.md). Sans lui, tout fonctionne.

## Vérifier

Les tests d'intégration parlent à la vraie pile Supabase locale, pas à un
simulacre : c'est le seul moyen de vérifier le contrat entre les structures
Rust et le schéma SQL.

```sh
supabase status                # relever les cles

cd daemon
VIBEMAP_TEST_URL=http://127.0.0.1:54321 \
VIBEMAP_TEST_SERVICE_KEY=<service_role> \
VIBEMAP_TEST_ANON_KEY=<anon> \
  cargo test
```

Côté web :

```sh
cd web && npx eslint . && npm run build
```

L'intégration continue ne joue que ce qui n'a besoin de personne : clippy, les
tests hors réseau, le lint et le build web. Les tests qui touchent la base se
lancent à la main avant chaque fusion.

## Pièges connus

- **Le trousseau macOS bloque le daemon en silence.** À chaque recompilation la
  signature du binaire change et le système redemande l'autorisation, sans rien
  afficher. Passe `VIBEMAP_TOKEN=<jeton>` pour t'en dispenser pendant le
  développement.
- **`supabase db reset` efface les comptes.** Il faut se reconnecter et
  réappairer la machine : le `machine_id` de la configuration devient invalide.
- **Le temps réel Supabase est un signal, pas une source de vérité.** Ses
  charges utiles arrivent incomplètes ; on relit toujours par l'API.

## Histoire

Ce dépôt a d'abord porté *Vibe Code Tracker*, une application macOS SwiftData
qui lisait les mêmes journaux hors ligne. Elle a été retirée au profit de Vibe
Map. Son code reste dans l'historique git, sous le commit `f435888`.

## Licence

MIT — voir [LICENSE](LICENSE).
