# PRD — Espace projet Vibe Map (Kanban lié au git)

> **Statut** : brouillon v7 — modèle figé ; restent le périmètre de §6.4 et des questions de forme (§10.2)
> **Date** : 2026-08-10
> **Produit** : Vibe Map (voir [PRODUCT.md](../PRODUCT.md))
> **Conception technique** : [spec du 2026-08-09](superpowers/specs/2026-08-09-espace-projet-kanban-design.md) — **désormais en retard sur ce PRD**, voir §11
> **À ne pas confondre avec** : [docs/PRD.md](PRD.md), archive de l'app macOS retirée le 2026-08-06

---

## 1. Problème

Vibe Map répond aujourd'hui à « **où ça en est maintenant** » : quels agents tournent, sur quels dossiers, pour combien de jetons. Il ne répond pas à « **où on en est tout court** » : ce qui est prévu, ce qui est en train de se faire, ce qui est livré.

Pour un développeur solo qui lance plusieurs agents en parallèle, cette seconde question est répartie entre un `TODO.md` qui se périme, un `git log` qu'il faut lire, et sa mémoire. Rien ne relie l'intention (« refaire la landing ») au fait (« commit `a3f9` a touché `web/app/landing` »).

Le coût réel : après quelques jours d'absence sur un dépôt, il faut relire du code pour savoir si une fonctionnalité est finie — et, quand elle est longue, pour savoir **combien il en reste**.

## 2. Promesse

Un tableau par dépôt qui **se tient à jour tout seul**. On y écrit une fois ce qu'on veut faire ; l'avancement se déduit de ce que les agents écrivent et de ce qui part en production.

Un chantier long n'est pas une case opaque : on l'ouvre et on voit ce qu'il reste.

Le tableau **reflète** le code, il ne le pilote pas. C'est un miroir git, pas un gestionnaire de tickets : la vérité est dans le dépôt — le PRD pour ce qui est prévu, la fusion dans `main` pour ce qui est fait.

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

## 5. Les types de travail

Quatre types. Les trois premiers se distinguent par une seule question — **est-ce que ça existe déjà en production, et est-ce que c'est cassé ?** Le quatrième se distingue autrement : il ne produit pas du code, il produit un **document**.

| Type | Définition | Sortie | Existe en prod | Est cassé |
|---|---|---|---|---|
| **Feature** | à développer, n'existe pas encore | du code | non | — |
| **Correction** | existe et ne marche pas comme prévu | du code | oui | oui |
| **Technique** | existe et marche, mais coûte : refactorisation, montée de version, CI, performance | du code | oui | non |
| **Exploration** | comprendre, décider, cadrer : PRD, ADR, spec de conception, plan | un document | — | — |

Le type ne change pas la mécanique — les quatre traversent les mêmes colonnes, avec les mêmes règles. Il change **la lecture** : dix corrections et deux features ne racontent pas la même semaine que l'inverse. Le type est porté par un libellé, jamais par la seule couleur.

### 5.1 L'exploration, seul type qui se découvre

Les trois premiers types se **déclarent** : on sait ce qu'on veut avant de le faire. L'exploration se **découvre** : on ne prévoit pas « j'écrirai un ADR sur la file d'attente », on lance un agent et l'ADR apparaît.

C'est donc le seul type qui peut se **créer tout seul**. Quand un agent écrit un document sous un emplacement de documentation reconnu (`docs/adr/`, `docs/superpowers/specs/`, `docs/superpowers/plans/`, `docs/PRD*.md`) et qu'aucun bloc ne couvre ce fichier, un bloc *exploration* apparaît en **En cours**, ancré à ce document, titré d'après lui. On peut le renommer, le retyper ou le supprimer ; on n'a pas eu à le saisir.

Rien de nouveau à construire côté machine pour ça : le daemon remonte déjà chaque écriture de fichier avec son chemin relatif. Un document est un fichier comme un autre.

**Ce qui reste hors du tableau** : une exploration qui ne produit rien sur le disque — une conversation, une lecture, un comparatif resté dans une session. Sans artefact, rien à ancrer et rien à fermer. La règle est nette : **pas de document, pas de carte**.

