---
id: PRD-001
titre: Espace projet Kanban
statut: validé
valide_le: 2026-08-11
date: 2026-08-10
maj: 2026-08-12
repo: vibecode-traker-app
---

# PRD-001 — Espace projet Kanban

## Contexte

Vibe Map répond aujourd'hui à « **où ça en est maintenant** » : quels agents tournent, sur quels dossiers, pour combien de jetons. La carte est un instrument de mesure de l'instant.

Cet espace répond à une autre question, à un autre rythme : « **où on en est tout court** ». Il vit à côté de la carte, pas dedans.

| | Carte (`/repo/[id]`) | Espace projet (`/repo/[id]/projet`) |
|---|---|---|
| Question | « Puis-je lancer un agent ici, et où ? » | « Où en est ce dépôt, et que reste-t-il ? » |
| Durée | 10 secondes, avant d'agir | 1–2 minutes, reprise de contexte |
| Horizon | l'instant | le projet |

L'utilisateur est celui de [`PRODUCT.md`](../../PRODUCT.md) : un développeur seul, une cinquantaine de dépôts, des agents en parallèle sur plusieurs machines.

### Vocabulaire

**Bloc** — l'unité du tableau, ce qu'on lit dans une colonne. Il porte un type, un titre, une référence et son avancement. Il est **simple** (ancré lui-même à un emplacement) ou **découpé** (il contient des issues).

**Issue** — l'unité de travail à l'intérieur d'un bloc. Elle porte un emplacement et une référence. Elle n'apparaît jamais seule dans une colonne.

**Emplacement** — un chemin du dépôt, relatif à la racine. C'est lui que l'activité des agents atteint.

**Référence** — `VM-7`, unique dans le dépôt, portée par tout travail suivi (bloc simple ou issue). C'est ce qu'on donne à l'agent qu'on lance, et ce qu'un commit doit écrire pour fermer.

Une feature transverse — « migrer l'authentification » — est **un bloc, trois issues** : `web/app/auth`, `daemon/src/auth.rs`, `supabase/migrations`. Un bloc simple reste simple : une correction d'une ligne n'a pas d'issue.

### Les quatre types

Ce qui les départage n'est pas le support produit mais **ce qui clôt le travail**.

| Type | Définition | Se termine par |
|---|---|---|
| **Feature** | à développer, n'existe pas encore | du code qui tourne |
| **Correction** | existe et ne marche pas comme prévu | du code qui tourne |
| **Technique** | existe et marche, mais coûte : refacto, montée de version, CI, performance | du code qui tourne |
| **Exploration** | on ne sait pas encore quoi faire : un PRD en cours d'écriture, un cadrage | **une décision** |

L'**exploration est une phase, pas une catégorie de document**, et pas une charge de travail : c'est un mémo, la trace qu'on a réfléchi à quelque chose. Un PRD non validé est **un** bloc, quel que soit le nombre de features qu'il décrit. À la validation, il **devient** ses features et ne laisse pas de carte derrière lui — une conversion, pas une suppression.

La documentation qui ne décide rien — README, guides, commentaires d'API — n'entre pas dans le tableau.

### Deux signaux, deux portées

C'est le cœur du mécanisme, et la décision la plus structurante du document.

| Signal | Ce qu'il prouve | Ce qu'il déclenche |
|---|---|---|
| Écriture à un emplacement | il se passe quelque chose ici | passage **en cours** |
| Référence dans le message d'un **commit local** | ce travail-ci est fait | passage **terminé** |

Un commit n'est pas lié à un endroit du code, il est lié à un **travail** : trois issues peuvent vivre dans `web/app/checkout` sans qu'aucun chemin ne dise laquelle un commit vient de régler. Le chemin est une trace, pas une déclaration — il entame, il ne ferme jamais.

« Terminé » veut dire **un commit local a nommé le travail**, pas « fusionné » ni « déployé » : le daemon lit le disque, pas la forge. Attendre la fusion serait attendre un signal qui n'arrive pas.

### D'où viennent les blocs

| Origine | Ce qui la déclenche | Type | Arrive en |
|---|---|---|---|
| Un PRD | le document apparaît (exploration), puis il est validé (ses features) | exploration, puis feature | En cours, puis À faire |
| Le PM | une tâche saisie directement | correction, technique, feature | À faire |
| Un agent | un document de conception écrit là où il n'y en avait pas | exploration | En cours |

