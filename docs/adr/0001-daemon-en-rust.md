# ADR 0001 : le daemon local est écrit en Rust

Date : 2026-08-01
Statut : accepté
Concerne : issue #1, section 10 de `docs/superpowers/specs/2026-08-01-vibe-map-observabilite-web-design.md`

## Contexte

Vibe Map a besoin d'un programme qui tourne en fond sur chaque machine de développement. Il scanne les repos clonés, suit les journaux de session de Claude Code, interroge les worktrees git, et pousse des métadonnées vers Supabase. Aucune interface, démarrage avec la machine, fonctionnement continu.

L'application web est Next.js, ce point n'a jamais été en jeu.

Le projet a un objectif déclaré au-delà de l'usage personnel : être publié sur GitHub et servir de pièce de portfolio.

Trois éléments de contexte pèsent sur la décision :

- Le code réellement réutilisable de l'application macOS abandonnée tient en 281 lignes : `ModelPricing.swift` (46) et `JSONLParser.swift` (235). Sur 78 fichiers Swift, le reste est de l'interface, jetée.
- Le daemon n'écrit que du HTTPS et du JSON vers Supabase. Il n'a besoin ni de Realtime, ni d'Auth, ni d'abonnements : c'est le web qui consomme le client complet.
- Le poste de développement n'a que Node installé. Aucune chaîne d'outils compilée n'est en place.

## Décision

Le daemon est écrit en **Rust**.

## Options écartées

**TypeScript compilé en binaire autonome.** L'option la plus rapide à mettre en œuvre : un seul langage, un seul dépôt, les types de la base générés une fois et partagés entre le daemon et le web. L'objection habituelle, exiger Node chez l'utilisateur, ne tient plus depuis que Bun compile un exécutable autonome. Écartée au profit de la qualité de l'artefact final : binaire de quelques mégaoctets contre quelques dizaines, empreinte mémoire bien plus faible pour un processus permanent, et valeur de démonstration supérieure sur un dépôt public.

**Go.** Techniquement le meilleur ajustement au problème : conçu pour les processus de fond, quatre boucles concurrentes triviales à écrire, compilation quasi instantanée, bibliothèque standard suffisante. Écarté pour la même raison que TypeScript, à l'envers : il coûte le même prix que Rust en double langage à maintenir, sans en offrir le bénéfice de démonstration.

**TypeScript d'abord, réécriture Rust ensuite.** Écartée parce qu'une réécriture annoncée mais non planifiée n'arrive jamais, et parce que le coût d'apprentissage est payé de toute façon, autant le payer une seule fois.

## Conséquences

### Ce qu'on gagne

Un binaire unique de quelques mégaoctets, sans dépendance à installer, qui tourne aussi bien sur un serveur Linux distant que sur le poste local. Une empreinte mémoire adaptée à un processus qui ne s'arrête jamais. Un compilateur qui refuse le code faux au lieu de le laisser échouer à trois heures du matin dans un processus sans interface.

### Ce qu'on paie

**Deux langages à tenir d'accord.** La forme des données existe désormais des deux côtés d'une frontière que rien ne vérifie automatiquement. C'est le risque principal de cette décision.

**Une chaîne d'outils à installer et à apprendre**, sur le composant le plus difficile à déboguer du projet, puisqu'il n'a pas d'interface.

**Une itération plus lente** pendant la phase où le schéma de données change encore.

### Comment on limite ces risques

Ces mesures ne sont pas facultatives : elles sont la contrepartie de la décision.

1. **Les migrations SQL sont l'unique source de vérité.** Les structures Rust et les types TypeScript en descendent tous les deux, aucun des deux ne définit le contrat.
2. **Un test d'intégration vérifie le contrat** en envoyant une charge réelle du daemon vers une base Supabase locale. Une divergence entre le schéma et les structures Rust casse la compilation ou le test, jamais la production.
3. **Le daemon reste mince.** Requêtes HTTP et sérialisation JSON, rien d'autre. Pas de couche d'abstraction sur la base, pas de client Supabase maison.
4. **Le schéma est figé avant d'écrire le daemon.** L'issue #3 pose les tables, l'issue #4 les événements. Le gros du code Rust vient après, quand la forme ne bouge plus.
5. **Des journaux réels sont versionnés comme échantillons de test.** Le lecteur de journaux se teste hors ligne, sans avoir à provoquer une vraie session d'agent.
6. **La compilation croisée et la publication des binaires sont automatisées** dès la première version, pour macOS arm64 et Linux x86_64. Un binaire qu'on ne sait pas produire n'existe pas.

### Choix techniques induits

| Besoin | Retenu |
|---|---|
| Exécution concurrente des quatre boucles | tokio |
| Surveillance du système de fichiers | notify |
| Requêtes vers Supabase | reqwest, avec serde pour le JSON |
| État git et worktrees | appel au binaire `git`, sortie `--porcelain` |
| Stockage du jeton machine | keyring, trousseau du système |
| Démarrage automatique | launchd sur macOS, systemd sur Linux |
| Installation | Homebrew sur macOS, binaire publié en release GitHub |

Le portage des 281 lignes de Swift vers Rust est traité dans les issues #4 et #7, pas comme un travail séparé.

## Ce qui reste vrai quoi qu'il arrive

Le contrat de données, le schéma, l'interface et les critères d'acceptation de la spec ne dépendent pas de cette décision. Si Rust devait s'avérer intenable, le remplacer ne changerait rien au reste du produit.