**Conséquence sur le cycle.** Une exploration entre presque toujours directement en **En cours** — elle est constatée pendant qu'elle a lieu. La colonne « À faire » lui reste ouverte (on peut décider à l'avance qu'un ADR est nécessaire), mais elle y passe rarement. Elle est **terminée** quand son document arrive en production, au même sens que les autres (§10.1) — pour un document, cela veut dire fusionné dans `main`, donc opposable.

L'exploration est aussi le type qui **repart** le plus souvent : un ADR remplacé, un spec révisé. Le versionnage `v2` (§9 règle 5) y est la norme, pas l'exception.

## 6. Colonnes et avancement

Trois colonnes : **À faire · En cours · Terminé**.

**Bloc simple** — se comporte comme une carte unique : `À faire` à la création, `En cours` dès qu'un agent écrit à son emplacement, `Terminé` quand son travail est en production.

**Bloc découpé** — son état ne se saisit pas, il **se déduit** de ses issues :

| État des issues | Colonne du bloc |
|---|---|
| toutes à faire | À faire |
| au moins une entamée ou terminée, pas toutes terminées | En cours |
| toutes terminées et en production | Terminé |

L'avancement se lit sur le bloc : **`12 / 17`** — nombre d'issues terminées sur le total. Le reste à faire s'ouvre dans le bloc, sans quitter le tableau. C'est ça, la to-do list : ce qui n'est pas encore terminé à l'intérieur d'un bloc en cours.

L'étendue d'un chantier se mesure donc en **issues restantes**, pas en commits. Les commits sont la preuve, pas l'unité.

« Terminé » veut dire **fusionné dans `main`** (§10.1). Un commit sur une branche de travail fait entamer, jamais terminer : tant que la pull request n'est pas passée, l'issue reste en cours même si le code existe.

### 6.1 D'où viennent les blocs

Trois origines, dont une seule passe par le clavier.

| Origine | Ce qui la déclenche | Type produit | Arrive en |
|---|---|---|---|
| **Un PRD** | le dépôt contient un PRD ; ses features en sont tirées automatiquement | feature | À faire |
| **Le PM** | une tâche saisie directement — une correction constatée, un chantier technique décidé | correction, technique, feature | À faire |
| **Un agent** | un document de conception écrit là où il n'y en avait pas (§5.1) | exploration | En cours |

La colonne **À faire** reste la colonne de l'**intention** : tout ce qui s'y trouve a été décidé par un humain, que ce soit écrit dans un PRD ou saisi à la main. L'automatisme ne fait que déplacer ce qui s'y trouve — et n'y ramène jamais rien (§9 règle 5).

### 6.2 Le PRD comme source

**Quand un PRD existe dans le dépôt, il est lu et ses features deviennent des blocs en « À faire ».** On n'a pas à retranscrire ce qu'on vient d'écrire.

Le PRD reste **la source d'intention** ; le tableau en est le reflet. On ne modifie pas une feature depuis le tableau : on modifie le PRD, et le tableau suit. Ce qui est ajouté au PRD apparaît ; ce qui en est retiré ne fait pas disparaître un bloc déjà entamé — l'histoire ne s'efface pas parce qu'un document a changé d'avis.

**Ce que ça coûte à la doctrine.** Lire un PRD, c'est lire le contenu d'un fichier. La lecture reste **locale, par le daemon** ; seuls les **titres** extraits partent. Le corps du document ne quitte pas le poste. Un titre de feature est écrit pour être lu ; un diff, non. Cette exception est inscrite dans la liste de §7.1 — c'est ce qui la rend tenable : on sait exactement ce qu'on a concédé.

**Ce que ça demande au PRD.** Il faut une convention pour savoir ce qui est une feature : à défaut d'autre décision, les entrées d'une liste sous une section reconnue (« Objectifs fonctionnels », « Features »). C'est ce que fait déjà ce document même — voir §7. La convention exacte reste à figer (§10.2, Q10).

### 6.3 Ce qui ferme une issue

