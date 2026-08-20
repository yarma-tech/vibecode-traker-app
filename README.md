# Vibe Map

Une carte de tes codebases qui s'allume quand un agent y travaille.

> Dix secondes sur un onglet, entre deux autres, pour décider si tu lances un
> deuxième agent. Puis tu refermes.

Un dossier par parcelle, la surface proportionnelle aux lignes. Bleu quand on
lit, ambre quand on écrit, rouge quand deux agents se marchent dessus. La v1 ne
fait qu'observer : elle ne lance rien, n'arrête rien, ne corrige rien.

**Ce qui sort de ta machine est une liste fermée.** Des noms de dossiers
relatifs, des compteurs et des horodatages voyagent, comme avant ; le chemin
absolu d'un repo devient une empreinte. Deux lectures de contenu peuvent s'y
ajouter, et jamais d'autres : les titres lus dans un PRD - celui du document
et ceux de ses features -, et - plus tard - la phrase d'un verdict de
vérification. Ni code, ni diff, ni prompt, ni secret : aucune table n'a de
colonne où l'un d'eux pourrait entrer.

## Les quatre morceaux

| | |
|---|---|
| `daemon/` | Un binaire Rust sur ton poste. Il cartographie tes repos, lit les journaux de Claude Code, et pousse des métadonnées. Voir [l'ADR 0001](docs/adr/0001-daemon-en-rust.md). |
| `supabase/` | Le schéma : machines, repos, modules, sessions, activité. Toute la logique d'accès est en RLS ; révoquer une machine coupe ses écritures à la milliseconde. |
| `web/` | Next.js. La carte, le journal, le relevé. |
| `bureau/` | L'application macOS (Tauri). Elle sert `web/` depuis la machine sur `127.0.0.1:51789` et l'affiche dans sa fenêtre. Voir [le PRD-002](docs/prd/PRD-002-app-de-bureau.md). |

La conception complète est dans
[la spec](docs/superpowers/specs/2026-08-01-vibe-map-observabilite-web-design.md),
la personnalité du produit dans [PRODUCT.md](PRODUCT.md).

## Installer

Vibe Map ne vise qu'une plateforme : **macOS Apple Silicon**. Aucune version
Intel, Windows ou Linux n'est produite (PRD-002, FR-079).

### L'application de bureau

Chaque [release GitHub](https://github.com/yarma-tech/vibecode-traker-app/releases)
publie `VibeMap-<version>-aarch64-apple-darwin.app.zip` et sa somme de contrôle
sha256. Décompresse l'archive, glisse `Vibe Map.app` dans `/Applications`,
ouvre-la.

**Au premier lancement, macOS refuse de l'ouvrir.** Le message dit que
« *Vibe Map* ne peut pas être ouvert car son développeur n'a pas pu être
vérifié », avec un seul bouton pour l'envoyer à la corbeille. Ce n'est pas un
problème d'installation : l'application n'est ni signée ni notariée - cela
demande un compte développeur Apple payant, et c'est un hors-scope assumé du
PRD-002. Pour passer outre, **une seule fois** :

1. Dans le Finder, **clic droit** (ou Contrôle-clic) sur `Vibe Map.app`, puis
   **Ouvrir**.
2. Jusqu'à macOS 14, le même message revient avec un bouton **Ouvrir** : clique
   dessus, et c'est réglé.
3. Depuis macOS 15, Apple a retiré ce bouton du message. Ferme-le, puis va dans
   **Réglages Système › Confidentialité et sécurité** : tout en bas de la
   section *Sécurité*, juste après la tentative, un bouton **Ouvrir quand
   même** apparaît. Clique dessus et confirme.

Les lancements suivants ne demandent plus rien.

### Le binaire en ligne de commande

Il accompagne l'application sur le même Mac (PRD-002, FR-079). En une
commande :

```sh
brew install yarma-tech/vibemap/vibemap
```

Puis `vibemap --version` pour vérifier. La release publie aussi son archive
`vibemap-<version>-aarch64-apple-darwin.tar.gz` et sa somme de contrôle sha256.

La compilation, les sommes de contrôle et la publication de la release sont
automatisées à chaque tag `v*` (voir
[`.github/workflows/release.yml`](.github/workflows/release.yml) et
[`Formula/vibemap.rb`](Formula/vibemap.rb)). L'étiquette et les numéros de
version des manifestes doivent coïncider : la chaîne refuse de publier sinon,
plutôt que de livrer un paquet qui ment sur son propre numéro.

Il n'y a pas de démarrage automatique à l'ouverture de session : le lecteur vit
et meurt avec l'application de bureau (PRD-002, FR-011). Le plist launchd qui
existait ici est retiré du produit.

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

Pour l'application de bureau, il faut d'abord construire l'interface qu'elle
sert, puis la lancer :

```sh
cd web && npm run build
cd ../bureau && cargo run
```

Elle démarre le service d'interface sur `127.0.0.1:51789` - un port fixe,
jamais un autre - et charge cette origine dans sa fenêtre. Ce qui ne se
vérifie qu'à l'œil est listé dans
[bureau/VERIFICATIONS.md](bureau/VERIFICATIONS.md), à jouer avant chaque
fusion touchant `bureau/`.

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
