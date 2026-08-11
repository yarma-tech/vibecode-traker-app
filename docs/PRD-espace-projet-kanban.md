# PRD — Espace projet Vibe Map (Kanban lié au git)

> **Statut** : brouillon v24 — relu par un agent le 2026-08-10 ; toutes les questions bloquantes sont tranchées (§10.1)
> **Date** : 2026-08-10
> **Produit** : Vibe Map (voir [PRODUCT.md](../PRODUCT.md))
> **Conception technique** : [spec du 2026-08-09](superpowers/specs/2026-08-09-espace-projet-kanban-design.md) — **désormais en retard sur ce PRD**, voir §11

---

## 1. Problème

Vibe Map répond aujourd'hui à « **où ça en est maintenant** » : quels agents tournent, sur quels dossiers, pour combien de jetons. Il ne répond pas à « **où on en est tout court** » : ce qui est prévu, ce qui est en train de se faire, ce qui est livré.

Pour un développeur solo qui lance plusieurs agents en parallèle, cette seconde question est répartie entre un `TODO.md` qui se périme, un `git log` qu'il faut lire, et sa mémoire. Rien ne relie l'intention (« refaire la landing ») au fait (« commit `a3f9` a touché `web/app/landing` »).

Le coût réel : après quelques jours d'absence sur un dépôt, il faut relire du code pour savoir si une fonctionnalité est finie — et, quand elle est longue, pour savoir **combien il en reste**.

## 2. Promesse

Un tableau par dépôt qui **se tient à jour tout seul**. On y écrit une fois ce qu'on veut faire ; l'avancement se déduit de ce que les agents écrivent et de ce qu'ils committent.

Un chantier long n'est pas une case opaque : on l'ouvre et on voit ce qu'il reste.

Le tableau **reflète** le code, il ne le pilote pas. C'est un miroir git, pas un gestionnaire de tickets : la vérité est dans le dépôt — le PRD pour ce qui est prévu, les commits pour ce qui est fait.

## 3. Utilisateur et moment d'usage

L'utilisateur de PRODUCT.md — un développeur seul, une cinquantaine de dépôts, des agents en parallèle sur plusieurs machines.

Le moment d'usage diffère de celui de la carte :

| | Carte (`/repo/[id]`) | Espace projet (`/repo/[id]/projet`) |
|---|---|---|
| Question | « Puis-je lancer un agent ici, et où ? » | « Où en est ce dépôt, et que reste-t-il ? » |
| Durée | 10 secondes, avant d'agir | 1–2 minutes, reprise de contexte |
| Fréquence | plusieurs fois par jour | à la reprise d'un dépôt, en fin de journée |
| Horizon | l'instant | le projet |

Les deux espaces sont voisins, pas imbriqués : la carte reste le produit (PRODUCT §5), le Kanban est l'espace d'à côté.

## 4. Vocabulaire

Deux niveaux, un seul déplaçable.

**Bloc** — l'unité du tableau. C'est ce qu'on lit dans une colonne. Il porte un **type** (§5), un titre, et son avancement. Un bloc est soit **simple** (rien dedans, ancré lui-même à un emplacement), soit **découpé** (il contient des issues).

**Issue** — l'unité de travail. Elle porte un emplacement dans le code (dossier ou fichier), c'est elle que l'activité des agents et les commits atteignent. Une issue n'apparaît pas seule dans une colonne : elle vit à l'intérieur de son bloc.

**Emplacement** — un chemin du dépôt, relatif à la racine. C'est l'ancrage qui permet à l'automatisme de trouver le bon travail sans qu'on ait rien à relier à la main.

Conséquence directe : une feature transverse — « migrer l'authentification » — est **un bloc, trois issues** : `web/app/auth`, `daemon/src/auth.rs`, `supabase/migrations`. Le bloc n'a pas besoin d'emplacement propre ; ses issues en ont un chacune.

Un bloc simple reste simple : une correction d'une ligne est un bloc sans issue, ancré directement. On ne découpe que ce qui le mérite.

Quand on découpe un bloc simple **après coup**, son emplacement descend et devient sa première issue. Le bloc n'en garde pas : deux sources d'état sur le même objet se contrediraient tôt ou tard (règle 4).

## 5. Les types de travail

Quatre types. Les trois premiers se distinguent par une seule question — **est-ce que ça existe déjà en production, et est-ce que c'est cassé ?** Le quatrième n'est pas de même nature : c'est une **phase**, celle d'avant la décision.

| Type | Définition | Se termine par |
|---|---|---|
| **Feature** | à développer, n'existe pas encore | du code qui tourne |
| **Correction** | existe et ne marche pas comme prévu | du code qui tourne |
| **Technique** | existe et marche, mais coûte : refactorisation, montée de version, CI, performance | du code qui tourne |
| **Exploration** | on ne sait pas encore quoi faire : un PRD en cours d'écriture, un cadrage, une décision à prendre | **une décision** |

Le critère qui départage n'est pas le support — du code d'un côté, du markdown de l'autre — c'est **ce qui clôt le travail**. Une exploration se termine quand on a tranché. Une feature se termine quand quelque chose fonctionne.

Le type ne change pas la mécanique : les quatre traversent les mêmes colonnes, avec les mêmes règles. Il change **la lecture** — dix corrections et deux features ne racontent pas la même semaine que l'inverse. Il est porté par un libellé, jamais par la seule couleur, et se modifie à tout moment.

### 5.1 L'exploration est une phase, pas un dossier

**Un PRD non validé est une exploration.** Un seul bloc, quel que soit le nombre de features qu'il décrit. Tant que le document n'est pas validé, ce qu'il contient n'est pas décidé : le faire apparaître en douze cartes donnerait à un brouillon l'autorité d'un plan.

**Et une exploration n'est pas une charge de travail.** C'est un **mémo** — la trace qu'on a réfléchi à quelque chose, pour s'en souvenir. Elle ne compte dans aucun total, aucun reste à faire, aucun avancement. Elle dit *on a commencé à y penser*, pas *il y a ça à faire*. Le compte de features s'affiche sur la carte, mais comme un ordre de grandeur, jamais comme du travail engagé.

**À la validation, l'exploration devient les features.** Elle ne se range pas dans « Terminé » et ne laisse pas de carte derrière elle : les douze features **la remplacent** en « À faire ». Le mémo a rempli son office au moment où il se transforme en travail décidé — sa trace, désormais, ce sont les cartes qu'il a produites.

C'est une conversion, pas une suppression : la règle 7 tient, rien n'est effacé. Ce qui existait sous une forme continue sous une autre.

C'est la règle qui commande tout le reste : **sans validation, pas de feature.**

**Ce qui n'entre pas dans le tableau.** La documentation qui ne décide rien — README, guide d'installation, commentaires d'API — n'a rien à y faire. Elle ne se termine ni par une décision ni par une capacité nouvelle. On ne la suit pas.

**Une exploration qui ne produit pas de features.** Un ADR, un spec de conception : ça se termine bien par une décision, mais aucune feature n'en sort. Le mémo reste un mémo — il se ferme quand la décision est prise, sans rien engendrer. Deux formes d'exploration, donc, un seul critère : *ça finit par un choix arrêté*, et ni l'une ni l'autre ne pèse sur la charge de travail.

## 6. Colonnes et avancement

Trois colonnes : **À faire · En cours · Terminé**.