La colonne **À faire** est celle de l'intention : le dépôt peut être automatique, la décision jamais. **Sans validation, pas de feature.**

## Problème

Pour un développeur solo qui lance plusieurs agents en parallèle, l'état d'avancement d'un dépôt est réparti entre un `TODO.md` qui se périme, un `git log` qu'il faut lire, et sa mémoire. Rien ne relie l'intention (« refaire la landing ») au fait (« commit `a3f9` a touché `web/app/landing` »).

Le coût réel : après quelques jours d'absence sur un dépôt, il faut relire du code pour savoir si une fonctionnalité est finie — et, quand elle est longue, pour savoir **combien il en reste**.

## Features à développer

### F1 — Déclarer un travail (Priorité : P1)

- **User story** : En tant que développeur, je veux déclarer un travail en une saisie courte afin de ne pas tenir un backlog à part.
- **Pourquoi cette priorité** : sans saisie, il n'y a pas de tableau. C'est la fondation de tout le reste.
- **Exigences** :
  - **FR-001** : Le système DOIT permettre de créer un bloc avec un titre, un type et un emplacement.
  - **FR-002** : Le système DOIT proposer l'emplacement par autocomplétion sur les dossiers connus du dépôt, et accepter la saisie libre d'un fichier.
  - **FR-003** : Le système DOIT attribuer à tout travail créé une référence unique dans le dépôt, de la forme `VM-7`.
  - **FR-004** : Le système NE DOIT JAMAIS réattribuer une référence libérée par une suppression.
- **Critères d'acceptation** :
  - [ ] Étant donné un dépôt cartographié, quand je crée un bloc avec un titre et un emplacement, alors il apparaît en « À faire » avec une référence.
  - [ ] Étant donné une issue `VM-7` supprimée, quand je crée un travail, alors sa référence n'est pas `VM-7`.
- **Hors scope** : échéances, assignés, étiquettes libres.

### F2 — Découper un bloc en issues (Priorité : P1)

- **User story** : En tant que développeur, je veux découper un chantier long en issues afin de voir ce qu'il me reste plutôt qu'une case opaque.
- **Pourquoi cette priorité** : c'est ce qui distingue ce tableau d'une liste de cases à cocher.
- **Exigences** :
  - **FR-005** : Le système DOIT permettre de découper un bloc à tout moment, y compris après sa création.
  - **FR-006** : QUAND un bloc simple reçoit sa première issue, le système DOIT faire descendre l'emplacement du bloc dans cette issue et retirer l'emplacement du bloc.
  - **FR-007** : Le système NE DOIT PAS permettre à un bloc de porter à la fois un emplacement et des issues.
  - **FR-008** : Le système NE DOIT PAS permettre à une issue de contenir des sous-issues.
- **Critères d'acceptation** :
  - [ ] Étant donné un bloc simple ancré à `web/app/hero`, quand j'y ajoute une première issue, alors cette issue porte `web/app/hero` et le bloc n'a plus d'emplacement.
- **Hors scope** : dépendances entre issues, ordonnancement imposé.

### F3 — Entamer un travail sans intervention (Priorité : P1)

- **User story** : En tant que développeur, je veux qu'un travail passe en « En cours » dès qu'un agent y écrit afin de ne rien avoir à déplacer.
- **Pourquoi cette priorité** : c'est la moitié de la promesse « se tient à jour tout seul », et le signal existe déjà dans la télémétrie.
- **Exigences** :
  - **FR-009** : QUAND un agent écrit dans un fichier, le système DOIT passer en « en cours » le travail vivant dont l'emplacement est le plus profond à préfixer ce chemin.
  - **FR-010** : Le système NE DOIT PAS rouvrir un travail terminé sur la seule foi d'une écriture — écrire dans un dossier n'est pas reprendre un travail livré.
  - **FR-011** : QUAND plusieurs travaux vivants partagent un emplacement, le système DOIT tous les passer en « en cours ».
- **Critères d'acceptation** :
  - [ ] Étant donné six issues ancrées à `web/app/checkout`, quand un agent y écrit, alors les six passent en « En cours ».
  - [ ] Étant donné une issue terminée, quand un agent écrit à son emplacement, alors elle reste terminée.
- **Hors scope** : distinguer quel agent a écrit.

### F4 — Fermer un travail par une référence (Priorité : P1)

