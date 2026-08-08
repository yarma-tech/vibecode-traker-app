# Contribuer à Vibe Map

Le dépôt tient trois morceaux : un daemon Rust (`daemon/`), un schéma Supabase
(`supabase/`) et une application web Next (`web/`). Le [README](README.md) dit
comment tout démarrer.

## Ce qui gouverne le code

- **Rien du code de l'utilisateur ne sort de sa machine.** Les chemins absolus
  deviennent des empreintes, les chemins de modules sont relatifs, et aucune
  table n'a de colonne où un contenu de fichier ou un prompt pourrait entrer.
  Une contribution qui ajoute une telle colonne sera refusée.
- **Les règles d'accès vivent dans la RLS**, pas dans le client. Une nouvelle
  table a ses policies *et* ses `grant` dans la même migration : sans les
  seconds, PostgREST répond « permission denied » malgré des policies justes.
- **L'état ne se stocke pas**, il se calcule à la lecture. Un état stocké serait
  faux la seconde suivante.
- **Les commentaires expliquent le pourquoi**, jamais le quoi. Ils sont en
  français, comme le reste du dépôt.
- **[Conventional Commits](https://www.conventionalcommits.org/)** :
  `feat(#12): …`, `fix: …`, `chore: …`, `docs: …`.

## Les tests d'abord

On écrit le test, on le regarde échouer, puis on écrit le code. Un test qui
passe du premier coup ne prouve rien.

Les tests d'intégration parlent à la **vraie pile Supabase locale**, pas à un
simulacre : c'est le seul moyen de vérifier le contrat entre les structures Rust
et le schéma SQL, et c'est exigé par [l'ADR 0001](docs/adr/0001-daemon-en-rust.md).

```sh
supabase start

cd daemon
VIBEMAP_TEST_URL=http://127.0.0.1:54321 \
VIBEMAP_TEST_SERVICE_KEY=<service_role> \
VIBEMAP_TEST_ANON_KEY=<anon> \
  cargo test

cd ../web && npx eslint . && npm run build
```

Une aide de test doit crier quand elle échoue. Une helper muette fait passer au
vert des tests qui devraient être rouges : les nôtres vérifient le code HTTP
**et** qu'une ligne a bien été touchée.

## Les demandes de fusion

1. Brancher depuis `main`.
2. Une PR, un sujet.
3. `cargo clippy --all-targets -- -D warnings` et la suite complète au vert,
   base comprise. L'intégration continue ne joue que ce qui n'a besoin de
   personne ; le reste est de ta responsabilité avant de demander la fusion.
4. Dire ce qui a été vérifié pour de vrai, et comment.

## Signaler un problème

Les gabarits d'issue sont là pour ça. Pour une faille de sécurité, écrire au
mainteneur plutôt que d'ouvrir une issue publique.