**Bloc simple** — se comporte comme une carte unique : `À faire` à la création, `En cours` dès qu'un agent écrit à son emplacement, `Terminé` quand un commit le nomme (§6.3). Il porte sa propre référence, exactement comme une issue.

**Bloc découpé** — son état ne se saisit pas, il **se déduit** de ses issues :

| État des issues | Colonne du bloc |
|---|---|
| toutes à faire | À faire |
| au moins une entamée ou terminée, pas toutes terminées | En cours |
| toutes terminées | Terminé |

L'avancement se lit sur le bloc : **`12 / 17`** — nombre d'issues terminées sur le total. Le reste à faire s'ouvre dans le bloc, sans quitter le tableau. C'est ça, la to-do list : ce qui n'est pas encore terminé à l'intérieur d'un bloc en cours.

L'étendue d'un chantier se mesure donc en **issues restantes**, pas en commits. Les commits sont la preuve, pas l'unité.

« Terminé » veut dire **un commit local l'a nommé** (§10.1). Pas « fusionné », pas « déployé » : ce que le daemon voit, c'est ton disque. Le jour où le commit part sur une branche qui n'aboutit pas, la carte est fermée à tort — c'est le prix, il est en §13.

### 6.1 D'où viennent les blocs

Trois origines, dont une seule passe par le clavier.

| Origine | Ce qui la déclenche | Type produit | Arrive en |
|---|---|---|---|
| **Un PRD** | un PRD apparaît dans le dépôt (exploration), puis il est validé (ses features) | exploration, puis feature | En cours, puis À faire |
| **Le PM** | une tâche saisie directement — une correction constatée, un chantier technique décidé | correction, technique, feature | À faire |
| **Un agent** | un document de conception écrit là où il n'y en avait pas (§5.1) | exploration | En cours |

La colonne **À faire** reste la colonne de l'**intention** : tout ce qui s'y trouve a été décidé par un humain, que ce soit écrit dans un PRD ou saisi à la main. L'automatisme ne fait que déplacer ce qui s'y trouve — et n'y ramène jamais rien (§9 règle 5).

### 6.2 Le PRD comme source

**Quand un PRD existe dans le dépôt, il est lu et ses features deviennent des blocs en « À faire ».** On n'a pas à retranscrire ce qu'on vient d'écrire.

Le PRD reste **la source d'intention** ; le tableau en est le reflet. On ne modifie pas une feature depuis le tableau : on modifie le PRD, et le tableau suit.

**La convention de rédaction.** Elle est tenue en amont, pas devinée en aval. Un PRD lisible a cette forme :

```
---
id: PRD-004
titre: Export CSV
statut: valide        # draft | valide | en cours | livre | abandonne
valide_le: 2026-08-12 # rempli au passage en « valide »
date: 2026-08-10      # creation — ne change jamais, elle porte l'identite
maj: 2026-08-12       # derniere edition — a rafraichir a chaque retouche
repo: yarma-decks
---

## Features à développer

### F1 — Export d'une sélection (Priorité : P1)
- **User story** : ...
- **Exigences** : FR-001, FR-002 ...
- **Critères d'acceptation** : - [ ] ...
- **Hors scope** : ...
- **[À CLARIFIER]** : ...
```

Le parseur en tire quatre choses, et rien d'autre :

| Ce qu'il lit | Ce qu'il en fait |
|---|---|
| L'**en-tête** — `id`, `titre`, `statut`, `valide_le`, `date`, `maj`, `repo` | reconnaît le fichier comme un PRD vivant, et dit **quand** ses features entrent sur le tableau (voir ci-dessous). Pas d'en-tête, pas de lecture |
| `## Features à développer` | délimite la zone à lire ; le reste du document est ignoré |
| `### F1 — nom court (Priorité : P1)` | un bloc de type *feature*, titré `nom court` |
| La case `[À CLARIFIER]`, si présente | le bloc est marqué **à clarifier** : il est prévu, mais pas prêt à lancer |

**La clé stable est `2026-08-10/PRD-004/F1`** — date du document, identifiant, feature. Ni `F1` seul, ni `PRD-004/F1` ne suffisent : deux PRD du même dépôt ont chacun leur `F1`, et un identifiant peut être réemployé d'une série de documents à l'autre. C'est cette clé qui permet de reformuler le titre d'une feature sans créer de doublon.

**Deux dates, deux rôles.** `date` porte la **création** et entre dans la clé : elle ne se retouche pas, sous peine de dédoubler toutes les cartes du PRD — les anciennes restant en place (règle 7). `maj` porte la **dernière révision** et bouge librement, c'est elle qui dit qu'un document a vécu. La séparation est dans le gabarit lui-même, à l'endroit où quelqu'un serait tenté de mettre `date` à jour.

`maj` sert aussi à l'écran : un bloc peut dire d'où il vient et **de quand** — « feature du PRD-004, révisé le 12 août ». Une carte en « À faire » depuis six semaines dont le PRD a bougé hier ne raconte pas la même chose qu'une carte oubliée.

**Le champ `repo` fait autorité.** Un PRD qui nomme un autre dépôt que celui où il se trouve ne peuple pas le tableau courant. On écrit parfois le PRD d'un projet dans le dépôt d'à côté ; le document dit pour qui il est écrit, et on le croit.

**La priorité est affichée, jamais gérée.** `P1` est lu et montré sur le bloc, parce que c'est une information utile pour choisir quoi lancer. Mais elle ne se modifie pas depuis le tableau et ne trie rien automatiquement : elle appartient au PRD (§12).

**Les critères d'acceptation restent lisibles, pas suivis.** Ils s'affichent dans le bloc, en lecture seule — ils disent *comment on saura que c'est fait*. Les issues, elles, disent *où le travail se passe* et se créent à la main avec leur emplacement. Les deux listes cohabitent sans se confondre : on ne coche pas un critère d'acceptation, on le lit.

**Le statut dit quand lire, jamais quoi afficher.** Un PRD a son propre cycle de vie, le tableau a le sien : l'un est déclaré par son auteur, l'autre est observé dans le dépôt. Le premier ne commande pas le second.

| `statut` | Ce que le tableau en fait |
|---|---|
| `draft` | **un seul bloc, de type exploration** — « cadrage PRD-004 », avec le nombre de features qu'il décrit. Un brouillon bouge encore : le montrer en douze cartes donnerait à des intentions non arrêtées l'autorité d'un plan, et chaque feature retirée laisserait une carte orpheline (règle 7) |
| `validé` | l'exploration **devient** ses features : le bloc de cadrage disparaît, remplacé par autant de blocs en **À faire**. `valide_le` date le basculement |
| `en cours` | rien de plus — le tableau montrait déjà l'avancement réel, il n'avait pas besoin qu'on le lui dise |
| `livré` | rien de plus. Si des features restent ouvertes, le tableau **le signale** au lieu de les fermer : « PRD marqué livré, 3 features encore ouvertes » |
| `abandonné` | plus aucune feature nouvelle. Les cartes existantes **restent**, marquées « PRD abandonné » — le travail déjà fait ne s'efface pas |

C'est `valide_le` qui déclenche la bascule, et c'est voulu : **un PRD validé est une intention arrêtée**, un brouillon est une intention en train de se former. La colonne « À faire » n'accueille que la première (règle 9). Sans validation, pas de feature (§5.1).

`valide_le` sert aussi à lire le temps : *validé il y a six semaines, toujours `0 / 9`* n'est pas la même phrase que *validé hier*. C'est l'information que le tableau ne saurait pas produire tout seul.