- **User story** : En tant que développeur, je veux qu'un commit ferme le travail qu'il nomme afin que « Terminé » veuille dire quelque chose.
- **Pourquoi cette priorité** : c'est l'autre moitié de la promesse, et la seule affirmation forte du tableau.
- **Exigences** :
  - **FR-012** : QUAND un commit local est ingéré, le système DOIT fermer exactement les travaux que son message désigne par leur référence.
  - **FR-013** : Le système NE DOIT PAS fermer un travail sur la seule foi des chemins touchés par un commit.
  - **FR-014** : QUAND une référence désigne un travail déjà terminé, le système DOIT le rouvrir en version suivante.
  - **FR-015** : QUAND une référence ne correspond à aucun travail du dépôt, le système DOIT l'ignorer sans bruit.
  - **FR-016** : Le système DOIT reconnaître la forme `VM-7` et NE DOIT PAS reconnaître la forme `#7`, déjà employée pour les issues GitHub.
- **Critères d'acceptation** :
  - [ ] Étant donné un commit dont le message contient six références, quand il est ingéré, alors les six travaux sont fermés.
  - [ ] Étant donné un commit `feat(#7): …`, quand il est ingéré, alors le travail `VM-7` n'est pas fermé.
  - [ ] Étant donné un commit touchant l'emplacement d'un travail vivant sans le nommer, quand il est ingéré, alors rien ne se ferme.
- **Hors scope** : réconciliation d'un historique réécrit (rebase, amend).

### F5 — Dériver l'état d'un bloc découpé (Priorité : P1)

- **User story** : En tant que développeur, je veux qu'un bloc suive ses issues afin de ne jamais avoir à tenir deux états cohérents à la main.
- **Pourquoi cette priorité** : sans dérivation, un bloc et ses issues peuvent se contredire, et le tableau ment.
- **Exigences** :
  - **FR-017** : Le système DOIT déduire l'état d'un bloc découpé de celui de ses issues : toutes à faire → À faire ; au moins une entamée ou terminée → En cours ; toutes terminées → Terminé.
  - **FR-018** : Le système NE DOIT PAS permettre de déplacer un bloc découpé directement.
- **Critères d'acceptation** :
  - [ ] Étant donné un bloc de neuf issues dont une est entamée, quand je consulte le tableau, alors le bloc est en « En cours ».
- **Hors scope** : pondération des issues.

### F6 — Lire le reste à faire (Priorité : P1)

- **User story** : En tant que développeur, je veux lire `12 / 17` sur un bloc afin de mesurer un chantier sans l'ouvrir.
- **Pourquoi cette priorité** : c'est la réponse directe à « combien il en reste », la question du problème.
- **Exigences** :
  - **FR-019** : Le système DOIT afficher sur un bloc découpé le nombre d'issues terminées sur le total.
  - **FR-020** : Le système DOIT permettre d'ouvrir un bloc pour voir quelles issues restent, sans quitter le tableau.
  - **FR-021** : Le système NE DOIT PAS afficher les issues comme des cartes autonomes dans une colonne.
- **Critères d'acceptation** :
  - [ ] Étant donné un bloc de dix-sept issues, quand je consulte la colonne, alors je vois une ligne et son avancement, pas dix-sept cartes.
- **Hors scope** : estimation, points, vélocité.

### F7 — Suivre un travail qui repart (Priorité : P1)

- **User story** : En tant que développeur, je veux qu'un travail livré puis repris reste la même carte afin de garder son histoire.
- **Pourquoi cette priorité** : sans versionnage, une itération crée un doublon et le tableau se dédouble à chaque reprise.
- **Exigences** :
  - **FR-022** : QUAND un travail terminé est rouvert, le système DOIT incrémenter sa version et le passer en « en cours ».
  - **FR-023** : Le système NE DOIT JAMAIS ramener un travail en « À faire » automatiquement.
  - **FR-024** : Le système DOIT conserver, pour chaque version, le commit qui l'a close.
- **Critères d'acceptation** :
  - [ ] Étant donné un travail terminé en v1, quand un commit le nomme de nouveau, alors il est en « En cours » v2 et son historique porte deux commits.
- **Hors scope** : comparaison entre versions.

### F8 — Corriger une fermeture à tort (Priorité : P1)