Un commit n'est pas lié à un endroit du code, il est lié à un **travail**. Trois issues peuvent vivre dans `web/app/checkout` sans qu'aucun chemin ne dise laquelle un commit vient de régler. Le chemin est une trace, pas une déclaration.

D'où la séparation nette :

| Signal | Ce qu'il prouve | Ce qu'il déclenche |
|---|---|---|
| **Écriture à un emplacement** (activité d'agent) | il se passe quelque chose ici | l'issue passe **en cours** |
| **Référence dans un message de commit**, une fois fusionné dans `main` | ce travail-ci est fait | l'issue passe **terminée** |

Chaque issue porte donc une **référence courte, stable et visible** — de la forme `#7`, unique dans le dépôt. Elle est affichée sur la carte, copiable d'un geste, et c'est ce qu'on donne à l'agent quand on le lance. L'agent la remet dans son message de commit ; à la fusion, l'issue se ferme.

Le coût est honnête : **fermer n'est plus tout à fait gratuit**. Mais la saisie ne tombe pas sur l'humain — il donne une référence en lançant son agent, ce qu'il fait déjà quand il décrit la tâche. C'est l'agent qui écrit le message.

Et quand personne n'a rien nommé ? L'issue reste **en cours**. Elle ne se ferme pas toute seule sur un chemin, et elle ne se ferme pas à tort. Deux replis, dans cet ordre : on la fait **vérifier** (§6.4), ou on la ferme à la main d'un clic.

### 6.4 Faire vérifier une issue

Quand une fusion touche l'emplacement d'une issue sans la nommer, la carte propose : **« Vérifier »**. Un clic, et un sous-agent Claude Code va lire le dépôt sur la machine et répondre à une seule question — *ce travail est-il implémenté ?*

Il revient avec un verdict — **implémenté · partiellement · non trouvé** — une confiance, et les chemins qui l'ont convaincu. La carte affiche ce verdict à côté du bouton **Fermer**.

**Le verdict propose, il ne ferme jamais.** Un jugement de modèle n'est pas un fait. C'est le même principe que §13 : mieux vaut rater une fermeture que d'en inventer une, et une vérification automatique qui ferme d'autorité serait exactement l'invention qu'on refuse. L'humain reste le seul à fermer — mais il décide en une seconde au lieu d'aller lire le code.

Ce que ça demande, et qui n'existe pas encore :

- **Le daemon devient bidirectionnel.** Il ne fait que pousser aujourd'hui ; il devra écouter une demande de vérification, l'exécuter sur la machine où le dépôt vit, et reposer le résultat. C'est la première fois que Vibe Map **exécute** au lieu d'observer, et c'est un écart assumé avec `PRODUCT.md` (« la v1 observe, elle ne pilote pas ») — cette phrase annonçait la suite, la voici.
- **Une demande sans texte libre.** Elle ne transporte qu'un identifiant d'issue. Le prompt du sous-agent est écrit dans le daemon, jamais reçu du serveur : une ligne écrite côté serveur ne doit pas pouvoir faire exécuter n'importe quoi sur la machine.
- **Un sous-agent en lecture seule.** Il lit, il ne modifie rien, il ne committe rien.
- **Un retour borné.** Verdict, confiance, chemins relatifs, une phrase de justification — jamais d'extrait de code, jamais de contenu de fichier. C'est la ligne la plus exposée de §7.1, et la seule qu'on doit pouvoir couper sans perdre le reste du produit.
- **Un coût visible.** Une vérification consomme des jetons. Ils sont attribués au dépôt comme le reste (le suivi coût/jetons existe déjà), et la vérification reste **sur demande** : rien ne se vérifie tout seul.
- **Une machine choisie.** Un dépôt peut vivre sur plusieurs machines ; la vérification part sur celle qui a eu de l'activité en dernier sur ce dépôt. Si aucune n'est joignable, la demande attend et le dit.

## 7. Objectifs

### Fonctionnels

- **F1** — Déclarer un travail en une saisie courte : titre, type, et emplacement (ou découpage en issues).
- **F2** — Découper un bloc en issues à tout moment, y compris après coup — un bloc simple devenu gros se découpe sans se recréer.
- **F3** — Voir une issue passer en **entamée** dès qu'un agent écrit à son emplacement, sans intervention.
- **F4** — Voir une issue passer en **terminée** quand le travail correspondant est en production, sans intervention.
- **F5** — Voir un bloc changer de colonne tout seul quand l'état de ses issues le justifie.
- **F6** — Lire d'un coup d'œil le reste à faire d'un bloc (`12 / 17`) et l'ouvrir pour voir quelles issues restent.
- **F7** — Suivre un travail qui repart (livré, puis repris) sur le même bloc, avec son historique de versions et les commits qui les ont closes.
- **F8** — Corriger le tableau à la main quand l'automatisme s'est trompé, dans n'importe quel sens.
- **F9** — Voir le tableau se mettre à jour sur un appareil alors que le travail se fait sur un autre.
- **F10** — Filtrer le tableau par type (feature / correction / technique / exploration).
- **F11** — Voir apparaître tout seul un bloc *exploration* quand un agent écrit un document de conception, sans l'avoir déclaré ; pouvoir le renommer, le retyper ou le supprimer.
- **F12** — Voir les features d'un PRD présent dans le dépôt apparaître en « À faire » sans les retranscrire, et se mettre à jour quand le PRD change (§6.2).
- **F13** — Lire et copier d'un geste la référence courte d'une issue (`#7`) pour la donner à l'agent qu'on lance ; fermer une issue à la main quand aucun commit ne l'a nommée.
- **F14** — Demander en un clic la vérification d'une issue par un sous-agent local, et lire son verdict sur la carte avant de décider de fermer (§6.4).

### Non-fonctionnels

- **NF1** — Un passage en production sur une machine déplace le bloc sur un autre appareil en **moins de 5 secondes**, sans rechargement ni intervention.
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
| Nature d'un accès (lecture / écriture), horodatage, session | journaux Claude Code | allumer la carte, entamer une issue |
| Compteurs : jetons, coût, durée | journaux Claude Code | le suivi de coût existant |
| **Titres** extraits d'un PRD | lecture locale du PRD (§6.2) | peupler « À faire » sans retranscrire |
| **Verdicts** de vérification : état, confiance, chemins, une phrase | sous-agent local (§6.4) | décider de fermer sans aller lire le code |

**Ce qui ne sort jamais** — le plancher :

- le contenu d'un fichier de code, et tout diff ;
- les prompts et les réponses d'agent ;
- les chemins absolus : la racine d'un dépôt voyage en empreinte, jamais en clair, parce qu'un chemin absolu porte le nom de l'utilisateur du système ;
- les secrets : `.env`, clés, jetons — quelle que soit la fonctionnalité qui les croiserait.

**Le risque résiduel, nommé.** La phrase de justification d'une vérification (§6.4) est écrite par un modèle qui vient de lire le code. Elle est plafonnée et il lui est interdit de citer du code, mais c'est la seule ligne de la liste dont le contenu n'est pas mécaniquement dérivé — c'est là que ça fuirait si ça devait fuir. Un utilisateur qui n'en veut pas doit pouvoir couper la vérification sans perdre le reste.

**Ce que ça oblige ailleurs.** `README.md` (l. 12-15) promet aujourd'hui qu'« aucune table n'a de colonne où un contenu de fichier ou un prompt pourrait entrer ». Le plancher ci-dessus le tient encore pour les *prompts* et le *code* — mais les titres de PRD et les verdicts sont du texte rédigé, et la promesse doit être réécrite en conséquence. Un dépôt dont le README promet plus que le produit ne tient vaut moins qu'un README honnête.

## 8. Parcours

**A — Je prévois une petite chose.** « Nouveau », type *correction*, titre « Le hero déborde sur mobile », emplacement `web/app/landing/hero` par autocomplétion sur les dossiers connus. Bloc simple, colonne **À faire**. Je n'y retouche plus.

**B — Je prévois un gros chantier.** Type *feature*, « Refonte du tunnel de commande ». Je le découpe en issues, chacune avec son emplacement. Le bloc affiche `0 / 9` en **À faire**.

**C — Un agent s'y met.** Je lance un agent sur une des issues. Elle passe entamée ; le bloc glisse en **En cours** et affiche `0 / 9`. Depuis mon téléphone, il y est déjà.

**D — Ça avance.** Les issues tombent une à une : `3 / 9`, `6 / 9`. Le bloc reste en **En cours**. J'ouvre le bloc pour voir lesquelles restent — c'est ma to-do list du chantier.

**E — C'est livré.** J'avais lancé l'agent en lui donnant la référence `#42`. Ses commits la portent. La pull request est fusionnée dans `main` : `#42` se ferme, et elle seule — les deux autres issues du même dossier ne bougent pas. Quand la dernière tombe, le bloc glisse en **Terminé**, `9 / 9`, avec les commits qui l'ont clos. Je sais que c'est fait, et par quoi.

**F — Je reprends dessus.** Trois semaines plus tard, un agent retouche le tunnel. Le bloc repart en **En cours**, marqué `v2`. Il ne redevient jamais « À faire » : ce qui a été livré une fois reste livré.

**G — L'automatisme s'est trompé.** Une issue a été fermée trop tôt. Je la rouvre à la main ; le bloc revient en **En cours**. La prochaine mise en production pourra le re-fermer — le manuel corrige, il ne verrouille pas.

**H — Je n'avais rien prévu, et pourtant.** Je lance un agent pour cadrer une idée. Il écrit `docs/adr/0012-file-attente.md`. Sans que j'aie rien saisi, un bloc *exploration* « ADR 0012 — file d'attente » apparaît en **En cours**. Quand le document arrive sur la branche de référence, le bloc passe en **Terminé**. Le lendemain, je vois dans le tableau que la décision a été prise, et laquelle.

## 9. Règles produit

Le cœur du comportement. Traduction technique à faire dans le spec (§11).

1. **Deux signaux, deux portées.** L'**emplacement** dit qu'il se passe quelque chose — il fait *entamer*. La **référence dans le commit** dit ce qui est fait — elle seule *ferme*. On ne déduit jamais une fin d'un chemin.
2. **Un commit ne ferme que ce qu'il nomme.** Une fusion dans `main` ferme exactement les issues que ses messages de commit désignent par leur référence — une, six, aucune. Un commit qui ne nomme rien ne ferme rien, même s'il touche l'emplacement d'une issue vivante.
3. **Plusieurs travaux peuvent partager un emplacement.** Six issues dans `web/app/checkout` cohabitent sans ambiguïté : un commit dans ce dossier les met en cours, mais ne ferme que celles qu'il nomme. C'est ce que le chemin seul ne saurait pas faire.
4. **L'état d'un bloc découpé ne se saisit pas**, il se déduit de ses issues (§6). On ne déplace pas un bloc découpé à la main : on agit sur ses issues.
5. **L'automatisme n'avance jamais à reculons.** `À faire → En cours`, `→ Terminé`, `Terminé → En cours v+1`. Jamais de retour en « À faire ».
6. **Le manuel corrige, il ne verrouille pas.** Une correction manuelle peut être ré-avancée par l'automatisme.
7. **Terminé veut dire fusionné dans `main`**, pas « committé ». Un commit sur une branche de travail fait avancer, il ne termine pas.
8. **La colonne « À faire » n'accueille que de l'intention humaine** — une feature décidée par un PRD, une tâche créée par le PM (§6.1). Aucun automatisme n'y dépose quoi que ce soit.
9. **Le passé n'est pas rejoué.** Au premier branchement sur un dépôt, l'historique git existant est ignoré : « Terminé » se peuple à partir de ce qui suit.
10. **Le tableau ne réclame rien.** Pas d'échéance, pas de relance, pas de notification. Il est consulté, pas subi.
11. **Le tableau ne crée de lui-même qu'une seule chose : une exploration**, et seulement quand un document de conception apparaît là où il n'y en avait pas (§5.1). Tout le reste se déclare. Ce qu'il crée seul, on peut toujours le renommer, le retyper ou l'effacer.

## 10. Décisions et questions ouvertes

### 10.1 Tranché

- **« En production » = fusion dans `main`.** C'est le seul signal que le daemon peut lire sans configuration, et il correspond au flux par pull request déjà utilisé sur ce dépôt. Un commit sur une branche de travail fait **avancer** (l'activité entame), il ne **termine** pas. Conséquence : le daemon ne peut plus se contenter de suivre `HEAD` — il doit distinguer la branche de travail de la branche de référence (§11).
- **Le commit fait foi — et il ne ferme que ce qu'il nomme.** Un commit n'est pas lié à un emplacement, il est lié à un travail : trois travaux distincts peuvent vivre dans `web/app/checkout`, et un commit n'en règle qu'un. Le chemin ne peut donc pas fermer, il ne peut qu'entamer. La fermeture demande que le commit **désigne** l'issue par sa référence (§6.3). Conséquences : la contrainte « une seule tâche vivante par emplacement » disparaît quand même (elle n'existait que pour lever une ambiguïté qui n'a plus lieu d'être), le départage par profondeur aussi pour la fermeture — mais chaque issue a besoin d'une **référence courte et visible**.
- **Un PRD présent dans le dépôt est lu, et ses features deviennent des blocs.** NF2 est précisée plutôt qu'abandonnée : la lecture est **locale**, seuls les titres extraits sortent de la machine (§6.2).
- **Le produit change de logique, et on l'assume.** Vibe Map n'observait que. Il crée désormais de lui-même (§5.1) et se nourrit d'une intention écrite ailleurs (§6.1). C'est un déplacement volontaire : l'espace projet n'est pas la carte, il n'a pas à en avoir la retenue.

### 10.2 À trancher

Le modèle est arrêté. Q13 décide du périmètre à livrer ; Q12, Q10 et Q11 sont à figer avant d'écrire le code ; le reste est du dessin.

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q12** | Quelle **forme** prend la référence d'une issue ? | `#7`, court et familier, unique par dépôt — ou préfixé (`VM-7`), plus verbeux mais lisible hors contexte. | `#7`. Le dépôt est déjà le contexte. |
| **Q13** | La vérification par sous-agent (§6.4) est-elle **dans ce périmètre**, ou une suite ? | (a) dedans — c'est ce qui rend le modèle « le commit nomme » vivable — (b) après : livrer d'abord la fermeture manuelle, ajouter la vérification une fois le tableau utilisé | (b) : le daemon bidirectionnel est un chantier à lui seul, et on saura mieux après une semaine d'usage combien d'issues restent orphelines. Le PRD la décrit pour qu'elle soit conçue juste, pas pour qu'elle soit livrée d'abord. |
| **Q10** | À quoi reconnaît-on une **feature dans un PRD** ? | (a) les entrées d'une liste sous une section reconnue (« Objectifs fonctionnels », « Features ») — (b) un balisage dédié dans le document — (c) tout titre de niveau donné | (a) : c'est la forme que prennent déjà les PRD de ce dépôt (§7), et elle ne demande rien à l'auteur. |
| **Q11** | Que se passe-t-il quand une feature est **renommée** dans le PRD ? | (a) le bloc est renommé si on sait le rattacher — (b) un nouveau bloc apparaît et l'ancien reste | Sans identifiant stable dans le PRD, (b) est ce qui arrive par défaut. Un identifiant court par feature (`F7`) rendrait (a) possible — ce document en a déjà. |
| **Q3** | Les explorations vivent-elles sur **le même tableau** que le reste ? | (a) même tableau, 4ᵉ type, isolable par le filtre F10 — (b) vue séparée à deux états (en cours / terminé) | (a) : une seule réponse à « où en est ce dépôt », zéro écran de plus, et le filtre donne la vue dédiée gratuitement. À basculer en (b) seulement si le volume d'explorations noie le tableau. |
| Q4 | Quels chemins déclenchent la **création automatique** d'une exploration ? | Proposé : `docs/adr/`, `docs/superpowers/specs/`, `docs/superpowers/plans/`, `docs/PRD*.md`. Plus large (`docs/**`) = plus de bruit ; plus étroit = des décisions manquées. | Commencer étroit sur la liste proposée. Élargir si l'on constate des trous, jamais l'inverse. |
| Q5 | Le **type** est-il modifiable après création ? | Une correction qui s'avère être une feature, ça arrive. | Oui, sans effet sur l'état. |
| Q6 | Faut-il **geler** un bloc contre l'automatisme ? | (a) non — règle 6 telle quelle — (b) oui, épinglage | (a) tant que la règle 6 ne fait pas mal. |
| Q7 | Que montre l'espace **avant qu'il y ait quoi que ce soit** ? | État vide du tableau, et colonne « Terminé » vide au premier jour (règle 9). | À dessiner. |
| Q8 | `docs/PRD.md` reste-t-il l'archive macOS ? | Vibe Map n'a pas de PRD produit global, seulement `PRODUCT.md` et des specs. | Laisser l'archive, ce PRD couvre le Kanban seul. |

## 11. Impact sur le spec technique

Le [spec du 2026-08-09](superpowers/specs/2026-08-09-espace-projet-kanban-design.md) a été écrit sur un modèle à un seul niveau. Il devra être repris sur ces points :

- **Modèle de données** : `tasks` devient deux niveaux (bloc / issue) ; l'emplacement descend sur l'issue ; le type apparaît ; l'état d'un bloc découpé est dérivé.
- **Fermeture** : le déclencheur n'est plus « un commit touche l'ancre » mais « fusionné dans `main` » (§10.1) — le daemon doit observer les fusions sur la branche de référence, et non plus seulement suivre `HEAD`.
- **Routage** : dédoublé. L'**activité** continue de router par emplacement (entame). La **fermeture** ne route plus par emplacement du tout : elle lit les références dans les messages des commits fusionnés. La contrainte « un seul travail vivant par emplacement » disparaît (index partiel à retirer), puisque plus rien ne dépend de sa levée d'ambiguïté.
- **Références** : nouveau — chaque issue porte un identifiant court, stable et unique par dépôt, exposé dans l'UI et reconnu dans les messages de commit (forme à figer, Q12).
- **Daemon bidirectionnel** (si Q13 = dedans) : voie de retour serveur → machine, demande sans texte libre, sous-agent en lecture seule, verdict borné à NF2. C'est un chantier de sécurité autant que de fonctionnalité — il mérite sa propre section de spec, pas un paragraphe.
- **Lecture du PRD** : nouveau travail côté daemon — repérer le PRD, en extraire les titres de features localement, ne poster que ces titres (§6.2, Q10, Q11).
- **Doctrine** : le spec du 2026-08-01 (§7) et `README.md` (l. 12-15) portent l'ancienne promesse. Les deux sont à reprendre sur la liste fermée de §7.1 — c'est un changement de texte public, pas un détail interne.
- **Création automatique** : le spec ne prévoit que des tâches saisies. Il faut la règle d'apparition d'un bloc *exploration* sur écriture d'un document reconnu (§5.1, Q4) — c'est la seule écriture que le système s'autorise de lui-même.
- **Périmètre exclu** : « pas de sous-tâches » tombe — c'est devenu le cœur du produit. « Pas d'exploration » tombe aussi.

## 12. Hors scope

- Import d'un backlog autre qu'un PRD (`TODO.md`, tracker externe).
- Édition d'une feature depuis le tableau quand elle vient d'un PRD : la source est le document (§6.2).
- Reprise de l'historique git antérieur au branchement (pas de rétro-remplissage).
- Réconciliation d'un historique réécrit (rebase, amend) : les nouvelles empreintes comptent comme de nouveaux commits.
- Échéances, priorités, assignés, étiquettes libres.
- Niveaux d'imbrication au-delà de deux : une issue ne se découpe pas à son tour.
- Suivi d'une exploration qui ne produit aucun document (§5.1) — sans artefact, rien à ancrer.
- Lecture du contenu des documents pour en tirer un titre intelligent : le titre auto vient du **nom du fichier**, pas de ce qu'il y a dedans (NF2).
- Multi-utilisateur, commentaires, mentions.
- Péremption ou archivage automatique des blocs « À faire » anciens.
- Dépendances entre issues (« celle-ci avant celle-là ») et ordonnancement imposé.

## 13. Ce qu'on assume

Points inconfortables, retenus en connaissance de cause.

- **Le découpage reste à la charge de l'humain.** Rien ne devine que « refonte du tunnel » vaut neuf issues. Un bloc mal découpé donne un avancement mensonger.
- **Fermer demande une référence.** Rien ne tombe sur l'humain — il donne `#42` en lançant son agent, l'agent l'écrit dans son commit — mais la chaîne a un maillon de plus, et un maillon peut sauter. La vérification (§6.4) est là pour rattraper les maillons cassés, pas pour dispenser de la référence.
- **Vérifier, c'est exécuter.** Le bouton de §6.4 fait tourner du code sur la machine depuis une action web. Même verrouillé (pas de texte libre, lecture seule, verdict borné), c'est une porte qui n'existait pas. On l'ouvre en connaissance de cause, et pas avant d'avoir écrit comment elle se referme.
- **Un commit non référencé ne ferme rien.** Le tableau montrera des issues en cours qui sont en réalité terminées, tant qu'on n'a pas pris l'habitude de passer la référence à l'agent. C'est le choix inverse de la fermeture au chemin : on préfère **rater une fermeture que d'en inventer une**. Une carte qui traîne se remarque ; une carte fermée à tort passe inaperçue et ment.
- **Un emplacement large ne ferme plus large**, mais il entame large : une issue ancrée à `web/` passera en cours au premier agent qui écrit sous `web/`. Bruit acceptable — « en cours » n'est pas une affirmation forte.
- **On a échangé une promesse simple contre une liste à tenir.** « Ton code ne quitte pas ta machine » se vérifiait d'un coup d'œil au schéma. Une liste fermée se maintient : chaque fonctionnalité future devra dire si elle y ajoute une ligne, et le README devra suivre. C'est plus de travail, et c'est le prix de l'objectif produit.
- **Le PRD devient une pièce du système.** Un PRD mal structuré donne un tableau mal peuplé, et le renommer une feature peut créer un doublon (Q11).
- **Le tableau démarre vide côté « Terminé »** (règle 9) : le premier jour, l'espace ne prouve rien.
- **`12 / 17` compte des issues, pas de l'effort.** Neuf issues triviales et une énorme donnent `9 / 10` alors qu'il reste l'essentiel. Assumé : pas d'estimation, pas de points.
- **La création automatique fera du bruit.** Un document retouché en passant peut faire naître un bloc qu'on n'avait pas demandé. C'est le prix du « rien à saisir » : on efface plus vite qu'on ne saisit. La liste de chemins reste volontairement étroite (Q4).
- **Le titre d'une exploration auto est le nom de son fichier.** `0012-file-attente.md` donne un titre correct, `notes.md` non. Le contenu n'est jamais lu — c'est la doctrine NF2, pas une limite technique.

## 14. Critères de succès

Le Kanban est réussi si, sur les dépôts de l'utilisateur :

- Après une mise en production sur une machine, le bloc correspondant est en « Terminé » sur un autre appareil en moins de 5 secondes, sans que personne ait touché au tableau.
- Sur une semaine d'usage réel, **aucune** intervention manuelle n'a été nécessaire pour faire avancer un travail que l'automatisme aurait dû avancer.
- En reprenant un dépôt laissé de côté deux semaines, l'utilisateur sait où il en est **et combien il en reste** sans ouvrir un terminal ni relire du code.
- Le nombre de corrections manuelles reste marginal — sinon le routage est mal calibré.
- Un chantier à double chiffre d'issues se lit sans effort : une ligne dans une colonne, le détail à la demande.
- Les décisions de conception prises pendant la semaine sont **toutes** sur le tableau sans que personne les y ait mises, et l'utilisateur en supprime moins d'une sur cinq comme non pertinente.