**Rien ne se supprime tout seul.** Une feature retirée du PRD, un PRD renommé ou effacé : le bloc **reste**, marqué « plus dans le PRD ». Le tableau ne détruit pas du travail parce qu'un document a changé d'avis — et un bloc entamé porte de l'histoire qui n'est écrite nulle part ailleurs.

**Ce que ça coûte à la doctrine.** Lire un PRD, c'est lire le contenu d'un fichier. La lecture reste **locale, par le daemon** ; seuls les couples `(clé, titre)`, la priorité et le marqueur *à clarifier* partent. Le corps du document ne quitte pas le poste : ni user story, ni exigences, ni critères. Cette exception est inscrite dans la liste de §7.1 — c'est ce qui la rend tenable : on sait exactement ce qu'on a concédé.

**C'est un parseur, pas un modèle.** Le daemon lit le markdown et applique la règle — quelques dizaines de lignes, aucun jeton, aucune latence. Surtout : **même fichier, même résultat**. Un modèle relisant le même PRD reformulerait légèrement d'une fois sur l'autre et fabriquerait des doublons à chaque passage ; c'est pour ça qu'il est écarté (§12).

**Quand rien n'est reconnu, le produit le dit.** Un PRD hors convention ne donne aucune carte — mais l'espace projet affiche qu'il a vu un PRD sans y reconnaître de feature, plutôt que de rester muet. Un silence est indiscernable d'une panne.

### 6.3 Ce qui ferme un travail

Un commit n'est pas lié à un endroit du code, il est lié à un **travail**. Trois issues peuvent vivre dans `web/app/checkout` sans qu'aucun chemin ne dise laquelle un commit vient de régler. Le chemin est une trace, pas une déclaration.

D'où la séparation nette :