- **User story** : En tant que développeur, je veux ramener une carte fermée à tort afin de corriger l'automatisme sans pouvoir le tromper.
- **Pourquoi cette priorité** : fermer sur un commit local ferme parfois trop tôt ; il faut une sortie.
- **Exigences** :
  - **FR-025** : Le système DOIT permettre de ramener un travail de « Terminé » vers « En cours ».
  - **FR-026** : Le système NE DOIT PAS permettre de déplacer un travail vers « Terminé » à la main.
  - **FR-027** : Le système NE DOIT PAS permettre de soustraire un travail à l'automatisme — pas d'épinglage, pas de gel.
- **Critères d'acceptation** :
  - [ ] Étant donné une carte en « Terminé », quand je tente de la glisser depuis « En cours » vers « Terminé », alors le geste n'existe pas et le serveur refuse l'écriture.
  - [ ] Étant donné une carte ramenée en « En cours », quand un commit la nomme, alors elle se referme.
- **Hors scope** : historique des corrections manuelles.

### F9 — Voir le tableau se mettre à jour à distance (Priorité : P1)

- **User story** : En tant que développeur, je veux voir le tableau bouger sur un appareil pendant que je travaille sur un autre afin de ne jamais rafraîchir.
- **Pourquoi cette priorité** : le travail se fait sur plusieurs machines ; un tableau qui demande un rechargement n'est pas consulté.
- **Exigences** :
  - **FR-028** : QUAND un agent écrit, le système DOIT refléter le changement sur un autre appareil en moins de 5 secondes.
  - **FR-029** : QUAND un commit ferme un travail, le système DOIT le refléter en moins d'une minute.
  - **FR-030** : Le système DOIT rendre le même tableau pour tous les clones d'un même dépôt distant.
- **Critères d'acceptation** :
  - [ ] Étant donné le même dépôt cloné sur deux machines, quand je crée un bloc sur l'une, alors il apparaît sur l'autre.
  - [ ] Étant donné un dossier de dépôt renommé, quand la cartographie repasse, alors le tableau est conservé.
- **Hors scope** : résolution de conflits d'édition simultanée.

### F10 — Lire et filtrer par type (Priorité : P1)

- **User story** : En tant que développeur, je veux filtrer par type et corriger une étiquette afin de lire ma semaine sans me tromper de rangement.
- **Pourquoi cette priorité** : dix corrections et deux features ne racontent pas la même semaine que l'inverse.
- **Exigences** :
  - **FR-031** : Le système DOIT permettre de filtrer le tableau par type.
  - **FR-032** : Le système DOIT permettre de changer le type d'un travail à tout moment, sans effet sur son état.
  - **FR-033** : Le système DOIT porter chaque type par un libellé textuel, jamais par la seule couleur.
- **Critères d'acceptation** :
  - [ ] Étant donné une carte typée « correction », quand je la retype en « feature », alors son état et son historique sont inchangés.
- **Hors scope** : types définis par l'utilisateur.

### F11 — Copier une référence pour lancer un agent (Priorité : P1)

- **User story** : En tant que développeur, je veux copier `VM-7` d'un geste afin de le passer à l'agent que je lance.
- **Pourquoi cette priorité** : c'est le seul maillon manuel de la chaîne de fermeture ; s'il est pénible, la chaîne casse.
- **Exigences** :
  - **FR-034** : Le système DOIT afficher la référence sur chaque carte et permettre de la copier sans souris.
  - **FR-035** : Le système DOIT rendre accessibles au clavier la création, le déplacement, l'ouverture d'un bloc et la copie d'une référence.
- **Critères d'acceptation** :
  - [ ] Étant donné le tableau ouvert, quand je navigue au clavier seul, alors je peux créer un travail et copier sa référence.
- **Hors scope** : intégration directe avec le lancement d'un agent.

### F12 — Peupler « À faire » depuis un PRD (Priorité : P2)