| Signal | Ce qu'il prouve | Ce qu'il déclenche |
|---|---|---|
| **Écriture à un emplacement** (activité d'agent) | il se passe quelque chose ici | passage **en cours** |
| **Référence dans le message d'un commit local** | ce travail-ci est fait | passage **terminé** |

**Toute unité suivie porte une référence** — une issue, mais aussi un bloc simple, une feature tirée d'un PRD, une exploration créée toute seule. Sans elle, ces trois-là n'auraient aucun moyen de se fermer : c'est le seul signal qui désigne un travail plutôt qu'un endroit.

La référence est **courte, stable, unique dans le dépôt**. Elle est affichée sur la carte, copiable d'un geste, et c'est ce qu'on donne à l'agent quand on le lance. L'agent la remet dans son message de commit ; au commit suivant, le travail se ferme. Elle s'écrit **`VM-7`**. Le préfixe n'est pas décoratif : les messages de ce dépôt contiennent déjà `feat(#7):` et `Merge pull request #17`, des numéros d'issues GitHub. Une référence en `#7` aurait fermé des cartes au hasard dès le premier jour.

**Le commit local suffit, et c'est un choix.** Le daemon lit le dépôt sur la machine : il voit un commit dès qu'il existe, il ne voit pas une pull request fusionnée chez GitHub tant que personne n'a tiré. Attendre la fusion, ce serait attendre un signal que le produit ne reçoit pas. On ferme donc sur le commit local, en sachant qu'un commit sur une branche abandonnée ferme une carte à tort (§13).

**Trois cas que la règle doit couvrir.**

- *Une référence nomme un travail déjà terminé* — c'est une **reprise** : le travail repart en cours, en version suivante (`v2`, `v3`…). C'est le cas normal d'une itération, et un silence ferait perdre l'information.
- *Le versionnage porte sur l'unité nommée* — l'issue si le bloc est découpé, le bloc sinon — et **seul un commit qui nomme rouvre**. Écrire dans un dossier n'est pas reprendre un travail livré : l'activité entame ce qui n'est pas fini, elle ne ressuscite rien.
- *Les références ne sont jamais recyclées.* Un compteur par dépôt, `VM-1`, `VM-2`, sans réemploi après suppression. Un numéro recyclé rendrait un vieux message de commit capable de fermer une carte neuve, des mois plus tard.

Une référence qui ne correspond à rien dans le dépôt est ignorée sans bruit : les messages de commit contiennent toutes sortes de choses.

Le coût est honnête : **fermer n'est plus tout à fait gratuit**. Mais la saisie ne tombe pas sur l'humain — il donne une référence en lançant son agent, ce qu'il fait déjà quand il décrit la tâche. C'est l'agent qui écrit le message.

**On ne pousse pas une carte dans « Terminé ».** Le geste n'existe pas : pas de glisser-déposer vers la troisième colonne, pas de case à cocher. « Terminé » s'obtient, il ne s'attribue pas — c'est ce qui fait qu'une carte dans cette colonne veut dire quelque chose. Un geste libre, fait de travers un jour de fatigue, casserait la seule affirmation forte du tableau.

Et quand personne n'a rien nommé ? Le travail reste **en cours** — il ne se ferme ni sur un chemin, ni sur un glissement. Le repli est de le faire **vérifier** (§6.4) : le verdict revient avec ses preuves, et on le confirme. Fermer reste possible, mais seulement comme une décision adossée à quelque chose, jamais comme un mouvement de souris.

### 6.4 Faire vérifier une issue

Quand un commit touche l'emplacement d'un travail sans le nommer, la carte propose : **« Vérifier »**. Un clic, et un sous-agent Claude Code va lire le dépôt sur la machine et répondre à une seule question — *ce travail est-il implémenté ?*

Il revient avec un verdict — **implémenté · partiellement · non trouvé** — une confiance, et les chemins qui l'ont convaincu. La carte affiche ce verdict, et c'est **la seule situation où un bouton « Fermer » apparaît** : ailleurs, la troisième colonne ne s'atteint que par un commit qui nomme le travail (§6.3).

**Le verdict propose, il ne ferme jamais.** Un jugement de modèle n'est pas un fait. C'est le même principe que §13 : mieux vaut rater une fermeture que d'en inventer une, et une vérification automatique qui ferme d'autorité serait exactement l'invention qu'on refuse. L'humain reste le seul à fermer — mais il décide en une seconde au lieu d'aller lire le code.

Ce que ça demande, et qui n'existe pas encore :

- **Le daemon devient bidirectionnel.** Il ne fait que pousser aujourd'hui ; il devra écouter une demande de vérification, l'exécuter sur la machine où le dépôt vit, et reposer le résultat. C'est la première fois que Vibe Map **exécute** au lieu d'observer, et c'est un écart assumé avec `PRODUCT.md` (« la v1 observe, elle ne pilote pas ») — cette phrase annonçait la suite, la voici.
- **Une demande sans texte libre.** Elle ne transporte qu'un identifiant d'issue. Le prompt du sous-agent est écrit dans le daemon, jamais reçu du serveur : une ligne écrite côté serveur ne doit pas pouvoir faire exécuter n'importe quoi sur la machine.
- **Un sous-agent en lecture seule.** Il lit, il ne modifie rien, il ne committe rien.
- **Un retour borné.** Verdict, confiance, chemins relatifs, une phrase de justification — jamais d'extrait de code, jamais de contenu de fichier. C'est la ligne la plus exposée de §7.1, et la seule qu'on doit pouvoir couper sans perdre le reste du produit.
- **Un coût visible.** Une vérification consomme des jetons. Ils sont attribués au dépôt comme le reste (le suivi coût/jetons existe déjà), et la vérification reste **sur demande** : rien ne se vérifie tout seul.
- **Une machine choisie.** Depuis que l'identité d'un dépôt vient de son distant (§10.1), un tableau peut correspondre à plusieurs clones. La vérification part sur la machine qui a eu de l'**activité la plus récente** sur ce dépôt — c'est celle qui a le plus de chances d'avoir le travail sur son disque. Si son daemon ne répond pas dans le délai, la demande le dit et propose la machine suivante, plutôt que d'échouer en silence.
- **Des bornes.** Une vérification a un **délai maximal** au-delà duquel elle est abandonnée, une **seule à la fois par dépôt**, et un **plafond de jetons** connu d'avance. Sans ces trois bornes, « un coût visible » n'est pas un garde-fou mais une constatation après coup.

### 6.5 Ce que porte une carte

Une carte dit quatre choses avant qu'on l'ouvre : **quel travail**, **d'où il vient**, **où il en est**, **comment le nommer**.

| Marque | Ce qu'elle dit | Quand elle apparaît |
|---|---|---|
| Titre | le travail | toujours |
| Référence `VM-7` | ce qu'on donne à l'agent, ce qu'un commit doit écrire | toujours |
| Pastille d'**origine** — `PRD` | la carte vient d'un document, pas du clavier ; on la corrige dans le PRD, pas ici (§6.2) | features tirées d'un PRD |
| Type — *feature · correction · technique · exploration* | la nature du travail | toujours |
| Priorité — `P1` | lue dans le PRD, affichée, jamais gérée ici | features tirées d'un PRD |
| `12 / 17` | l'avancement | blocs découpés |
| *à clarifier* | prévu, pas prêt à lancer | si le PRD le signale |

**Chaque marque est un mot, jamais une couleur seule** — doctrine d'accessibilité de `PRODUCT.md`, qui vaut ici comme sur la carte.

**Le risque, nommé.** Une carte peut porter jusqu'à six marques. Le registre visé est Linear — densité maîtrisée, neutres à peine teintés — pas un tableau de bord bavard. Le travail de design consiste donc à **hiérarchiser**, pas à tout afficher au même niveau : le titre et l'avancement se lisent de loin, l'origine et la priorité se lisent quand on s'approche, la référence ne sert qu'au moment de lancer un agent. Une carte qui ressemble à un formulaire aura raté sa cible même si toutes les informations y sont.

### 6.6 Quand il n'y a rien à montrer

Un tableau vide n'est pas un état, c'en est trois. Ils se ressemblent à l'écran et ne veulent pas dire la même chose — les confondre est la faute à éviter.

**1. Le premier jour — un aperçu, pas une page blanche.** Le tableau n'a jamais rien contenu. Il montre alors **à quoi il ressemblera** : des cartes d'exemple en sourdine, non interactives, dans les trois colonnes, avec les marques réelles (`VM-7`, pastille `PRD`, `3 / 9`). On apprend le produit en le regardant plutôt qu'en lisant une notice. Une seule action est offerte — créer un travail — et une phrase dit d'où viendra le reste : un PRD dans le dépôt, ou un agent qui écrit.

La colonne « Terminé » mérite son mot à elle : elle restera vide même si le dépôt a dix ans d'historique, parce que le passé n'est pas rejoué (règle 10). Sans cette phrase, le premier réflexe est de croire à un bug.

**2. Le tableau vidé — tout est fini.** Plus rien en « À faire » ni en « En cours », et des cartes en « Terminé ». C'est un **succès**, et c'est le seul endroit du produit où un clin d'œil est permis : rien à faire, rien à surveiller, on peut fermer l'onglet. Trois bornes pour qu'il ne dérive pas vers l'anti-référence de `PRODUCT.md` : **image fixe**, jamais de boucle ni d'animation d'ambiance ; **coupé sous `prefers-reduced-motion`** comme le reste ; **jamais une alerte**, seulement une récompense. Le ton reste celui de la maison — un sourire, pas une fanfare.

**3. Le tableau muet — rien ne remonte.** Le daemon est arrêté, la machine dort, la dernière écriture date d'hier. À l'écran, c'est exactement le même vide que le cas 2, et c'est là qu'un mot d'esprit deviendrait un mensonge. L'espace affiche donc **depuis quand il n'a rien reçu**, et le dit comme un état, pas comme une erreur : « aucun signal depuis 3 h ». La règle est simple — **le clin d'œil n'est permis que si l'on est sûr d'être dans le cas 2**, c'est-à-dire si le produit a reçu quelque chose récemment. Dans le doute, on dit ce qu'on sait.

Ce troisième cas dépasse les états vides : un tableau plein mais figé ment tout autant. La fraîcheur du signal est donc une information de l'espace entier, pas une décoration de sa page vide.

**4. Un PRD vu, aucune feature reconnue.** Déjà décidé en §6.2 : on le dit, on ne reste pas muet. Même principe que le cas 3 — un silence est indiscernable d'une panne.

## 7. Objectifs

### Fonctionnels

- **F1** — Déclarer un travail en une saisie courte : titre, type, et emplacement (ou découpage en issues).
- **F2** — Découper un bloc en issues à tout moment, y compris après coup — un bloc simple devenu gros se découpe sans se recréer.
- **F3** — Voir une issue passer en **entamée** dès qu'un agent écrit à son emplacement, sans intervention.
- **F4** — Voir un travail passer en **terminé** quand un commit le nomme, sans intervention.
- **F5** — Voir un bloc changer de colonne tout seul quand l'état de ses issues le justifie.
- **F6** — Lire d'un coup d'œil le reste à faire d'un bloc (`12 / 17`) et l'ouvrir pour voir quelles issues restent.
- **F7** — Suivre un travail qui repart (livré, puis repris) sur le même bloc, avec son historique de versions et les commits qui les ont closes.
- **F8** — Ramener une carte fermée à tort vers « En cours ». Le mouvement inverse n'existe pas : on ne pousse jamais une carte dans « Terminé » à la main (§6.3).
- **F9** — Voir le tableau se mettre à jour sur un appareil alors que le travail se fait sur un autre.
- **F10** — Filtrer le tableau par type (feature / correction / technique / exploration), et changer le type d'une carte à tout moment.
- **F11** — Voir apparaître tout seul un bloc *exploration* quand un agent écrit un document de conception, sans l'avoir déclaré ; pouvoir le renommer, le retyper ou le supprimer.
- **F12** — Voir les features d'un PRD présent dans le dépôt apparaître en « À faire » sans les retranscrire, et se mettre à jour quand le PRD change (§6.2).
- **F13** — Lire et copier d'un geste la référence d'un travail (`VM-7`) pour la donner à l'agent qu'on lance.
- **F14** — Demander en un clic la vérification d'une issue par un sous-agent local, et lire son verdict sur la carte avant de décider de fermer (§6.4).
- **F15** — Savoir d'un coup d'œil si ce qu'on lit est frais : depuis quand l'espace n'a rien reçu, et distinguer « rien à faire » de « rien ne remonte » (§6.6).

### Non-fonctionnels

- **NF1** — Une écriture d'agent déplace une carte en « En cours » sur un autre appareil en **moins de 5 secondes** ; un commit la ferme en **moins d'une minute**. Les deux chiffres diffèrent parce que les deux signaux ne sont pas lus au même rythme : les journaux d'agent sont relus toutes les 2 s, les commits demandent une lecture git. Le mécanisme d'ingestion des commits doit donc avoir sa propre cadence — pas celle de la cartographie, qui tourne toutes les 5 minutes (§11).
- **NF2 — ce qui sort est une liste fermée, et elle est publiée.** Voir §7.1. La règle n'est plus « rien de sensible ne sort » : le produit doit aider un *ai-native builder* à suivre ses projets, et cet objectif prime. Ce qui la remplace est plus exigeant que vague : une liste explicite de ce qui part, un plancher de ce qui ne part jamais, et un endroit où l'utilisateur peut lire les deux.
- **NF3** — Registre visuel Linear, aligné sur la carte existante : densité maîtrisée, la couleur ne dit que l'état, jamais la couleur seule. Le type est un libellé, pas une teinte supplémentaire.
- **NF4** — WCAG AA ; mouvement coupé sous `prefers-reduced-motion`.
- **NF5** — Zéro configuration : aucun réglage à poser pour que l'automatisme fonctionne sur un dépôt déjà cartographié.
- **NF6** — Un bloc à 17 issues reste lisible : la colonne montre le bloc et son avancement, jamais 17 cartes.

### 7.1 Ce qui sort de la machine

L'ancienne doctrine — *rien de sensible ne quitte le poste* — a tenu tant que le produit ne faisait qu'observer. Lire un PRD (§6.2) et faire vérifier une issue (§6.4) la percent. Plutôt que de la garder en la contournant, on la remplace.

**Le principe.** Ce qui sort de la machine est une **liste fermée**. On n'y ajoute rien sans le décider ; elle est lisible par l'utilisateur ; et elle est bornée par un plancher qui, lui, ne bouge pas.

**Ce qui peut sortir :**

| Donnée | D'où elle vient | Pourquoi elle sort |
|---|---|---|
| Empreinte, message, branche, date d'un commit | git | dire ce qui est fait, et par quoi |
| Chemins **relatifs** à la racine du dépôt | git, journaux d'agent | situer le travail sans nommer la machine |
| Empreinte du dépôt distant | `git remote get-url origin`, normalisée puis hachée | reconnaître le même dépôt d'une machine à l'autre (§10.1) |
| Nature d'un accès (lecture / écriture), horodatage, session | journaux Claude Code | allumer la carte, entamer une issue |
| Compteurs : jetons, coût, durée | journaux Claude Code | le suivi de coût existant |
| **Titres** de features, leur clé `2026-08-10/PRD-004/F1`, leur priorité, leur marqueur *à clarifier*, le statut du PRD et ses dates (`valide_le`, `maj`) | lecture locale du PRD (§6.2) | peupler « À faire » sans retranscrire, et dire de quand date l'intention. Ni user story, ni exigences, ni critères d'acceptation ne sortent |
| **Verdicts** de vérification : état, confiance, chemins, une phrase | sous-agent local (§6.4) | décider de fermer sans aller lire le code |

**Ce qui ne sort jamais** — le plancher :

- le contenu d'un fichier de code, et tout diff ;
- les prompts et les réponses d'agent ;
- les chemins absolus : la racine d'un dépôt voyage en empreinte, jamais en clair, parce qu'un chemin absolu porte le nom de l'utilisateur du système ;
- les secrets : `.env`, clés, jetons — quelle que soit la fonctionnalité qui les croiserait.

**Le risque résiduel, nommé.** La phrase de justification d'une vérification (§6.4) est écrite par un modèle qui vient de lire le code. Elle est plafonnée et il lui est interdit de citer du code, mais c'est la seule ligne de la liste dont le contenu n'est pas mécaniquement dérivé — c'est là que ça fuirait si ça devait fuir. Un utilisateur qui n'en veut pas doit pouvoir couper la vérification sans perdre le reste.

**Ce que ça oblige ailleurs, et quand.** Deux textes portent aujourd'hui la promesse absolue : `README.md` (l. 12-15, « aucune table n'a de colonne où un contenu de fichier ou un prompt pourrait entrer ») et la §7 du spec du 2026-08-01. Ils restent **exacts tant que l'espace projet n'est pas livré** — aucune table ne reçoit encore de titre de PRD ni de verdict. Les réécrire maintenant leur ferait décrire un produit qui n'existe pas.

Ils doivent donc changer **au moment où le code arrive**, pas avant : c'est une ligne de la liste de livraison, pas une correction à faire tout de suite. Le spec du 2026-08-01 porte déjà l'avertissement en tête de sa §7. Un dépôt dont le README promet plus que le produit ne tient vaut moins qu'un README honnête — mais un README qui promet moins que ce que le produit fait n'est pas mieux.

## 8. Parcours

**A — Je prévois une petite chose.** « Nouveau », type *correction*, titre « Le hero déborde sur mobile », emplacement `web/app/landing/hero` par autocomplétion sur les dossiers connus. Bloc simple, colonne **À faire**, avec sa référence. Je n'y retouche plus : je la passerai à l'agent, son commit fermera la carte.

**B — Je prévois un gros chantier.** Type *feature*, « Refonte du tunnel de commande ». Je le découpe en issues, chacune avec son emplacement. Le bloc affiche `0 / 9` en **À faire**.

**C — Un agent s'y met.** Je lance un agent sur une des issues. Elle passe entamée ; le bloc glisse en **En cours** et affiche `0 / 9`. Depuis mon téléphone, il y est déjà.

**D — Ça avance.** Les issues tombent une à une : `3 / 9`, `6 / 9`. Le bloc reste en **En cours**. J'ouvre le bloc pour voir lesquelles restent — c'est ma to-do list du chantier.

**E — C'est livré.** J'avais lancé l'agent en lui donnant la référence de l'issue. Son commit la porte : l'issue se ferme, et elle seule — les deux autres issues du même dossier ne bougent pas. Quand la dernière tombe, le bloc glisse en **Terminé**, `9 / 9`, avec les commits qui l'ont clos. Je sais que c'est fait, et par quoi.

**F — Je reprends dessus.** Trois semaines plus tard, un agent retouche le tunnel. Le bloc repart en **En cours**, marqué `v2`. Il ne redevient jamais « À faire » : ce qui a été livré une fois reste livré.

**G — L'automatisme s'est trompé.** Une issue a été fermée trop tôt — un commit l'a nommée alors qu'il restait du travail. Je la ramène en **En cours** d'un geste ; le bloc suit. Le prochain commit qui la nomme pourra la refermer — corriger ne verrouille pas.

**I — J'ai oublié la référence.** Le travail est fait, mais aucun commit ne l'a nommé : la carte traîne en **En cours**. Je clique **Vérifier**. Trente secondes plus tard : *implémenté, confiance haute*, avec les trois fichiers qui l'attestent. Je confirme. C'est le seul chemin vers « Terminé » qui ne passe pas par un commit — et il demande une preuve, pas un glissement.

**H — Je n'avais rien prévu, et pourtant.** Je lance un agent pour cadrer une idée. Il écrit `docs/adr/0012-file-attente.md`. Sans que j'aie rien saisi, un bloc *exploration* « ADR 0012 — file d'attente » apparaît en **En cours**. Le bloc porte sa propre référence ; le commit qui range l'ADR la nomme, et le bloc passe en **Terminé**. Le lendemain, je vois dans le tableau que la décision a été prise, et laquelle.

## 9. Règles produit

Le cœur du comportement. Traduction technique à faire dans le spec (§11).