- **User story** : En tant que développeur, je veux que les features d'un PRD validé apparaissent afin de ne pas retranscrire ce que je viens d'écrire.
- **Pourquoi cette priorité** : forte valeur, mais le tableau est utilisable sans — la saisie manuelle couvre le besoin.
- **Exigences** :
  - **FR-036** : Le système DOIT lire les fichiers markdown dont l'en-tête porte `id`, `statut`, `date` et `repo`, et ignorer les autres.
  - **FR-037** : QUAND `repo` désigne un autre dépôt, le système NE DOIT PAS peupler le tableau courant.
  - **FR-038** : QUAND `statut` vaut `draft`, le système DOIT créer un unique bloc de type exploration et aucune feature.
  - **FR-039** : QUAND `statut` passe à `validé`, le système DOIT créer un bloc par feature et retirer le bloc d'exploration s'il ne porte ni issue ni fermeture.
  - **FR-040** : Le système DOIT identifier chaque feature par la clé `<date>/<id>/<Fn>` et rattacher par cette clé, jamais par le titre.
  - **FR-041** : QUAND une feature disparaît du document, le système DOIT conserver son bloc et le marquer, sans jamais le supprimer.
  - **FR-042** : Le système DOIT afficher la priorité lue dans le document sans permettre de la modifier depuis le tableau.
  - **FR-043** : Le système NE DOIT transmettre hors de la machine que la clé, le titre, la priorité, le marqueur « à clarifier », le statut et les dates du document.
  - **FR-044** : QUAND un document porte l'en-tête mais qu'aucune feature n'y est reconnue, le système DOIT le signaler à l'écran.
- **Critères d'acceptation** :
  - [ ] Étant donné un PRD `draft` de douze features, quand il est lu, alors le tableau porte un bloc d'exploration et zéro feature.
  - [ ] Étant donné ce même PRD passé en `validé`, quand il est relu, alors le tableau porte douze features en « À faire » et plus de bloc d'exploration.
  - [ ] Étant donné un PRD déjà lu, quand il est relu sans changement, alors aucun bloc n'est créé.
- **Hors scope** : extraction par un modèle, import d'un `TODO.md` ou d'un tracker externe, édition d'une feature depuis le tableau.
- **[À CLARIFIER]** : ce PRD-ci ne suit pas encore le gabarit qu'il décrit ; il sera le premier cas de test du parseur.

### F13 — Constater une exploration (Priorité : P2)

- **User story** : En tant que développeur, je veux qu'un document de conception écrit par un agent laisse une trace afin de me souvenir des décisions prises.
- **Pourquoi cette priorité** : confort de mémoire, sans effet sur la charge de travail.
- **Exigences** :
  - **FR-045** : QUAND un agent écrit sous `docs/adr/`, `docs/superpowers/specs/` ou `docs/superpowers/plans/` un fichier qu'aucun bloc ne couvre, le système DOIT créer un bloc de type exploration en « En cours ».
  - **FR-046** : Le système DOIT titrer ce bloc d'après le nom du fichier, sans lire son contenu.
  - **FR-047** : Le système NE DOIT PAS compter les explorations dans un avancement ou un reste à faire.
  - **FR-048** : Le système DOIT permettre de renommer, retyper ou supprimer un bloc qu'il a créé seul.
- **Critères d'acceptation** :
  - [ ] Étant donné un agent qui écrit `docs/adr/0012-file-attente.md`, quand la carte apparaît, alors elle est titrée d'après le fichier et n'entre dans aucun total.
- **Hors scope** : explorations sans artefact sur le disque, documents créés hors des outils d'écriture d'agent.

### F14 — Savoir si ce qu'on lit est frais (Priorité : P2)

- **User story** : En tant que développeur, je veux distinguer « rien à faire » de « rien ne remonte » afin de ne pas prendre une panne pour du calme.
- **Pourquoi cette priorité** : un tableau figé est indiscernable d'un tableau stable, et un outil qui ment une fois n'est plus consulté.
- **Exigences** :
  - **FR-049** : Le système DOIT afficher depuis quand il n'a reçu aucun signal pour un dépôt.
  - **FR-050** : QUAND aucun signal récent n'est reçu, le système NE DOIT PAS présenter un tableau vide comme un travail achevé.
  - **FR-051** : Le système DOIT présenter un tableau neuf par un aperçu de ce qu'il deviendra, et rappeler que « Terminé » restera vide malgré l'historique du dépôt.
- **Critères d'acceptation** :
  - [ ] Étant donné un daemon arrêté depuis trois heures, quand j'ouvre l'espace, alors il indique l'absence de signal au lieu d'un tableau muet.
  - [ ] Étant donné un tableau dont tout est terminé et un signal récent, quand j'ouvre l'espace, alors il le présente comme un achèvement.
- **Hors scope** : notifications, alertes poussées.

### F15 — Faire vérifier un travail (Priorité : P2)