1. **Deux signaux, deux portées.** L'**emplacement** dit qu'il se passe quelque chose — il fait *entamer*. La **référence dans le commit** dit ce qui est fait — elle seule *ferme*. On ne déduit jamais une fin d'un chemin.
2. **Un commit ne ferme que ce qu'il nomme.** Un commit local ferme exactement les travaux que son message désigne par leur référence — un, six, aucun. Un commit qui ne nomme rien ne ferme rien, même s'il touche l'emplacement d'un travail vivant.
3. **Plusieurs travaux peuvent partager un emplacement.** Six issues dans `web/app/checkout` cohabitent sans ambiguïté : un commit dans ce dossier les met en cours, mais ne ferme que celles qu'il nomme. C'est ce que le chemin seul ne saurait pas faire.
4. **L'état d'un bloc découpé ne se saisit pas**, il se déduit de ses issues (§6). On ne déplace pas un bloc découpé à la main : on agit sur ses issues.
5. **L'automatisme n'avance jamais à reculons.** `À faire → En cours`, `→ Terminé`, `Terminé → En cours v+1`. Jamais de retour en « À faire ».
6. **Le manuel sort de « Terminé », il n'y entre pas — et ne verrouille rien.** On peut ramener une carte fermée à tort ; on ne peut pas en pousser une dedans, ni la soustraire à l'automatisme. Le prochain commit qui nomme le travail peut le refermer : aucune carte n'échappe à ce que dit le dépôt.
7. **Rien ne se supprime tout seul.** L'automatisme crée (une exploration, une feature de PRD) et déplace ; il n'efface jamais. Une feature retirée du PRD laisse son bloc en place, marqué comme telle.
8. **Terminé veut dire nommé par un commit local**, quelle que soit la branche. Le daemon voit le disque, pas la forge : attendre la fusion serait attendre un signal qui n'arrive pas.
9. **La colonne « À faire » n'accueille que de l'intention humaine** — une feature écrite dans un PRD, une tâche créée par le PM (§6.1). Le dépôt peut être automatique ; la décision, jamais. Rien n'y entre qu'un humain n'ait écrit quelque part.
10. **Le passé n'est pas rejoué.** Au premier branchement sur un dépôt, l'historique git existant est ignoré : « Terminé » se peuple à partir de ce qui suit.
11. **Le tableau ne réclame rien.** Pas d'échéance, pas de relance, pas de notification. Il est consulté, pas subi.
12. **Le tableau ne crée de lui-même que ce qu'il a lu quelque part** : une feature écrite dans un PRD (§6.2), une exploration constatée quand un document de conception apparaît (§5.1). Il n'invente jamais un travail. Ce qu'il crée seul se renomme, se retype et s'efface.

## 10. Décisions et questions ouvertes

### 10.1 Tranché

- **« Terminé » = un commit local a nommé le travail.** Le premier jet retenait « fusionné dans `main` ». Vérification faite dans le code, c'était intenable : le daemon n'appelle jamais `git fetch`, donc une pull request fusionnée chez GitHub n'existe pas sur le disque tant que personne n'a tiré ; et il ne connaît que la branche courante, pas la branche de référence. Le produit aurait attendu un signal qu'il ne reçoit pas. On ferme donc sur le **commit local**, qui est visible tout de suite et sans configuration. Ce qu'on abandonne en le faisant est écrit en §13 : une branche abandonnée ferme une carte à tort.
- **Le PRD est lu par un parseur, pas par un modèle.** La convention de rédaction est tenue en amont — un marqueur qui déclare le fichier lisible, un identifiant par feature (`F12`). En échange, l'extraction est déterministe : même fichier, même résultat, donc pas de doublon à chaque relecture. Un modèle aurait accepté n'importe quel PRD, au prix de reformulations qui auraient fabriqué des cartes en double.
- **La référence s'écrit `VM-7`.** Le `#7` nu était inutilisable : l'historique de ce dépôt porte déjà `feat(#7):` et `Merge pull request #17`, qui désignent des issues GitHub. Le préfixe évite qu'un message de commit ordinaire ferme une carte par accident.
- **Toute unité suivie porte une référence, pas seulement les issues.** Sans cela, un bloc simple, une feature de PRD et une exploration auto-créée n'avaient aucun chemin vers « Terminé » — trois origines sur quatre produisaient des cartes immortelles.
- **Le commit fait foi — et il ne ferme que ce qu'il nomme.** Un commit n'est pas lié à un emplacement, il est lié à un travail : trois travaux distincts peuvent vivre dans `web/app/checkout`, et un commit n'en règle qu'un. Le chemin ne peut donc pas fermer, il ne peut qu'entamer. La fermeture demande que le commit **désigne** le travail par sa référence (§6.3). Conséquences : la contrainte « une seule tâche vivante par emplacement » disparaît quand même — elle n'existait que pour lever une ambiguïté qui n'a plus lieu d'être — et le départage par profondeur ne joue plus pour la fermeture.
- **Un PRD présent dans le dépôt est lu, et ses features deviennent des blocs.** La lecture est **locale** ; seuls les couples `(identifiant, titre)` sortent de la machine. C'est cette décision qui a fait tomber l'ancienne doctrine de confidentialité, remplacée par la liste fermée de §7.1.
- **`docs/PRD.md` est supprimé.** C'était le PRD de l'app macOS retirée le 2026-08-06 : un document mort, au nom qui promettait le contraire. Il reste dans l'historique git si on le cherche. Le dépôt n'a donc plus de PRD produit global — `PRODUCT.md` porte la personnalité, les specs portent la conception, et chaque PRD couvre un chantier.
- **Les vides sont distingués et dessinés** (§6.6) : un tableau neuf montre un **aperçu** de ce qu'il deviendra ; un tableau vidé parce que tout est fini s'autorise un **clin d'œil** ; un tableau vide parce que **rien ne remonte** le dit franchement. Confondre les deux derniers ferait afficher une plaisanterie pendant une panne.
- **Rien ne se gèle.** Pas d'épinglage, pas de cadenas : aucune carte n'est soustraite à l'automatisme. Un gel serait un réglage, et `PRODUCT.md` pose que le produit ne se configure pas ; il servirait surtout à figer un état que le dépôt contredit, c'est-à-dire à faire mentir le tableau. Si une carte est refermée alors qu'on venait de la rouvrir, c'est qu'un commit l'a nommée : le problème est dans le message de commit, pas dans l'absence de verrou.
- **Le type se change à tout moment, sans effet sur l'état.** Une correction qui s'avère être une feature, une exploration devinée par le tableau qui était en fait un chantier technique : l'étiquette se corrige. Elle ne sert qu'à la lecture — la bloquer aurait forcé à recréer la carte et à perdre son historique. Une feature venue d'un PRD retypée à la main garde son nouveau type : le document dit ce qui est à faire, pas comment on le range.
- **Un seul tableau, et l'origine se lit sur la carte.** Les quatre types cohabitent dans les trois mêmes colonnes ; une vue séparée aurait rendu deux réponses à « où en est ce dépôt ». Ce qui distingue les cartes n'est pas leur emplacement mais une **pastille d'origine** : `PRD` pour une feature tirée d'un document, rien pour une carte saisie à la main. Voir §6.5.
- **L'exploration est une phase, pas une catégorie de document** — et pas une charge de travail. Un PRD non validé est **un** bloc, quel que soit le nombre de features qu'il décrit : un **mémo** qui dit qu'on a réfléchi à quelque chose, compté dans aucun total. À la validation, il **devient** ses features et ne laisse pas de carte derrière lui. **Sans validation, pas de feature.** Ce qui départage les types n'est donc pas le support produit mais ce qui clôt le travail : une décision, ou quelque chose qui tourne.
- **La documentation qui ne décide rien n'entre pas dans le tableau.** README, guides, commentaires d'API : ni décision, ni capacité nouvelle. Le Kanban ne les suit pas.
- **Six comportements jusque-là non définis sont fixés** (§6.2, §6.3) : une référence qui nomme un travail terminé le **rouvre en version suivante** ; le versionnage porte sur l'unité nommée et **seul un commit rouvre**, jamais l'activité ; découper un bloc simple fait **descendre son emplacement** dans sa première issue ; les références `VM-n` ne sont **jamais recyclées** ; un PRD lisible **n'est pas** une exploration ; **rien ne se supprime tout seul**.
- **Le PRD suit un gabarit fixe** (§6.2) : en-tête `id`/`titre`/`statut`/`date`/`repo`, section `## Features à développer`, une feature par `### Fn — nom (Priorité : Pn)`. La clé stable est `2026-08-10/PRD-004/F1` — date et identifiant du document en font partie, sans quoi deux PRD du même dépôt entreraient en collision sur `F1`. Contrepartie : `date` devient immuable — le gabarit porte un champ `maj` distinct pour les révisions.
- **Un dépôt est identifié par son distant, plus par son chemin.** L'empreinte porte désormais sur l'URL du dépôt distant (`git remote get-url origin`, normalisée) et non sur le chemin absolu. Un dossier renommé ou déplacé garde ses cartes ; deux clones de la même origine partagent un seul tableau. C'est la contrepartie du fait que le Kanban est le premier objet du produit qui contient du travail saisi à la main : il ne peut pas disparaître sur un `mv`. Un dépôt sans distant retombe sur l'empreinte du chemin, avec ce que ça implique.
- **On ne pousse pas une carte dans « Terminé ».** Le glisser-déposer vers la troisième colonne est supprimé : un geste libre peut fermer par erreur, et « Terminé » est la seule affirmation forte du tableau. On peut en revanche en **sortir** une carte fermée à tort — l'asymétrie est le principe même du document. Seul un commit qui nomme le travail, ou la confirmation d'un verdict de vérification, y fait entrer une carte.
- **La vérification par sous-agent est dans le périmètre** (§6.4), pas repoussée : c'est elle qui rend vivable le modèle « seul un commit qui nomme ferme », en rattrapant les travaux dont personne n'a passé la référence. Sans elle, supprimer la fermeture manuelle laisserait ces travaux ouverts pour toujours. Elle amène avec elle un daemon bidirectionnel, donc une surface d'exécution à border.
- **Le produit change de logique, et on l'assume.** Vibe Map n'observait que. Il crée désormais de lui-même (§5.1) et se nourrit d'une intention écrite ailleurs (§6.1). C'est un déplacement volontaire : l'espace projet n'est pas la carte, il n'a pas à en avoir la retenue.

Ces décisions ont été prises au fil de la rédaction ; les questions qu'elles ferment portaient les numéros Q1 (ce que veut dire « terminé »), Q2 (ce qui ferme un travail), Q3 (où vivent les explorations), Q5 (le type se change), Q6 (pas de gel), Q7 (les états vides), Q8 (le sort de l'ancien `docs/PRD.md`), Q9 (lire le PRD), Q10 et Q11 (comment le lire), Q12 (la forme de la référence), Q13 (le périmètre de la vérification), Q14 (l'identité d'un dépôt). Elles ne figurent plus en §10.2.

### 10.2 À trancher

Le modèle est arrêté et toutes les questions bloquantes sont tranchées (§10.1). Ce qui suit relève du dessin et peut se régler pendant l'écriture du spec.

| # | Question | Options | Recommandation |
|---|---|---|---|
| Q4 | Quels chemins déclenchent la **création automatique** d'une exploration ? | Proposé : `docs/adr/`, `docs/superpowers/specs/`, `docs/superpowers/plans/`. Un fichier portant l'en-tête PRD en est exclu (§5.1). Plus large (`docs/**`) = plus de bruit ; plus étroit = des décisions manquées. | Commencer étroit sur la liste proposée. Élargir si l'on constate des trous, jamais l'inverse. |

## 11. Impact sur le spec technique

Le [spec du 2026-08-09](superpowers/specs/2026-08-09-espace-projet-kanban-design.md) a été écrit sur un modèle à un seul niveau. Il devra être repris sur ces points :

- **Modèle de données** : `tasks` devient deux niveaux (bloc / issue) ; l'emplacement descend sur l'issue ; le type apparaît ; l'état d'un bloc découpé est dérivé.
- **Fermeture** : le déclencheur n'est plus « un commit touche l'ancre » mais « un commit local nomme la référence ». Le curseur `last_commit_sha` sur `HEAD` du spec convient — c'est le seul point où le spec existant tombe juste.
- **Cadence** : l'ingestion des commits ne peut pas vivre dans la boucle de cartographie (`scan_seconds = 300`, `daemon/src/config.rs`) sans rendre NF1 faux. Elle a besoin de sa propre cadence, entre celle des journaux (2 s) et celle du plan.
- **Routage** : dédoublé. L'**activité** continue de router par emplacement (entame). La **fermeture** ne route plus par emplacement du tout : elle lit les références dans les messages des commits locaux. La contrainte « un seul travail vivant par emplacement » disparaît (index partiel à retirer), puisque plus rien ne dépend de sa levée d'ambiguïté.
- **Références** : nouveau — chaque issue porte un identifiant court, stable et unique par dépôt, exposé dans l'UI et reconnu dans les messages de commit (forme à figer, Q12).
- **Daemon bidirectionnel** : voie de retour serveur → machine, demande sans texte libre, sous-agent en lecture seule, verdict borné à §7.1, plus les trois bornes de §6.4 (délai, concurrence, jetons). C'est un chantier de sécurité autant que de fonctionnalité — il mérite sa propre section de spec, pas un paragraphe. **Dans le périmètre** (Q13).
- **Identité des dépôts** : passer de `sha256(chemin absolu)` (`daemon/src/plan.rs:250`) à l'empreinte du distant touche `plan.rs`, la table `repos` et sa contrainte `unique (machine_id, root_hash)` — qui devient une unicité **par compte**, plus par machine. Trois pièges à traiter dans le spec :
  - **normaliser l'URL avant de la hacher.** `git@github.com:org/repo.git`, `https://github.com/org/repo.git` et `https://github.com/org/repo` désignent le même dépôt et donneraient trois empreintes différentes. Sans normalisation, la décision ne produit rien.
  - **un dépôt sans distant** garde l'empreinte du chemin ; s'il en gagne un plus tard, son identité change — il faut décider si l'ancien tableau suit ou si l'on repart de zéro.
  - **plusieurs clones actifs** alimentent désormais un seul tableau : l'activité de deux machines se mélange sur les mêmes cartes, ce qui est voulu, mais rend la provenance d'un événement moins évidente.