- **User story** : En tant que développeur, je veux faire vérifier un travail dont j'ai oublié la référence afin de le fermer sans relire le code.
- **Pourquoi cette priorité** : c'est le rattrapage qui rend vivable la fermeture par référence ; sans lui, les oublis restent ouverts pour toujours.
- **Exigences** :
  - **FR-052** : QUAND un commit touche l'emplacement d'un travail sans le nommer, le système DOIT proposer de le vérifier.
  - **FR-053** : Le système DOIT exécuter la vérification sur la machine ayant eu l'activité la plus récente sur ce dépôt, en lecture seule.
  - **FR-054** : Le système NE DOIT PAS transmettre d'instruction depuis le serveur vers la machine : la demande ne porte que la désignation du travail.
  - **FR-055** : Le système NE DOIT retourner qu'un verdict, une confiance, des chemins relatifs et une phrase — jamais d'extrait de code.
  - **FR-056** : Le système DOIT borner chaque vérification par un délai maximal, une seule en cours par dépôt, et un plafond de jetons connu.
  - **FR-057** : Le système NE DOIT JAMAIS fermer un travail sur la seule foi d'un verdict ; le verdict fait apparaître l'unique bouton de fermeture du produit.
  - **FR-058** : QUAND la machine désignée ne répond pas dans le délai, le système DOIT le dire et proposer la suivante sans basculer seul.
- **Critères d'acceptation** :
  - [ ] Étant donné un travail non nommé par un commit, quand je demande une vérification, alors j'obtiens un verdict et un bouton « Fermer ».
  - [ ] Étant donné une vérification en cours sur un dépôt, quand j'en demande une seconde, alors elle est refusée.
- **Hors scope** : vérification automatique sans demande.

### F16 — Signaler un emplacement disparu (Priorité : P3)

- **User story** : En tant que développeur, je veux être averti qu'un travail vise un dossier qui n'existe plus afin de ne pas l'attendre indéfiniment.
- **Pourquoi cette priorité** : cas rare, information gratuite — la cartographie passe déjà.
- **Exigences** :
  - **FR-059** : QUAND l'emplacement d'un travail vivant n'existe plus dans le dépôt cartographié, le système DOIT le signaler sur la carte.
  - **FR-060** : Le système NE DOIT PAS corriger l'emplacement à la place de l'utilisateur.
- **Critères d'acceptation** :
  - [ ] Étant donné une issue ancrée à un dossier renommé, quand la cartographie repasse, alors la carte indique que son emplacement vise le vide.
- **Hors scope** : suivi des renommages, réancrage automatique.

## Hors scope global

- Le glisser-déposer vers « Terminé », et tout geste qui déclare un travail fini sans preuve.
- L'épinglage ou le gel d'une carte contre l'automatisme.
- Le rejeu de l'historique git antérieur au branchement du dépôt.
- La réconciliation d'un historique réécrit : rebase et amend produisent de nouveaux commits.
- Les échéances, assignés et étiquettes libres. La priorité fait exception : lue dans le PRD, affichée, jamais gérée.
- Un troisième niveau d'imbrication : une issue ne se découpe pas.
- Les dépendances entre issues et l'ordonnancement imposé.
- Le suivi de la documentation qui ne décide rien, et des explorations sans artefact.
- Le multi-utilisateur, les commentaires, les mentions.
- La péremption ou l'archivage automatique des travaux anciens.

## Critères de succès mesurables

- Après un commit nommant une référence sur une machine, la carte est en « Terminé » sur un autre appareil en **moins d'une minute**, sans intervention.
- Après une écriture d'agent, une carte est en « En cours » sur un autre appareil en **moins de 5 secondes**.
- Sur une semaine d'usage réel, les fermetures après vérification restent l'**exception** : elles mesurent les références non passées à l'agent, pas une défaillance de l'automatisme.
- En reprenant un dépôt laissé de côté deux semaines, l'utilisateur sait où il en est **et combien il en reste** sans ouvrir un terminal ni relire du code.
- Un chantier à double chiffre d'issues se lit **sans effort** : une ligne dans une colonne, le détail à la demande.
- Les décisions de conception écrites par un agent pendant la semaine apparaissent seules, et **moins d'une sur cinq** est supprimée comme non pertinente.
- Aucune carte n'est jamais en « Terminé » sans qu'un commit l'ait nommée ou qu'un humain l'ait confirmée après vérification.

## Hypothèses