- **Lecture du PRD** : nouveau travail côté daemon — repérer les fichiers dont l'en-tête porte `id` / `statut` / `repo`, y lire la section `## Features à développer`, en extraire pour chaque `### Fn — titre (Priorité : Pn)` le quadruplet `(clé, titre, priorité, à-clarifier)` avec `clé = <date>/<id du PRD>/Fn`, la `date` étant traitée comme immuable. Ne poster que ça. Déterministe, testable sur fixtures ; rattachement d'un bloc existant par clé, jamais par titre.
- **Doctrine** : `README.md` (l. 12-15) porte encore la promesse absolue. À réécrire sur la liste fermée de §7.1 **dans le commit qui livre la première lecture de PRD ou le premier verdict**, jamais avant — c'est un changement de texte public, il doit arriver avec le comportement qu'il décrit. Le spec du 2026-08-01 porte déjà l'avertissement.
- **Création automatique** : le spec ne prévoit que des tâches saisies. Il faut la règle d'apparition d'un bloc *exploration* sur écriture d'un document reconnu (§5.1, Q4) — c'est la seule écriture que le système s'autorise de lui-même.
- **Périmètre exclu** : « pas de sous-tâches » tombe — c'est devenu le cœur du produit. « Pas d'exploration » tombe aussi.

## 12. Hors scope

- Import d'un backlog autre qu'un PRD (`TODO.md`, tracker externe).
- Édition d'une feature depuis le tableau quand elle vient d'un PRD : la source est le document (§6.2).
- Reprise de l'historique git antérieur au branchement (pas de rétro-remplissage).
- Réconciliation d'un historique réécrit (rebase, amend) : les nouvelles empreintes comptent comme de nouveaux commits.
- Le glisser-déposer vers « Terminé », et tout geste qui déclare un travail fini sans preuve.
- L'épinglage ou le gel d'une carte contre l'automatisme.
- Échéances, assignés, étiquettes libres. La **priorité** fait exception : elle est lue dans le PRD et affichée, mais ni modifiable depuis le tableau, ni utilisée pour trier automatiquement (§6.2).
- Niveaux d'imbrication au-delà de deux : une issue ne se découpe pas à son tour.
- Suivi d'une exploration qui ne produit aucun document (§5.1) — sans artefact, rien à ancrer.
- Lecture du contenu d'un document pour en tirer un titre d'exploration : ce titre vient du **nom du fichier**, jamais de ce qu'il contient.
- Extraction des features par un modèle. Un PRD hors convention ne donne pas de cartes — on corrige le PRD, on ne devine pas. Écarté pour le non-déterminisme (§6.2), pas pour le coût.
- Multi-utilisateur, commentaires, mentions.
- Péremption ou archivage automatique des blocs « À faire » anciens.
- Dépendances entre issues (« celle-ci avant celle-là ») et ordonnancement imposé.

## 13. Ce qu'on assume

Points inconfortables, retenus en connaissance de cause.

- **Le découpage reste à la charge de l'humain.** Rien ne devine que « refonte du tunnel » vaut neuf issues. Un bloc mal découpé donne un avancement mensonger.
- **Fermer demande une référence.** Rien ne tombe sur l'humain — il donne `#42` en lançant son agent, l'agent l'écrit dans son commit — mais la chaîne a un maillon de plus, et un maillon peut sauter. La vérification (§6.4) est là pour rattraper les maillons cassés, pas pour dispenser de la référence.
- **Vérifier, c'est exécuter.** Le bouton de §6.4 fait tourner du code sur la machine depuis une action web. Même verrouillé (pas de texte libre, lecture seule, verdict et bornes), c'est une porte qui n'existait pas — et elle est dans le périmètre de la première livraison (Q13), pas repoussée. La section de spec qui la décrit doit être écrite avant celle qui l'utilise.
- **Une branche abandonnée ferme quand même.** Fermer sur le commit local, c'est fermer avant de savoir si le travail survivra : une branche jamais fusionnée, un commit annulé, un `reset` — la carte reste en « Terminé ». C'est le prix de ne dépendre que du disque. Atténuation : le rangement manuel existe, et un solo qui committe abandonne rarement.
- **Un commit non référencé ne ferme rien.** Le tableau montrera des issues en cours qui sont en réalité terminées, tant qu'on n'a pas pris l'habitude de passer la référence à l'agent. C'est le choix inverse de la fermeture au chemin : on préfère **rater une fermeture que d'en inventer une**. Une carte qui traîne se remarque ; une carte fermée à tort passe inaperçue et ment.
- **Un emplacement large ne ferme plus large**, mais il entame large : une issue ancrée à `web/` passera en cours au premier agent qui écrit sous `web/`. Bruit acceptable — « en cours » n'est pas une affirmation forte.
- **On a échangé une promesse simple contre une liste à tenir.** « Ton code ne quitte pas ta machine » se vérifiait d'un coup d'œil au schéma. Une liste fermée se maintient : chaque fonctionnalité future devra dire si elle y ajoute une ligne, et le README devra suivre. C'est plus de travail, et c'est le prix de l'objectif produit.
- **Deux clones ne se distinguent plus.** Identifier un dépôt par son distant est ce qui sauve le tableau d'un `mv`, mais deux copies volontairement séparées — un clone de travail, un clone d'expérimentation — partagent désormais les mêmes cartes. On assume : le besoin est rare, et il se traite par deux blocs plutôt que par deux tableaux.
- **Le PRD devient une pièce du système.** Il ne se rédige plus tout à fait librement : un fichier hors convention ne peuple rien. C'est un contrat de rédaction accepté en échange d'un tableau qui ne fabrique jamais de doublon — et il vaut aussi pour les agents, qui écrivent une bonne part de ces documents.
- **Le tableau démarre vide côté « Terminé »** (règle 10) : le premier jour, l'espace ne prouve rien.
- **`12 / 17` compte des issues, pas de l'effort.** Neuf issues triviales et une énorme donnent `9 / 10` alors qu'il reste l'essentiel. Assumé : pas d'estimation, pas de points.
- **La création automatique fera du bruit.** Un document retouché en passant peut faire naître un bloc qu'on n'avait pas demandé. C'est le prix du « rien à saisir » : on efface plus vite qu'on ne saisit. La liste de chemins reste volontairement étroite (Q4).
- **Le titre d'une exploration auto est le nom de son fichier.** `0012-file-attente.md` donne un titre correct, `notes.md` non. Le contenu n'est jamais lu — c'est la doctrine NF2, pas une limite technique.

## 14. Critères de succès

Le Kanban est réussi si, sur les dépôts de l'utilisateur :

- Après un commit nommant une référence sur une machine, la carte correspondante est en « Terminé » sur un autre appareil en moins d'une minute, sans que personne ait touché au tableau.
- Sur une semaine d'usage réel, les fermetures après vérification restent l'exception : elles mesurent les fois où la référence n'a pas été passée à l'agent, pas une défaillance de l'automatisme.
- En reprenant un dépôt laissé de côté deux semaines, l'utilisateur sait où il en est **et combien il en reste** sans ouvrir un terminal ni relire du code.
- Le nombre de corrections manuelles reste marginal — sinon le routage est mal calibré.
- Un chantier à double chiffre d'issues se lit sans effort : une ligne dans une colonne, le détail à la demande.
- Les décisions de conception écrites par un agent pendant la semaine apparaissent sur le tableau sans que personne les y ait mises, et l'utilisateur en supprime moins d'une sur cinq comme non pertinente. Un document créé hors des outils d'écriture d'agent (à la main, par un script) n'est pas vu — c'est une limite connue, pas un échec.