- L'utilisateur committe ce qu'il fait faire. Un travail livré sans commit est invisible au produit, par construction.
- Les agents écrivent les messages de commit, donc la référence peut leur être passée au lancement sans coût pour l'humain.
- Le découpage d'un chantier en issues reste un acte humain : rien ne devine que « refonte du tunnel » vaut neuf issues.
- Les PRD sont rédigés selon le gabarit de ce document. Un PRD hors convention ne peuple rien, et c'est un choix : on corrige le document, on ne devine pas.
- Le dépôt a un distant. Sans lui, le tableau reste attaché à un clone et ne survit pas à un déplacement de dossier.

## Contraintes techniques

La conception détaillée est dans [la spec du 2026-08-11](../superpowers/specs/2026-08-11-espace-projet-kanban-conception.md). Les contraintes qui pèsent sur le produit :

- **Le daemon lit le disque, pas la forge.** Il n'appelle jamais `git fetch` et ne connaît pas la branche de référence : « terminé » ne peut donc pas vouloir dire « fusionné ».
- **Ce qui sort de la machine est une liste fermée**, publiée et bornée par un plancher qui ne bouge pas : ni code, ni diff, ni prompt, ni chemin absolu, ni secret. Deux lectures de contenu sont admises — les titres de features d'un PRD, et la phrase d'un verdict de vérification. Cette règle remplace la promesse absolue portée aujourd'hui par `README.md` (l. 12-15), qui devra être réécrit **dans le commit qui livre la première de ces deux lectures**, jamais avant.
- **L'identité d'un dépôt vient de son distant normalisé**, pas de son chemin absolu. C'est ce qui permet à un tableau de survivre à un `mv` et d'être partagé entre clones.
- **Cette version de Next.js a des ruptures d'API** : lire `node_modules/next/dist/docs/` avant d'écrire du code d'app (`web/AGENTS.md`).
- **Registre visuel Linear**, aligné sur la carte : densité maîtrisée, la couleur ne dit que l'état, jamais la couleur seule, mouvement coupé sous `prefers-reduced-motion`, WCAG AA, clavier d'abord.

## Risques

**Une branche abandonnée ferme quand même.** Fermer sur le commit local, c'est fermer avant de savoir si le travail survivra. Atténuation : la sortie de « Terminé » reste possible, et un solo qui committe abandonne rarement.

**Un commit non référencé ne ferme rien.** Le tableau montrera des travaux « en cours » qui sont finis, tant que l'habitude n'est pas prise. C'est le choix inverse de la fermeture au chemin, et il est assumé : **mieux vaut rater une fermeture que d'en inventer une**. Une carte qui traîne se remarque ; une carte fermée à tort passe inaperçue et ment.

**Un emplacement large entame large.** Une issue ancrée à `web/` passera en cours au premier agent qui y écrit. Acceptable : « en cours » n'est pas une affirmation forte.

**`12 / 17` compte des issues, pas de l'effort.** Neuf issues triviales et une énorme donnent `9 / 10` alors qu'il reste l'essentiel.

**Le PRD devient une pièce du système.** Il ne se rédige plus tout à fait librement, et cela vaut aussi pour les agents qui en écrivent une bonne part.

**Vérifier, c'est exécuter.** Le bouton de F15 fait tourner du code sur la machine depuis une action web. Même verrouillé — pas de texte libre, lecture seule, retour borné — c'est une porte qui n'existait pas. Elle se relit avant de se coder.

**Le produit s'écarte de `PRODUCT.md` sur quatre points**, tous volontaires et bornés :

| Écart | Ce que `PRODUCT.md` dit | Ce que l'espace projet fait | Ce qui le borne |
|---|---|---|---|
| Il **crée** | « il montre l'état » | fait naître une exploration, tire des features d'un PRD | il ne crée que ce qu'il a lu quelque part |
| Il **exécute** | « la v1 observe, elle ne pilote pas » | lance un sous-agent de vérification | sur demande, lecture seule, borné |
| Il **commente** | « il ne le commente pas » | affiche une phrase de verdict | une phrase, jamais d'extrait, après un clic |
| Il **reçoit de la saisie** | « Vibe Map ne se configure pas » | titres, types, emplacements, découpages | du contenu, pas de la configuration |

La carte, elle, ne bouge pas : elle reste l'instrument de mesure de `PRODUCT.md`. L'espace projet est un voisin qui a d'autres manières, pas une dérive de la carte.

**On a échangé une promesse simple contre une liste à tenir.** « Ton code ne quitte pas ta machine » se vérifiait d'un coup d'œil au schéma. Une liste fermée se maintient : chaque fonctionnalité future devra dire si elle y ajoute une ligne, et le README devra suivre.
