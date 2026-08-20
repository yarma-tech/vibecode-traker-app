---
id: PRD-002
titre: App de bureau
statut: draft
valide_le:
date: 2026-08-19
maj: 2026-08-20
repo: vibecode-traker-app
---

# PRD-002 — App de bureau

## Contexte

Vibe Map tient en trois morceaux : un lecteur de dossiers sur le poste (`daemon/`), un schéma Supabase, une application web. Le premier est un binaire de terminal, la seconde une page dans un navigateur, et rien ne les relie sinon un code court à recopier à la main.

Ce PRD ne change ni la carte ni ce qu'elle mesure. Il change **la manière d'y entrer** : un seul objet à ouvrir, une seule connexion à faire, des dossiers qui se règlent au clic. Et il ajoute une information que la carte ne sait pas encore dire : **depuis quand** une zone n'a pas été touchée.

### Vocabulaire

**Lecteur de dossiers** - le programme qui cartographie les dépôts, lit les journaux de Claude Code et pousse des métadonnées. C'est le `vibemap` d'aujourd'hui, appelé « daemon » dans le dépôt. Ce PRD ne le réécrit pas : il le déménage.

**Zone** - un dossier d'un dépôt, ce que la carte appelle un module et dessine en parcelle.

**Dernière touche** - les deux dates d'une zone : la dernière fois qu'un agent y a écrit, la dernière fois qu'un agent s'est contenté d'y lire.

**Instantané** - l'état du catalogue à la dernière cartographie. Il vaut jusqu'à la suivante, et il est daté.

**Dossier surveillé** - une racine sous laquelle le lecteur cherche des dépôts. `~/Developer` par défaut.

### Deux rythmes, deux exigences

| | Le catalogue | Le vivant |
|---|---|---|
| Ce qu'il montre | quels dépôts existent, où, combien | quel agent travaille, sur quelle zone |
| Ce qu'on lui demande | être là, et dire de quand il date | être exact à la seconde |
| Ce qui le nourrit | la cartographie, toutes les 5 min | les journaux d'agents, toutes les 2 s |

Un catalogue vieux de deux jours ne gêne personne tant que la date est affichée. Une couleur fausse, si.

## Problème

Pour voir un dépôt aujourd'hui, il faut : ouvrir le web, demander un code d'appairage, ouvrir un terminal, taper `vibemap pair <code>`, taper `vibemap`, laisser la fenêtre ouverte, puis attendre jusqu'à cinq minutes. Quatre étapes, dont trois hors du navigateur, et une fenêtre de terminal à ne jamais fermer.

Ajouter un dossier hors de `~/Developer` impose d'éditer un fichier texte caché. Aucun bouton, aucune commande.

Et quand la carte est éteinte, elle ne dit rien du passé : impossible de savoir si une zone a été modifiée hier ou n'a pas été ouverte depuis un mois. La carte répond « qui travaille maintenant », jamais « qui est passé ici récemment ».

## Solution

Une application Mac unique, dans le Dock. On double-clique, on se connecte avec GitHub la première fois, et la carte est là. Le lecteur de dossiers tourne dans l'application, démarré à l'ouverture, arrêté à la fermeture : plus de terminal, plus de code à recopier, plus de fenêtre à laisser ouverte.

L'application porte aussi l'interface : elle la sert depuis la machine, elle ne va pas la chercher sur un site. C'est ce qui lui permet de parler au disque - lister et régler les dossiers surveillés - sans jamais ouvrir cette porte à une page venue d'ailleurs.

Un écran Réglages liste les dossiers surveillés. On en ajoute un avec le sélecteur du système, on en retire un d'un clic, et c'est pris en compte tout de suite. Le fichier de configuration existe toujours, mais on ne l'ouvre plus jamais.

Sur la carte, chaque zone porte deux dates en clair : « modifié il y a 2 j, relu il y a 20 min ». Elles remontent jusqu'à trente jours, alimentées par un dépouillement des journaux déjà présents sur le disque : massif au premier lancement, puis repris à chaque ouverture pour les seuls journaux écrits depuis la fois d'avant. Ce dépouillement renseigne les dates et rien d'autre : il n'allume aucune couleur, parce qu'un agent qui a travaillé la semaine dernière ne travaille pas maintenant.

## Zones touchées

- `bureau/`
- `daemon/src/`
- `daemon/tests/`
- `daemon/launchd/`
- `web/app/`
- `web/lib/`
- `web/lib/supabase/`
- `supabase/migrations/`
- `.github/workflows/`
- `Formula/`
- `README.md`

Le dossier de l'application de bureau est `bureau/`. Ce nom est arrêté : c'est une décision de nommage, pas une inconnue produit, et cette liste sert à détecter les collisions entre chantiers.

## Features à développer

### F1 — Ouvrir la carte depuis le Dock (Priorité : P1)

- **User story** : En tant que développeur, je veux ouvrir Vibe Map comme une application afin de ne plus avoir à trouver un onglet de navigateur.
- **Pourquoi cette priorité** : c'est le contenant de tout le reste. Sans fenêtre, il n'y a ni réglages, ni lecteur embarqué, ni entrée en un geste.
- **Exigences** :
  - **FR-001** : Le système DOIT fournir une application macOS unique qui s'ouvre par double-clic et affiche la carte dans sa propre fenêtre.
  - **FR-002** : La fenêtre NE DOIT PAS présenter de barre d'adresse ni de commande permettant de naviguer ailleurs que dans Vibe Map.
  - **FR-003** : Le système DOIT afficher le même écran que celui servi sur le web, sans en dupliquer le code.
  - **FR-004** : QUAND l'écran de l'interface est injoignable, l'application DOIT l'annoncer dans sa fenêtre et proposer de réessayer, plutôt que de rester blanche ou de se fermer.
  - **FR-005** : Le système DOIT retrouver la taille et la position de sa fenêtre d'un lancement à l'autre.
  - **FR-052** : L'application DOIT servir elle-même l'interface depuis une origine locale de la machine, et sa fenêtre NE DOIT charger aucune page venue d'un domaine distant.
  - **FR-053** : Le système NE DOIT exposer ses commandes locales - celles qui lisent ou règlent un fait du poste - qu'à l'intérieur de sa propre fenêtre, par un pont interne à l'application. Aucune page ouverte dans un navigateur, distante ou locale, NE DOIT pouvoir les appeler, et aucune de ces commandes NE DOIT lire ni écrire un fichier arbitraire.
  - **FR-070** : Le service d'interface DOIT écouter sur `127.0.0.1` à un port fixe et documenté - `51789` -, le même à chaque lancement. QUAND ce port est déjà pris, l'application DOIT annoncer l'indisponibilité dans sa fenêtre et proposer de réessayer ; elle NE DOIT PAS se rabattre en silence sur un autre port.
  - **FR-085** : QUAND la base est injoignable - réseau coupé, hébergeur en panne, adresse qui ne répond pas -, l'application DOIT l'annoncer dans sa fenêtre et proposer de réessayer, plutôt que d'afficher une carte vide, un écran de chargement sans fin ou une erreur brute. C'est le pendant de FR-004 pour la base : FR-004 couvre le service d'interface servi depuis la machine, celle-ci couvre ce qui vient du dehors. La fenêtre elle-même DOIT s'ouvrir malgré tout, puisqu'elle est servie localement.
- **Critères d'acceptation** :
  - [ ] Étant donné l'application installée, quand je double-clique dessus, alors la carte s'affiche dans une fenêtre sans barre d'adresse en moins de 5 secondes.
  - [ ] Étant donné un autre programme mis à écouter sur `127.0.0.1:51789` avant le lancement, quand j'ouvre l'application, alors elle affiche un message d'indisponibilité nommant ce port et un bouton pour réessayer, au lieu d'une fenêtre blanche ; et quand je libère le port puis réessaie, alors la carte s'affiche.
  - [ ] Étant donné l'application ouverte puis quittée et rouverte trois fois, quand j'ouvre `http://127.0.0.1:51789` dans Safari à chacun des trois lancements, alors l'interface répond les trois fois à cette même adresse.
  - [ ] Étant donné le réseau coupé, quand j'ouvre l'application, alors sa fenêtre s'affiche quand même, annonce que la base est injoignable et porte un bouton pour réessayer (FR-085), au lieu d'une carte vide ou d'un chargement sans fin ; et quand je rétablis le réseau puis réessaie, alors la carte se peuple.
  - [ ] Étant donné une fenêtre redimensionnée puis l'application quittée, quand je la rouvre, alors elle retrouve la même taille et la même position.
  - [ ] Étant donné l'application ouverte, quand j'observe le trafic réseau qu'elle produit au chargement de sa fenêtre, alors aucune page ni aucun script ne vient du site hébergé : seuls les appels à la base sortent de la machine.
- **Hors scope** : Windows et Linux, mode plein écran dédié, icône de barre de menus.

### F2 — Le lecteur vit et meurt avec l'app (Priorité : P1)

- **User story** : En tant que développeur, je veux que le lecteur de dossiers démarre et s'arrête avec l'application afin de ne plus tenir un terminal ouvert.
- **Pourquoi cette priorité** : c'est la moitié de la promesse « un seul objet ». Sans elle, l'application n'est qu'un navigateur déguisé. Cette feature se teste seule, avant F3 et F4 : on la joue sur une machine déjà reliée avant ce chantier, dont le jeton rangé au trousseau suffit à faire battre le lecteur embarqué.
- **Exigences** :
  - **FR-006** : QUAND l'application s'ouvre, le système DOIT démarrer le lecteur de dossiers dans son propre processus, sans aucune intervention et sans terminal.
  - **FR-007** : QUAND l'application se ferme, le système DOIT arrêter ce processus ; aucun lecteur NE DOIT survivre à la fermeture.
  - **FR-008** : Le système NE DOIT PAS faire tourner deux lecteurs à la fois sur la même machine.
  - **FR-009** : Le système DOIT afficher l'état du lecteur - en marche, arrêté, en échec - et, en cas d'échec, sa raison en clair.
  - **FR-010** : QUAND le lecteur s'arrête de lui-même, le système DOIT le signaler dans la fenêtre et permettre de le relancer sans quitter l'application.
  - **FR-011** : Le système NE DOIT PLUS proposer de démarrage automatique du lecteur à l'ouverture de session : il ne tourne que pendant que l'application est ouverte.
  - **FR-054** : QUAND un lecteur tourne déjà sur cette machine, quelle que soit la manière dont il a été lancé, l'application NE DOIT PAS en démarrer un second et DOIT le signaler dans sa fenêtre.
  - **FR-081** : Le verrou qui garantit l'unicité du lecteur appartient au lecteur partagé lui-même : il DOIT être posé par les deux véhicules qui l'embarquent - l'application de bureau et le binaire en ligne de commande -, et la publication DOIT les porter à la même version. Un lecteur issu d'une version antérieure à ce verrou ne pose aucune marque sur le poste : il n'est pas détectable, FR-054 ne vaut pas contre lui, et le système NE DOIT PAS annoncer une garantie plus large que celle-là.
- **Critères d'acceptation** :
  - [ ] Étant donné l'application ouverte depuis une minute, quand je consulte la liste des machines depuis un autre appareil, alors la machine est vue il y a moins de 90 secondes.
  - [ ] Étant donné l'application ouverte, quand je la quitte, alors aucun processus de lecture ne subsiste dans le Moniteur d'activité, et la machine passe muette au bout de 90 secondes.
  - [ ] Étant donné l'application déjà ouverte, quand je la relance, alors un seul lecteur tourne.
  - [ ] Étant donné `vibemap` lancé au terminal, dans la version publiée avec l'application et posant donc le même verrou, quand j'ouvre l'application, alors elle signale qu'un lecteur tourne déjà, et le Moniteur d'activité n'en montre qu'un seul.
  - [ ] Étant donné une session du Mac ouverte et l'application jamais lancée depuis le démarrage, quand j'attends deux minutes sans l'ouvrir, alors aucun processus de lecture ne tourne dans le Moniteur d'activité et la machine reste muette dans la liste des machines.
  - [ ] Étant donné l'application ouverte et son lecteur en marche, quand je termine le processus du lecteur depuis le Moniteur d'activité, alors l'application signale l'arrêt dans sa fenêtre en moins de 10 secondes, en donne la raison, et porte un bouton qui le relance sans que j'aie à quitter l'application ; après ce clic, la machine est de nouveau vue il y a moins de 90 secondes.
  - [ ] Étant donné une application recompilée, dont le trousseau redemande donc l'autorisation d'accéder au jeton, quand j'ouvre l'application et que je refuse la boîte de dialogue du système, alors elle affiche la raison du refus et un bouton pour réessayer, au lieu d'une carte muette sans explication.
- **Hors scope** : exécution en arrière-plan sans fenêtre, service de session, suivi pendant que l'application est fermée.

### F3 — Entrer avec GitHub, une seule fois (Priorité : P1)

- **User story** : En tant que développeur, je veux me connecter une fois pour toutes au premier lancement afin de n'avoir plus jamais qu'un double-clic à faire.
- **Pourquoi cette priorité** : c'est la première chose que l'utilisateur rencontre, et la marche la plus haute du parcours actuel.
- **Exigences** :
  - **FR-012** : QUAND l'application s'ouvre sans session, le système DOIT présenter un unique bouton de connexion avec GitHub, et rien d'autre.
  - **FR-013** : QUAND la connexion réussit, le système DOIT ouvrir la carte directement, sans étape supplémentaire.
  - **FR-014** : Le système DOIT conserver la session d'un lancement à l'autre : une ouverture suivante NE DOIT PAS redemander de se connecter.
  - **FR-015** : Le système DOIT permettre de se déconnecter depuis l'application, et QUAND l'utilisateur se déconnecte, il DOIT arrêter le lecteur.
  - **FR-016** : QUAND la connexion échoue, le système DOIT afficher la raison dans la fenêtre et permettre de recommencer.
  - **FR-071** : QUAND l'utilisateur demande à se connecter, le système DOIT ouvrir la page d'autorisation GitHub dans le navigateur du système, jamais dans la fenêtre de l'application, et DOIT demander un retour d'autorisation vers l'origine locale fixe de l'application définie par FR-070.
  - **FR-072** : QUAND l'autorisation revient, l'application DOIT reprendre la main dans sa propre fenêtre, session ouverte, sans que l'utilisateur ait à la rouvrir ni à recopier quoi que ce soit du navigateur.
- **Critères d'acceptation** :
  - [ ] Étant donné un Mac où Vibe Map n'a jamais été lancée, quand je l'ouvre, alors l'application ne présente qu'un bouton « Se connecter avec GitHub », et quand je m'en sers, alors la page d'autorisation s'ouvre dans mon navigateur habituel, et non dans la fenêtre de l'application.
  - [ ] Étant donné cette page d'autorisation ouverte dans le navigateur, quand j'accepte, alors la fenêtre de l'application repasse d'elle-même au premier plan avec la carte affichée, sans que je l'aie rouverte ni cliqué ailleurs.
  - [ ] Étant donné une première connexion faite, quand je quitte l'application et la rouvre, alors la carte s'affiche directement.
  - [ ] Étant donné une session ouverte et le lecteur en marche, quand je me déconnecte, alors l'écran de connexion revient, et le Moniteur d'activité ne montre plus aucun processus de lecture.
  - [ ] Étant donné une autorisation GitHub refusée, quand je reviens dans l'application, alors elle affiche la raison du refus et me laisse recommencer.
- **Hors scope** : autres fournisseurs d'identité, plusieurs comptes ouverts en même temps, connexion par mot de passe.
- **[À CLARIFIER]** : le véhicule du retour d'autorisation, si un essai infirme le chemin retenu. L'origine locale fixe est le chemin de référence ; un schéma d'adresse propre à l'application (`vibemap://`) enregistré par le paquet est la solution de repli, et seul le véhicule changerait - le navigateur du système, lui, reste le lieu de l'autorisation.

### F4 — La machine se déclare elle-même (Priorité : P1)

- **User story** : En tant que développeur, je veux que mon Mac soit reconnu sans code d'appairage afin de ne rien avoir à recopier d'un écran à l'autre.
- **Pourquoi cette priorité** : le code d'appairage n'existait que pour faire se reconnaître deux objets séparés. Ils n'en font plus qu'un. Cette feature suppose F3 livrée : la déclaration d'une machine présente la session de l'utilisateur, qui n'existe qu'une fois la connexion faite.
- **Exigences** :
  - **FR-017** : QUAND une session est ouverte sur une machine que le compte ne connaît pas encore, le système DOIT enregistrer cette machine sans aucune saisie.
  - **FR-018** : Le système DOIT ranger le jeton de la machine dans le trousseau du système, et JAMAIS dans un fichier. Il DOIT le ranger sous le service `fr.yarma.vibemap` et sous le compte que forme l'identifiant de la machine - la même entrée, exactement, que celle du binaire en ligne de commande - afin que les deux véhicules du lecteur lisent un seul et même jeton.
  - **FR-019** : Le système DOIT réutiliser la même machine à chaque lancement suivant : une nouvelle déclaration NE DOIT créer ni doublon de machine, ni doublon de dépôts.
  - **FR-020** : Le système DOIT nommer la machine d'après le nom que lui donne le système d'exploitation, et afficher ce nom dans les réglages.
  - **FR-021** : QUAND la machine a été révoquée depuis le web, le système DOIT l'annoncer dans la fenêtre et cesser d'émettre, sans se redéclarer sous une autre identité.
  - **FR-022** : Le système NE DOIT transmettre, au moment de la déclaration, que le nom de la machine et sa plateforme.
  - **FR-055** : Le système DOIT conserver localement l'identifiant de la machine déclarée, à côté de son jeton, et le présenter à chaque lancement suivant : c'est à cet identifiant, et à lui seul, que l'application reconnaît sa machine.
  - **FR-056** : QUAND l'identifiant conservé ne correspond à aucune machine du compte, le système DOIT redéclarer la machine et l'annoncer dans la fenêtre, plutôt que d'échouer en silence ou de rester muet.
  - **FR-057** : QUAND une configuration de ligne de commande existe déjà sur le poste, le système DOIT en reprendre l'identifiant de machine et les dossiers surveillés au premier lancement, sans les dupliquer ni les effacer.
  - **FR-073** : Le système NE DOIT PAS reprendre l'adresse de la base inscrite dans une configuration de ligne de commande existante : l'application publiée DOIT tenir cette adresse de sa propre compilation, la même pour tous les postes, et une configuration qui en désigne une autre NE DOIT PAS la détourner.
  - **FR-080** : QUAND une configuration de ligne de commande existe déjà sur le poste, le système DOIT reprendre le jeton de cette machine dans l'entrée de trousseau existante - service `fr.yarma.vibemap`, compte = identifiant de la machine repris par FR-057 -, et NE DOIT PAS en demander un neuf. Le système d'exploitation demandera son autorisation à l'utilisateur, l'application étant un exécutable différent du binaire en ligne de commande : c'est attendu, et le système DOIT le présenter comme tel plutôt que comme une anomalie. QUAND cet accès est refusé, le système DOIT l'annoncer dans sa fenêtre avec un bouton pour réessayer, et NE DOIT en aucun cas redéclarer la machine à la place.
- **Critères d'acceptation** :
  - [ ] Étant donné un Mac neuf, quand je me connecte pour la première fois dans l'application, alors la machine apparaît dans la liste des machines sans qu'aucun code d'appairage n'ait été demandé.
  - [ ] Étant donné une machine déjà déclarée, quand je quitte l'application et la rouvre trois fois, alors la liste ne porte toujours qu'une machine et le nombre de dépôts n'a pas changé.
  - [ ] Étant donné la machine révoquée depuis le web, quand l'application est ouverte, alors elle annonce la révocation et n'écrit plus rien.
  - [ ] Étant donné une base remise à zéro, dont l'identifiant de machine conservé localement ne désigne donc plus rien, quand je rouvre l'application, alors elle annonce que la machine a été redéclarée, la liste des machines n'en porte qu'une, et la carte se repeuple.
  - [ ] Étant donné un poste où `vibemap` était déjà appairé et surveillait `~/Developer` et `~/Sites`, quand j'ouvre l'application pour la première fois, alors la liste des machines n'en gagne aucune nouvelle et l'écran Réglages montre ces deux dossiers, sans doublon.
  - [ ] Étant donné ce même poste dont la configuration de ligne de commande désigne une base locale, quand j'ouvre l'application publiée, alors elle parle bien à la base de sa compilation - la machine y apparaît - et non à la base locale.
  - [ ] Étant donné un Mac nommé « MacBook de Yarma » dans les réglages du système, quand j'ouvre l'écran Réglages de l'application, alors ce nom y est affiché comme nom de la machine, identique à celui de la liste des machines.
  - [ ] Étant donné ce même poste déjà appairé avant ce chantier, quand j'ouvre l'application pour la première fois et que je refuse la demande d'accès au trousseau que le système affiche, alors l'application annonce le refus et porte un bouton pour réessayer, et la liste des machines n'en gagne aucune nouvelle ; et quand je réessaie en acceptant, alors la machine d'origine reprend son battement, toujours sans doublon.
  - [ ] Étant donné un Mac neuf, quand j'observe ce que l'application envoie au moment où elle déclare la machine, alors le message ne porte que le nom de la machine et sa plateforme, et aucun chemin absolu.
- **Hors scope** : renommer une machine depuis l'application, transférer une machine d'un compte à l'autre.

### F5 — Retirer le code d'appairage du parcours (Priorité : P2)

- **User story** : En tant que développeur, je veux que plus aucun écran ne me propose de code ni de commande afin qu'il n'existe qu'une seule manière d'entrer.
- **Pourquoi cette priorité** : le produit tient sans ce nettoyage, mais deux portes d'entrée dont une seule sert finissent par se contredire à l'écran. Son deuxième critère - le lien de téléchargement mène à une application téléchargeable - ne se joue qu'une fois F10 livrée : sans publication, ce lien pointe vers le vide. Le reste de la feature, lui, se teste seul.
- **Exigences** :
  - **FR-023** : Le système NE DOIT PLUS proposer de créer un code d'appairage depuis l'écran web.
  - **FR-024** : Le système NE DOIT PLUS afficher de commande à recopier dans un terminal pour relier une machine.
  - **FR-025** : QUAND aucune machine n'est encore déclarée, le premier lancement DOIT diriger vers l'installation et l'ouverture de l'application de bureau, jamais vers un terminal.
  - **FR-026** : Le système DOIT continuer d'accepter les machines déjà reliées par un code : un jeton existant NE DOIT PAS cesser de fonctionner.
  - **FR-058** : L'écran de premier lancement DOIT porter un lien vers la page des publications GitHub du dépôt - l'emplacement où l'intégration continue dépose l'application (FR-067), et le seul que ce PRD retient -, atteignable en un clic.
  - **FR-082** : Plus aucun appelant NE DOIT emprunter le chemin d'appairage par code, ni à l'écran ni en coulisses : le binaire en ligne de commande NE DOIT PLUS proposer de relier une machine par code, et DOIT se contenter de lire la configuration et le jeton que l'application de bureau a écrits sur le poste. Il devient le compagnon de l'application sur le même Mac, jamais une porte d'entrée autonome. Un poste qui n'a pas ouvert l'application de bureau n'a donc plus aucun chemin pour se relier, et les messages du binaire qui renvoyaient vers un code DOIVENT renvoyer vers l'application. La table des codes et la fonction d'échange cessent ainsi d'être un chemin d'entrée du seul fait que plus rien ne les appelle ; leur suppression en base, elle, reste hors scope de cette feature.
- **Critères d'acceptation** :
  - [ ] Étant donné un compte connecté sur le web, quand j'ouvre l'accueil, alors aucun code d'appairage ni aucune commande de terminal n'y figure.
  - [ ] Étant donné un compte sans aucune machine, quand j'ouvre l'accueil, alors l'écran me dit d'installer et d'ouvrir l'application de bureau, et son lien de téléchargement mène à une application téléchargeable.
  - [ ] Étant donné une machine reliée avant ce changement, quand son lecteur émet, alors ses écritures sont toujours acceptées.
  - [ ] Étant donné le binaire en ligne de commande de la version publiée, sur un Mac où l'application de bureau n'a jamais été ouverte et où aucune configuration n'existe donc, quand je l'exécute et que je parcours ses commandes, alors aucune ne relie une machine par code, et son message me renvoie vers l'application de bureau.
- **Hors scope** : la suppression de la table des codes et de la fonction d'échange en base. Plus rien ne les appelle une fois FR-082 tenue ; leur retrait est un nettoyage de schéma, qui se fait à part et ne conditionne pas cette feature.

### F6 — Voir ce qui est surveillé (Priorité : P1)

- **User story** : En tant que développeur, je veux voir quels dossiers sont surveillés et ce qu'on y a trouvé afin de comprendre pourquoi un dépôt manque à l'appel.
- **Pourquoi cette priorité** : aujourd'hui, la seule réponse à « pourquoi ce dépôt n'apparaît pas » est d'ouvrir un fichier caché. Lire vient avant régler. Cette feature suppose F1 livrée : l'écran Réglages vit dans la fenêtre de l'application, et le pont qui lui donne les faits du poste n'existe qu'à l'intérieur de celle-ci.
- **Exigences** :
  - **FR-027** : Le système DOIT présenter, dans un écran Réglages de l'application, la liste des dossiers surveillés.
  - **FR-028** : Le système DOIT indiquer, pour chaque dossier, combien de dépôts y ont été trouvés à la dernière cartographie.
  - **FR-029** : QUAND un dossier surveillé est introuvable ou illisible, le système DOIT le signaler sur sa ligne au lieu de l'afficher comme les autres.
  - **FR-030** : Le système DOIT afficher la date et l'heure de la dernière cartographie.
  - **FR-059** : La liste des dossiers surveillés, leur compte de dépôts, leur lisibilité et leur état d'autorisation DOIVENT être fournis à l'écran par l'application de bureau elle-même : ce sont des faits purement locaux, qui n'existent nulle part ailleurs que sur le poste et ne sortent pas de la machine. L'heure de la dernière cartographie, elle, DOIT continuer d'être lue en base avec le reste du catalogue : elle y est déjà, et c'est ce qui la rend lisible depuis un autre appareil et par-delà les fermetures de l'application.
  - **FR-060** : QUAND l'écran Réglages est ouvert dans un navigateur ordinaire, hors de l'application de bureau, le système DOIT afficher à la place de ces faits locaux une mention expliquant que ces réglages n'existent que dans l'application, et NE DOIT PAS afficher de liste vide ni d'erreur. Cette mention NE DOIT remplacer que ces faits du poste : ce que l'écran tient de la base, lui, continue de s'y afficher comme avant.
  - **FR-074** : QUAND aucune cartographie n'a jamais eu lieu sur ce poste, le système DOIT dire « jamais cartographié » à la place de l'heure, et « en attente de la première cartographie » à la place du compte de dépôts de chaque dossier, plutôt qu'une heure vide ou un compte de zéro qui se lirait comme un dossier vide. SINON le système DOIT afficher l'heure de la dernière cartographie connue, même antérieure au lancement en cours, et QUAND une cartographie est en train de se faire, la signaler comme telle à côté de cette heure au lieu de la remplacer. QUAND la base est injoignable et que l'heure connue ne peut donc pas être lue, le système DOIT le dire - comme FR-085 l'exige pour tout ce que la base porte - plutôt que de conclure « jamais cartographié ».
  - **FR-086** : L'heure de la dernière cartographie est un agrégat, et le système DOIT l'établir comme tel : chaque dépôt porte en base l'heure de sa propre cartographie, et l'écran DOIT afficher **la plus récente** de celles des dépôts de cette machine. Il NE DOIT PAS afficher une heure par dépôt à cet endroit, ni celle d'un dépôt pris au hasard.
  - **FR-087** : QUAND une cartographie a abouti sur ce poste sans y trouver aucun dépôt, la base ne porte aucune heure dont FR-086 pourrait tirer un agrégat, et FR-074 conclurait à tort « jamais cartographié » alors qu'une cartographie a bien abouti. Ce cas DOIT donc se distinguer des deux autres : le système DOIT dire que la dernière cartographie n'a trouvé aucun dépôt, et non « jamais cartographié ». Le fait « une cartographie a abouti sans rien trouver » est un fait purement local, au même titre que l'indication qu'une cartographie est en cours : il DOIT être fourni par l'application de bureau elle-même, par le pont des commandes locales, sans contredire FR-059 - qui ne réserve à la base que l'heure de cartographie, jamais ce fait-là. Hors de l'application, dans un navigateur ordinaire où FR-060 retire les faits du poste, ce fait n'est pas disponible : l'écran s'en tient alors à ce que la base porte.
- **Critères d'acceptation** :
  - [ ] Étant donné une configuration qui surveille `~/Developer`, quand j'ouvre Réglages dans l'application, alors ce dossier est listé avec le nombre de dépôts trouvés et l'heure de la dernière cartographie.
  - [ ] Étant donné un poste où aucune cartographie n'a jamais abouti et une application qui vient de s'ouvrir, quand j'ouvre Réglages, alors l'heure de cartographie dit « jamais cartographié » et les dossiers annoncent une attente, pas zéro dépôt ; et quand la première cartographie est passée et que je rouvre Réglages, alors l'heure et les comptes ont pris leur place.
  - [ ] Étant donné un poste cartographié la veille, puis l'application quittée, quand je la rouvre et ouvre Réglages avant que la cartographie du lancement en cours n'ait abouti, alors l'heure affichée est celle de la veille - jamais « jamais cartographié » -, et la cartographie en cours est signalée à côté d'elle.
  - [ ] Étant donné deux dossiers surveillés dont les dépôts ont été cartographiés à des moments différents - `~/Sites` ajouté et cartographié il y a cinq minutes, `~/Developer` cartographié il y a une heure -, quand j'ouvre Réglages, alors l'heure de la dernière cartographie affichée est celle d'il y a cinq minutes, la plus récente des deux, et une seule heure est affichée (FR-086).
  - [ ] Étant donné un poste dont le seul dossier surveillé ne contient aucun dépôt, et dont la cartographie a donc abouti sans rien trouver, quand j'ouvre Réglages, alors l'écran dit que la dernière cartographie n'a trouvé aucun dépôt (FR-087), et ne dit ni « jamais cartographié » ni « en attente de la première cartographie ».
  - [ ] Étant donné un dossier surveillé renommé sur le disque, quand j'ouvre Réglages, alors sa ligne le signale comme introuvable, et les autres dossiers restent affichés normalement.
  - [ ] Étant donné le site hébergé ouvert dans Safari, quand j'affiche Réglages, alors la section des dossiers surveillés est remplacée par une mention disant que ces réglages n'existent que dans l'application de bureau, et le reste de l'écran - à commencer par les comptes - s'affiche normalement, sans liste vide ni erreur.
- **Hors scope** : réglage des cadences, de la fenêtre d'activité ou des seuils ; édition du fichier de configuration depuis l'écran.

### F7 — Ajouter et retirer un dossier au clic (Priorité : P1)

- **User story** : En tant que développeur, je veux ajouter un dossier avec le sélecteur du Mac afin de ne plus jamais éditer un fichier texte caché.
- **Pourquoi cette priorité** : c'est la deuxième douleur exprimée, et la seule qui obligeait encore à connaître l'existence d'un fichier de configuration. Cette feature suppose F1 et F6 livrées : le sélecteur du système s'ouvre depuis la fenêtre de l'application, et les dossiers s'ajoutent et se retirent dans la liste que F6 met à l'écran.
- **Exigences** :
  - **FR-031** : Le système DOIT permettre d'ajouter un dossier surveillé par le sélecteur de fichiers du système.
  - **FR-032** : Le système DOIT permettre de retirer un dossier surveillé.
  - **FR-033** : QUAND un dossier est ajouté ou retiré, le système DOIT en tenir compte sans redémarrage de l'application.
  - **FR-034** : QUAND un dossier est ajouté, le système DOIT le cartographier sans attendre la cartographie périodique suivante, et ses dépôts DOIVENT être visibles en moins d'une minute.
  - **FR-035** : QUAND un dossier est retiré, le système DOIT cesser de cartographier ses dépôts et NE DOIT PAS effacer ce qui a déjà été observé.
  - **FR-036** : L'utilisateur NE DOIT JAMAIS avoir à ouvrir un fichier de configuration pour ajouter ou retirer un dossier.
  - **FR-037** : Le système DOIT refuser d'ajouter deux fois le même dossier, et le dire.
  - **FR-061** : QUAND le système d'exploitation refuse la lecture d'un dossier ajouté, l'application DOIT le dire sur la ligne de ce dossier et proposer de redemander l'autorisation, y compris lorsque le refus survient après l'ajout.
  - **FR-075** : Le système DOIT juger qu'un dossier est déjà surveillé après avoir déplié le `~`, résolu les liens symboliques et normalisé le chemin - barre finale comprise. Il DOIT refuser également un dossier contenu dans un dossier déjà surveillé, ou qui en contient un, et DOIT nommer dans son refus le dossier déjà surveillé qui s'y oppose.
  - **FR-076** : QUAND un dossier ajouté ne contient aucun dépôt à la profondeur explorée, le système DOIT le dire sur sa ligne en nommant ce qui a été cherché - un dossier contenant `.git`, parmi les enfants directs du dossier choisi -, plutôt que d'afficher zéro dépôt sans explication.
- **Critères d'acceptation** :
  - [ ] Étant donné l'application ouverte, quand j'ajoute `~/Sites` par le sélecteur, alors ses dépôts apparaissent sur l'accueil en moins d'une minute, sans redémarrage et sans qu'aucun fichier n'ait été ouvert.
  - [ ] Étant donné `~/Sites` surveillé, quand je le retire, alors il disparaît de la liste des dossiers et ses dépôts ne reçoivent plus de nouvelle cartographie.
  - [ ] Étant donné `~/Developer` déjà surveillé, quand je tente de l'ajouter une seconde fois - puis sous la forme `~/Developer/`, puis par son chemin absolu, puis par un lien symbolique qui pointe dessus -, alors l'application refuse les quatre fois en nommant `~/Developer` comme le dossier déjà surveillé.
  - [ ] Étant donné `~/Developer` déjà surveillé, quand je tente d'ajouter `~/Developer/vibecode-traker-app` qui est dedans, puis `~` qui le contient, alors l'application refuse les deux fois en disant lequel des deux dossiers est déjà surveillé.
  - [ ] Étant donné un dossier `~/Archives` qui est lui-même un dépôt et ne contient aucun dépôt en enfant direct, quand je l'ajoute au sélecteur, alors sa ligne dit qu'aucun dépôt n'a été trouvé et nomme ce qui a été cherché, au lieu d'afficher « 0 dépôt » sans explication.
  - [ ] Étant donné que je choisis `~/Documents` au sélecteur et que je refuse la demande d'accès du système, quand j'ouvre Réglages, alors la ligne de ce dossier annonce que l'autorisation manque et porte un bouton pour la redemander.
- **Hors scope** : exclusions fines à l'intérieur d'un dossier, motifs de filtrage, listes noires.
- **[À CLARIFIER]** : la profondeur d'exploration elle-même. La cartographie n'examine aujourd'hui que les **enfants directs** d'un dossier surveillé ; FR-076 garantit que ce choix ne se paie plus d'un silence, mais ne tranche pas s'il faut descendre plus profond, ni reconnaître un dossier qui est lui-même un dépôt. À trancher sur l'usage, une fois FR-076 en place.
- **[À CLARIFIER]** : que deviennent les dépôts d'un dossier qu'on cesse de surveiller - restent-ils au catalogue, figés et datés, ou en disparaissent-ils ? FR-035 retient le comportement prudent par défaut, en attendant la décision.

### F8 — Lire la dernière touche d'une zone (Priorité : P1)

- **User story** : En tant que développeur, je veux lire sur une zone quand un agent y a écrit et quand il s'y est contenté de lire afin de savoir ce qui a bougé récemment sans ouvrir le code.
- **Pourquoi cette priorité** : c'est la seule question à laquelle la carte ne sait pas répondre quand personne ne travaille, c'est-à-dire la plupart du temps.
- **Exigences** :
  - **FR-038** : Le système DOIT tenir, pour chaque zone d'un dépôt, deux dates distinctes : la dernière écriture d'un agent et la dernière lecture.
  - **FR-039** : QUAND un agent écrit dans une zone, le système DOIT mettre à jour sa date de dernière écriture, et NE DOIT PAS toucher sa date de dernière lecture.
  - **FR-040** : QUAND un agent lit dans une zone, le système DOIT mettre à jour sa date de dernière lecture.
  - **FR-041** : Le système DOIT rendre les deux dates de chaque parcelle lisibles en mots, jamais par la seule couleur, sous la forme « modifié il y a 2 j, relu il y a 20 min » : sur la parcelle elle-même quand elle est assez grande pour porter du texte, et dans l'infobulle de la parcelle dans tous les cas.
  - **FR-042** : Chaque date est jugée séparément : QUAND une date remonte à plus de 30 jours ou n'est pas connue, le système DOIT dire « rien de récent » pour celle-ci et afficher l'autre normalement. QUAND les deux sont dans ce cas, la zone entière DOIT dire « rien de récent », sans donner aucune date.
  - **FR-043** : Le système DOIT faire hériter une zone de la dernière touche de tout ce qui se passe sous elle. Cet héritage DOIT se calculer à la lecture, par préfixe de chemin, et NON à l'écriture : une touche dans une zone n'écrit qu'un seul enregistrement, celui de la zone touchée, et aucune ligne d'ancêtre n'est tenue à jour. C'est ce qui interdit toute dérive entre une zone et ses parents.
  - **FR-044** : Ces deux dates NE DOIVENT PAS entrer dans le calcul des couleurs de la carte : elles se lisent, elles n'allument rien.
  - **FR-045** : Le système NE DOIT transmettre hors de la machine, pour cette fonction, que des chemins de zone relatifs et des horodatages.
  - **FR-062** : Les deux dates DOIVENT survivre à une cartographie : le remplacement en bloc de la carte des zones d'un dépôt NE DOIT PAS les effacer, ni les remettre à vide.
  - **FR-063** : QUAND une parcelle est trop petite pour porter ses dates en clair, le système DOIT les rendre atteignables au survol et au clavier, sans changer de page.
  - **FR-083** : Pour que FR-063 soit tenue, deux choses DOIVENT changer sur la carte actuelle. Une parcelle sans sous-dossier, aujourd'hui désactivée et donc hors du parcours de tabulation, DOIT redevenir atteignable au clavier même si elle ne s'ouvre pas. Et les deux dates NE DOIVENT PAS reposer sur la seule infobulle native du navigateur, que le focus clavier n'affiche pas : le système DOIT les faire paraître au focus comme au survol.
  - **FR-064** : QUAND une zone n'existe plus dans le dépôt à la dernière cartographie, le système NE DOIT PLUS la dessiner sur la carte, mais NE DOIT PAS effacer ses deux dates : si la zone réapparaît, elles réapparaissent avec elle.
  - **FR-077** : Les deux dates DOIVENT s'ajouter à ce que porte déjà une parcelle lisible - son nom, la ligne d'activité vivante, son poids en lignes - sans remplacer aucune de ces trois lignes ni les réordonner : la ligne d'activité vivante dit ce qui se passe maintenant, les dates disent ce qui s'est passé avant, et QUAND aucun agent ne travaille dans la zone, la place de la ligne d'activité vivante DOIT rester vide plutôt que d'être occupée par une date. Dans l'infobulle, les deux dates DOIVENT s'ajouter au chemin, au nombre de lignes et au nombre de fichiers déjà présents.
  - **FR-088** : **Une date de dernière touche NE DOIT jamais reculer.** Quel que soit le canal qui l'écrit - la lecture vivante des journaux ou le dépouillement en arrière, qui écrivent tous deux dans le même agrégat -, une écriture NE DOIT remplacer une date existante que si la sienne est plus récente ; sinon elle DOIT être ignorée sans erreur. Le dépouillement en arrière (F9) est tenu par cette règle au même titre que le direct : il parcourt du passé, et un enregistrement naïf ferait reculer une date fraîche au moment précis où l'utilisateur découvre la fonctionnalité. La même règle vaut pour un envoi rejoué ou arrivé en retard par la file d'attente.
- **Critères d'acceptation** :
  - [ ] Étant donné un agent qui vient d'écrire dans `web/lib`, quand j'ouvre la parcelle `web` sur la carte puis survole la parcelle `lib` qu'elle contient, alors son infobulle annonce une dernière écriture de moins d'une minute et une dernière lecture inchangée, à la suite du chemin et des compteurs déjà affichés.
  - [ ] Étant donné un agent qui ne fait que lire dans `daemon/src`, quand j'ouvre la parcelle `daemon` puis survole la parcelle `src`, alors sa dernière lecture est fraîche et sa dernière écriture n'a pas bougé.
  - [ ] Étant donné une parcelle assez grande pour porter du texte et aucun agent au travail dedans, quand je la regarde, alors elle porte son nom, ses deux dates et son poids en lignes, et rien ne s'est substitué à son nom ni à son poids.
  - [ ] Étant donné une zone dont les deux dates remontent à 40 jours, quand je survole sa parcelle, alors elle dit « rien de récent » et n'affiche aucune date.
  - [ ] Étant donné une zone écrite il y a 40 jours mais relue il y a 20 minutes, quand je survole sa parcelle, alors elle dit « rien de récent » pour l'écriture et « relu il y a 20 min » pour la lecture.
  - [ ] Étant donné une zone écrite il y a deux jours et plus aucun agent en cours, quand je consulte la carte, alors cette zone porte ses deux dates et reste de la couleur d'une zone au repos.
  - [ ] Étant donné une zone parente dont seul un sous-dossier a été écrit, quand je survole la parcelle parente, alors elle porte la date de ce sous-dossier.
  - [ ] Étant donné une parcelle trop étroite pour afficher du texte et sans sous-dossier - donc hors du parcours de tabulation aujourd'hui -, quand je la joins au clavier par tabulations successives, alors elle prend le focus et ses deux dates s'affichent sans que la page change.
  - [ ] Étant donné un agent qui écrit dans une zone, quand j'observe ce que l'application envoie pour tenir ses deux dates, alors le message ne porte que des chemins de zone relatifs et des horodatages : aucun chemin absolu, aucun contenu de fichier, aucun nom d'agent.
  - [ ] Étant donné une zone écrite il y a deux jours, quand j'attends la cartographie suivante - au plus 5 minutes - puis que je recharge la carte, alors ses deux dates sont toujours celles d'avant, inchangées.
  - [ ] Étant donné une zone renommée sur le disque puis remise sous son ancien nom, quand la cartographie repasse, alors la parcelle disparaît de la carte puis y revient avec ses deux dates d'origine.
- **Hors scope** : le détail des fichiers touchés, l'agent ou la session responsable, le nombre de passages.

### F9 — Retrouver trente jours de passé sans rallumer la carte (Priorité : P2)

- **User story** : En tant que développeur, je veux que les dates de dernière touche soient déjà justes au premier lancement afin de ne pas attendre un mois pour qu'elles servent.
- **Pourquoi cette priorité** : sans ce dépouillement, F8 dit la vérité mais ne dit rien : « rien de récent » partout. C'est un enrichissement de forte valeur, pas une fondation. Cette feature suppose F1 et F8 livrées : le dépouillement est mené par l'application, son avancement se lit dans l'écran Réglages, et il n'a de dates à renseigner que là où F8 les tient.
- **Exigences** :
  - **FR-046** : Le système DOIT dépouiller les journaux d'agents déjà présents sur le disque pour renseigner les deux dates des 30 derniers jours.
  - **FR-047** : Ce dépouillement NE DOIT produire aucune activité vivante : aucune couleur allumée, aucune session affichée, aucune ligne dans le journal direct.
  - **FR-048** : Le système DOIT mener ce dépouillement sans interrompre la lecture des journaux en cours ni le battement de la machine.
  - **FR-049** : QUAND un dépouillement est interrompu, le système DOIT le reprendre où il s'est arrêté au lancement suivant, sans tout relire.
  - **FR-050** : Le système DOIT indiquer dans les réglages où en est le dépouillement - le nombre de journaux dépouillés sur le nombre total à dépouiller - et quand il s'est terminé.
  - **FR-051** : Le système NE DOIT PAS remonter au-delà de 30 jours.
  - **FR-065** : QUAND aucun journal d'agent n'est présent sur le poste, le dépouillement DOIT se terminer immédiatement, et les réglages DOIVENT l'indiquer plutôt que d'afficher un avancement qui ne bouge pas.
  - **FR-066** : QUAND un événement dépouillé vise un dépôt absent du catalogue, le système DOIT l'ignorer sans erreur et sans interrompre le dépouillement.
  - **FR-078** : À chaque ouverture de l'application, le système DOIT dépouiller les journaux écrits depuis la fin du dépouillement précédent, et NE DOIT jamais relire ceux qu'il a déjà dépouillés. Le dépouillement n'est donc ni une opération unique réservée au premier lancement, ni une relecture complète à chaque fois : c'est le même mécanisme de reprise que celui de FR-049, appliqué à toutes les ouvertures.
  - **FR-084** : La marque de progression du dépouillement en arrière DOIT être tenue séparément de la position de lecture des journaux qui sert le direct : ce sont deux marques distinctes, chacune avançant à son rythme, et aucune des deux NE DOIT écraser l'autre ni la faire reculer. QUAND le binaire en ligne de commande a fait avancer la position de lecture du direct pendant que l'application était fermée, le dépouillement en arrière DOIT quand même reprendre à sa propre marque et couvrir tout l'intervalle depuis celle-ci, sans tenir pour dépouillé ce que le direct a consommé ; et cette reprise NE DOIT PAS faire rejouer en direct l'activité déjà émise pendant la fermeture.
  - **FR-089** : QUAND l'application s'ouvre après une longue fermeture pendant laquelle **aucun lecteur n'a tourné**, la lecture vivante reprend à sa position enregistrée, qui date d'avant la fermeture : le système NE DOIT alors émettre en direct - couleurs, sessions, journal direct - que les événements compris dans la fenêtre de rattrapage du direct, et DOIT écarter les plus anciens, **y compris ceux d'un journal qu'il connaissait déjà**. Cette borne vaut aujourd'hui pour les seuls journaux jamais vus ; elle DOIT valoir pour tous. Les événements ainsi écartés ne sont pas perdus pour autant : c'est le dépouillement en arrière (FR-078, FR-084) qui en tire les deux dates, sans passer par le direct.
- **Critères d'acceptation** :
  - [ ] Étant donné un Mac dont les journaux d'agents remontent à deux mois et sur lequel aucun agent ne tourne, quand j'ouvre l'application et que le dépouillement se termine, alors les zones travaillées le mois dernier portent leurs deux dates et aucune zone de la carte n'est bleue, ambre ou rouge.
  - [ ] Étant donné ce même dépouillement terminé, quand j'ouvre le journal direct du dépôt, alors il ne montre aucune ligne issue du passé.
  - [ ] Étant donné un dépouillement interrompu par la fermeture de l'application alors que les réglages affichaient « 120 journaux sur 400 », quand je rouvre l'application, alors l'avancement repart d'un nombre supérieur ou égal à 120, jamais de zéro.
  - [ ] Étant donné un agent qui écrit pendant que le dépouillement tourne, quand je regarde la carte, alors sa zone s'allume en moins de 5 secondes comme d'habitude.
  - [ ] Étant donné un Mac où Claude Code n'a jamais tourné, donc sans aucun journal, quand j'ouvre l'application, alors les réglages annoncent tout de suite qu'il n'y a rien à dépouiller, et l'application reste utilisable.
  - [ ] Étant donné des journaux qui parlent d'un dépôt supprimé du disque, quand le dépouillement se termine, alors il s'annonce terminé sans erreur, et les zones des dépôts encore présents portent bien leurs dates.
  - [ ] Étant donné un dépouillement mené à son terme, puis l'application quittée, puis un agent lancé au terminal qui écrit dans une zone pendant ce temps, quand je rouvre l'application, alors le dépouillement reprend et annonce un nombre de journaux à dépouiller bien inférieur au total du premier passage, et la zone touchée pendant la fermeture porte sa nouvelle date d'écriture.
  - [ ] Étant donné ce même scénario mené avec le binaire en ligne de commande laissé en marche pendant la fermeture, qui a donc fait avancer la position de lecture du direct, quand je rouvre l'application, alors le dépouillement en arrière reprend à sa propre marque et rend leurs dates aux zones touchées pendant la fermeture, et la carte ne rejoue aucune activité déjà passée en direct.
  - [ ] Étant donné une zone écrite ce matin, dont la carte porte donc une date d'écriture de ce matin, quand je lance un dépouillement qui couvre un journal d'il y a un mois où cette même zone était écrite, alors, une fois le dépouillement terminé, la date de cette zone est toujours celle de ce matin : elle n'a pas reculé d'un mois (FR-088).
  - [ ] Étant donné une application fermée depuis une semaine, sans binaire en ligne de commande lancé pendant ce temps, et un agent qui a travaillé au terminal il y a cinq jours, quand je rouvre l'application, alors le journal direct du dépôt ne montre aucune ligne d'il y a cinq jours et aucune zone ne s'allume pour ce travail (FR-089) ; et la zone touchée porte bien sa date d'écriture d'il y a cinq jours, rendue par le dépouillement en arrière.
- **Hors scope** : profondeur au-delà de 30 jours, reconstitution du coût et des jetons passés, reconstitution des sessions passées, dépouillement des journaux d'une autre machine.

### F10 — Obtenir l'application et savoir laquelle on a (Priorité : P2)

- **User story** : En tant que développeur, je veux télécharger l'application depuis un emplacement stable et voir quelle version j'utilise afin de pouvoir l'installer sur un Mac neuf et dire ce que j'exécute quand quelque chose cloche.
- **Pourquoi cette priorité** : le critère de succès « sur un Mac neuf, moins de deux minutes » suppose quelque chose à télécharger. Sans publication, l'application n'existe que sur le poste qui la compile, et FR-025 pointe vers le vide.
- **Exigences** :
  - **FR-067** : QUAND une version est marquée d'une étiquette de publication, l'intégration continue DOIT produire et publier une application macOS téléchargeable, prête à être glissée dans les Applications.
  - **FR-068** : Le système DOIT afficher, depuis l'application elle-même, le numéro de version qu'elle porte, dans son écran Réglages, à côté du nom de la machine.
  - **FR-069** : L'application publiée DOIT embarquer le lecteur de dossiers et l'interface de la même version : aucun morceau ne DOIT être téléchargé au premier lancement.
  - **FR-079** : L'application publiée DOIT viser les Mac Apple Silicon, et elle seule : aucune version Intel n'est produite. La même publication DOIT continuer de porter, à côté d'elle, le binaire en ligne de commande pour macOS Apple Silicon, qui reste le compagnon de l'application sur ce même poste. Le binaire pour Linux x86_64, lui, NE DOIT PLUS être publié : le chemin par code ayant disparu (FR-082), aucun poste Linux neuf ne peut plus franchir l'entrée, et publier un binaire dont l'entrée est fermée serait publier une impasse.
- **Critères d'acceptation** :
  - [ ] Étant donné une étiquette de publication poussée, quand l'intégration continue a fini, alors la page des publications porte une application macOS Apple Silicon téléchargeable pour cette version et, à côté d'elle, l'archive du binaire en ligne de commande pour macOS Apple Silicon avec sa somme de contrôle, et aucune archive pour Linux.
  - [ ] Étant donné cette application téléchargée sur un Mac neuf, quand je la glisse dans les Applications et l'ouvre, alors elle démarre sans rien télécharger d'autre - vérifiable en coupant le réseau après l'installation : l'application s'ouvre et annonce seulement que la base est injoignable, comme FR-085 l'exige.
  - [ ] Étant donné l'application ouverte, quand j'ouvre son écran Réglages, alors le numéro de version y figure à côté du nom de la machine, et c'est celui de la publication téléchargée.
- **Hors scope** : la signature et la notarisation Apple, la mise à jour automatique, un cask Homebrew, la publication pour Windows ou Linux.

## Hors scope global

> **À ratifier par le PM avant le passage de ce PRD en « validé ».** Le retrait de la publication du binaire en ligne de commande pour Linux (point suivant, FR-079) est une décision prise **par ce PRD**, pas par le PM : son brief la listait parmi les points ouverts. Tant qu'il ne l'a pas ratifiée explicitement, ce retrait de portée reste une proposition, et ce PRD ne peut pas passer en « validé ». Refus du PM : FR-079 redevient la publication des deux cibles, et le hors-scope perd son deuxième point - rien d'autre ne bouge dans ce document.

- Windows et Linux : l'application de bureau vise macOS.
- **Le binaire en ligne de commande pour Linux cesse d'être publié. C'est un retrait de portée assumé, et il doit être lu comme tel** - sous réserve de la ratification demandée ci-dessus. Le produit devient une application macOS ; plus rien n'appelle le chemin d'appairage par code, ni à l'écran ni en coulisses (FR-082). Un poste Linux neuf n'aurait donc plus aucune manière de se relier, et publier un binaire dont l'entrée est fermée reviendrait à publier une impasse. Les postes Linux déjà reliés gardent leur jeton et continuent d'être acceptés (FR-026), mais aucun poste Linux neuf ne peut plus l'être. Le binaire survit sur macOS, comme compagnon de l'application : il ne s'appaire plus seul, il lit la configuration et le jeton que l'application a écrits.
- La signature et la notarisation Apple. L'avertissement du premier lancement d'une application non signée est assumé pour ce chantier.
- Une base de données locale embarquée dans l'application : deux sources de vérité, chaque règle écrite deux fois, catalogue indisponible dès que le Mac s'éteint.
- Le démarrage automatique du lecteur à l'ouverture de session, et tout suivi pendant que l'application est fermée.
- Lancer, arrêter ou piloter un agent depuis l'application : la carte observe.
- Le réglage des cadences, des seuils et de la fenêtre d'activité depuis un écran.
- La conservation de 30 jours d'événements bruts : seules deux dates par zone survivent.
- Toute modification de ce que la carte mesure et de la manière dont elle le colore.
- Le reliquat de PRD-001 (issues #41, #42, #43), indépendant de ce chantier.

## Critères de succès mesurables

- Sur un Mac neuf : de l'ouverture de l'application à une carte peuplée, **moins de deux minutes**, sans terminal, sans code à recopier, sans aucun fichier ouvert à la main.
- Aux lancements suivants : double-clic, puis la carte. **Aucune autre étape.**
- Ajouter un dossier surveillé prend **moins de trente secondes** de manipulation, et ses dépôts sont visibles en **moins d'une minute**.
- Après la fermeture de l'application, **aucun processus de lecture ne subsiste**, vérifiable dans le Moniteur d'activité.
- L'application fermée, le catalogue reste consultable depuis un autre appareil, **daté** de sa dernière cartographie.
- Sur un Mac dont les journaux remontent à deux mois, le premier dépouillement rend leurs deux dates aux zones travaillées dans les 30 derniers jours **sans allumer une seule couleur**.
- La liste fermée de ce qui sort de la machine **ne gagne aucune ligne**.

## Hypothèses

- Un seul utilisateur, un seul Mac, la quasi-totalité du travail en local.
- La base est hébergée au moment où l'application est utilisée pour de bon. Sans hébergement, le catalogue ne survit pas à l'extinction du poste, et la promesse « consultable depuis un autre appareil » tombe.
- Les journaux de Claude Code restent à leur emplacement actuel et gardent leur format. Le dépouillement en arrière repose entièrement dessus.
- L'utilisateur accepte l'avertissement de première ouverture d'une application non signée.
- Le trousseau du système reste accessible. S'il refuse, l'application le dit au lieu de se taire.
- L'écran affiché dans la fenêtre de bureau est bâti à partir du même code que celui du web : une seule interface à maintenir, servie ici depuis la machine plutôt que depuis l'hébergeur.
- Un dépôt reste un dossier contenant `.git`, et c'est `git` qui décide de ce qui existe à l'intérieur.

## Décisions d'implémentation

**L'application est écrite en Rust et embarque le lecteur existant comme bibliothèque**, jamais comme code recopié. Cartographie, lecture des journaux, file d'attente, envoi : une seule implémentation, partagée. Tauri est le véhicule pressenti ; tout autre choix devra offrir la même propriété, sans quoi la décision de l'ADR 0001 - deux langages à tenir d'accord, pas trois - serait perdue.

**La fenêtre est servie depuis une origine locale, par l'application elle-même.** L'interface web ne peut pas être figée en pages statiques : ses écrans sont des composants serveur asynchrones qui lisent la session dans les cookies, redirigent, appellent des fonctions de la base au moment de la requête, et l'échange du code d'autorisation GitHub passe par un gestionnaire de route côté serveur. Elle exige donc un serveur. L'application embarque ce service d'interface comme exécutable compagnon, le démarre à l'ouverture sur `127.0.0.1` au **port fixe `51789`**, et l'arrête à la fermeture, exactement comme le lecteur. Aucune page distante n'est chargée dans la fenêtre.

**Le port est fixe, jamais tiré au hasard au démarrage.** Trois choses en dépendent : l'URL de retour d'autorisation, que le fournisseur d'identité n'accepte que si elle figure dans sa liste blanche - un port éphémère la rendrait invalide à chaque lancement ; le cookie de session, lié à l'origine et donc au port, sans quoi la promesse « ne plus jamais se reconnecter » tomberait au premier redémarrage ; et la reproductibilité du cas « le port est déjà pris », qui ne se teste que sur un port connu. `51789` est choisi dans la plage dynamique, hors des ports d'usage courant des outils de développement. S'il est occupé, l'application le dit et propose de réessayer : elle ne se rabat pas sur un autre port, car ce serait casser la session et l'URL de retour en silence.

Ce choix coûte un exécutable de plus dans le paquet ; il achète une seule origine, donc un pont vers le disque sûr par construction, sans aucune autorisation accordée à un domaine distant, et une fenêtre qui s'ouvre et s'explique même sans réseau, au lieu de rester blanche. L'option inverse - pointer la fenêtre sur le site hébergé et ouvrir le pont à cette seule origine distante - est écartée : elle ferait d'une page distante compromise un exécutant de commandes locales.

**Le pont entre la fenêtre et l'application expose sept commandes locales nommées** :

- *lister les dossiers surveillés* - pour chacun, son chemin, son compte de dépôts à la dernière cartographie, son état de lisibilité et son état d'autorisation ;
- *ajouter un dossier* par le sélecteur du système ;
- *retirer un dossier* ;
- *redemander l'autorisation d'un dossier* auquel le système refuse l'accès ;
- *lire l'état du lecteur* - en marche, arrêté, en échec, sa raison d'échec, et si le verrou est détenu par un tiers ;
- *relancer le lecteur* ;
- *lire le contexte* - avancement et fin du dépouillement, indication qu'une cartographie est en train de se faire, indication qu'une cartographie a abouti sans trouver aucun dépôt (FR-087), version de l'application, nom et identifiant de la machine, état de sa révocation.

L'heure de la dernière cartographie, elle, ne passe pas par ce pont : elle vit déjà en base, avec le catalogue, et l'écran la lit là comme il le fait aujourd'hui (FR-059). C'est ce qui la rend lisible depuis un autre appareil et par-delà les fermetures de l'application. Elle est un agrégat, et l'écran l'établit comme tel : la plus récente des heures de cartographie des dépôts de cette machine (FR-086). Ce qui emprunte le pont, ce sont en revanche les faits du poste que la base ne porte pas : l'état « une cartographie est en cours », et le fait qu'une cartographie a abouti sans trouver aucun dépôt (FR-087) - cas où la base, n'ayant aucun dépôt, n'a aucune heure à donner et où « jamais cartographié » serait faux.

La clôture ne porte pas sur le nombre de commandes, qui suivra les besoins des écrans, mais sur leur nature : **aucune commande de lecture ou d'écriture de fichier arbitraire**. Le pont ne sait pas ouvrir un chemin qu'on lui donne ; il sait seulement lire des faits déjà tenus par l'application et régler la liste des dossiers surveillés. Ces commandes passent par le pont interne de l'application, pas par une adresse que l'on pourrait appeler d'ailleurs, et ne répondent qu'à l'intérieur de sa fenêtre. La même interface, ouverte dans un navigateur ordinaire, ne trouve pas ce pont : elle détecte son absence et remplace la section des dossiers surveillés par une mention disant que ces réglages n'existent que dans l'application de bureau. Le reste de l'écran Réglages - les comptes, qui vivent en base - continue de s'y afficher normalement.

**Deux identités cohabitent dans la même application.** La fenêtre parle à la base avec la session de l'utilisateur ; le lecteur parle avec le jeton de la machine, qui ne vaut que pour elle et que la révocation coupe. Elles ne se mélangent pas, et le jeton de machine ne transite jamais par la fenêtre.

**La déclaration d'une machine remplace l'échange d'un code.** C'est la même mécanique - une fonction de la base crée la ligne de machine et signe son jeton - à ceci près que l'appelant présente désormais la session de l'utilisateur au lieu d'un code à usage unique. Les règles d'accès restent en RLS, et la révocation continue de couper les écritures.

**L'identité d'une machine est l'identifiant que la base lui a donné, conservé localement à côté du jeton.** Il n'y a pas d'empreinte matérielle, pas de nom d'hôte servant de clé : l'application présente à chaque lancement l'identifiant qu'elle a gardé, et c'est la base qui confirme. Quand cet identifiant ne désigne plus rien - base remise à zéro, machine supprimée -, l'application redéclare et le dit, plutôt que de battre dans le vide. Une configuration de ligne de commande déjà présente sur le poste est reprise pour son identifiant de machine et ses dossiers surveillés : c'est le chemin de migration, et il n'en existe pas d'autre. **Le jeton, lui, n'est pas dans ce fichier** : il vit au trousseau du système, sous le service `fr.yarma.vibemap` et sous le compte que forme l'identifiant de la machine. L'application le relit dans cette entrée-là, plutôt que d'en demander un neuf. Comme elle est un exécutable différent du binaire en ligne de commande, le système soumettra cet accès à l'autorisation de l'utilisateur une première fois : c'est attendu et non une anomalie, et un refus s'annonce avec un bouton pour réessayer, sans jamais entraîner une redéclaration silencieuse de la machine. **L'adresse de la base, elle, n'est pas reprise** : ce champ existe dans la configuration actuelle et pointe aujourd'hui sur la pile locale de développement, où l'application publiée n'a rien à faire. L'adresse de la base est fixée à la compilation du paquet, la même pour tous les postes, et une configuration héritée ne la détourne pas.

**L'autorisation GitHub s'ouvre dans le navigateur du système et revient sur l'origine locale fixe.** La fenêtre de l'application n'est pas un lieu d'autorisation : GitHub refuse les vues web embarquées. Le retour se fait sur `http://127.0.0.1:51789`, qui doit à ce titre figurer dans la liste blanche des URL de retour du projet Supabase - c'est la deuxième raison pour laquelle le port ne peut pas être tiré au hasard. À ce retour, l'application reprend le premier plan d'elle-même : l'utilisateur ne rouvre rien et ne recopie rien.

**Un seul lecteur par machine, garanti par un verrou pris sur le poste**, pas par une convention. Le verrou appartient au lecteur partagé, pas à l'un de ses véhicules : il est posé au démarrage du lecteur et relâché à son arrêt, donc aussi bien par l'application que par le binaire en ligne de commande, et la publication les porte à la même version. L'application qui le trouve déjà pris ne démarre pas de second lecteur et l'affiche. C'est ce qui fait tenir ensemble l'interdiction de deux lecteurs simultanés et la survie du binaire sur macOS, désormais tranchée : il reste le compagnon de l'application sur le même poste. La garantie a une borne, et le PRD la dit plutôt que de la taire : un lecteur d'une version antérieure au verrou ne pose aucune marque et n'est donc pas détectable.

**La liste des dossiers surveillés reste dans la configuration du lecteur**, qui devient un détail de stockage : l'écran Réglages en est l'unique éditeur, et le lecteur relit sa liste à chaud plutôt qu'au démarrage.

**La dernière touche vit dans sa propre table d'agrégat, de clé `(repo_id, chemin de zone)`, tenue à jour à l'écriture.** Elle n'est pas une colonne ajoutée à la carte des zones, et ce point n'est pas négociable : la cartographie efface en bloc toutes les lignes de zones d'un dépôt avant de les repousser, toutes les cinq minutes. Deux colonnes posées là seraient perdues au tour suivant, et les tests passeraient en local avant d'échouer en usage réel. Le cycle de vie de l'agrégat est donc indépendant de celui des zones : la cartographie ne le touche pas.

Une zone qui disparaît du dépôt disparaît de la carte, mais sa ligne d'agrégat survit : c'est un fait daté, pas un état, et un dossier renommé puis remis en place retrouve ses deux dates. Les lignes d'agrégat ne sont effacées qu'avec le dépôt auquel elles appartiennent.

**L'écriture dans cet agrégat est monotone : une date n'y recule jamais** (FR-088). C'est la conséquence directe du point précédent : le direct et le dépouillement en arrière écrivent au même endroit, mais le second parcourt du passé. Une écriture ne garde donc que la plus récente des deux dates en présence, celle qui arrive et celle qui est déjà là. Sans cette règle, le premier dépouillement ferait reculer d'un mois des dates fraîches, au moment exact où l'utilisateur découvre la fonctionnalité.

Ce n'est pas un choix de confort : les événements bruts sont purgés au bout de sept jours par une tâche planifiée déjà en place, donc trente jours d'événements n'existent pas et n'existeront pas. C'est une exception assumée à la règle « l'état ne se stocke pas, il se calcule à la lecture » : une date n'est pas un état, elle ne se périme pas d'elle-même. La fenêtre glissante de 30 jours, elle, se juge bien à la lecture, comme le figement et la fraîcheur.

**Le dépouillement en arrière est un canal distinct de la lecture vivante.** Il alimente les deux dates et n'écrit jamais dans le flux d'activité qui colore la carte et remplit le journal direct. C'est cette séparation, et elle seule, qui garantit qu'un mois de passé n'allume rien.

**Le dépouillement est incrémental et rejoue à chaque ouverture.** Il n'y a pas un « premier dépouillement » suivi de rien : à chaque ouverture, l'application reprend là où le dépouillement précédent s'est arrêté et ne relit jamais ce qui l'a déjà été. Une marque de progression conservée sur le poste sert les deux cas d'un seul mécanisme : la reprise après une interruption, et le rattrapage de ce qu'un agent lancé au terminal a écrit pendant que l'application était fermée. Le premier lancement n'est que le cas particulier où cette marque n'existe pas encore, et où il y a donc trente jours à parcourir.

**Cette marque est distincte de la position de lecture qui sert le direct**, que le lecteur tient déjà à côté de sa configuration. Les deux avancent à leur rythme et ne s'écrivent jamais l'une l'autre : le direct consomme les journaux au fil de l'eau pour allumer la carte, le dépouillement les relit pour n'en tirer que deux dates. Un binaire en ligne de commande laissé en marche pendant que l'application est fermée fait avancer la première et pas la seconde - et c'est précisément ce qui permet au dépouillement de rattraper ses dates à l'ouverture suivante sans que la carte rejoue un direct déjà passé.

**Le cas inverse - personne n'a tourné pendant la fermeture - demande une borne de plus** (FR-089). La position de lecture du direct est alors restée à l'avant-fermeture, et la reprise lui ferait consommer plusieurs jours de journaux d'un coup, qui partiraient en direct. La fenêtre de rattrapage du direct existe déjà, mais elle ne s'applique aujourd'hui qu'aux journaux jamais vus ; un journal déjà connu de la position n'est pas borné. Elle doit s'appliquer à tous. Les couleurs, elles, ne risquent rien : l'état des modules se calcule sur une fenêtre glissante et ignore d'office ce qui est trop ancien. Ce qui risque, c'est le journal direct, qui lit les événements les plus récents sans borne de fenêtre - et les sessions qu'ils feraient réapparaître.

**Le démarrage automatique par le service de session est retiré du produit sur macOS**, puisque le lecteur vit et meurt avec l'application.

**L'application se publie à l'étiquette, en un seul paquet**, par la chaîne d'intégration continue déjà en place pour le binaire. Le paquet porte le lecteur, le service d'interface et la fenêtre, tous à la même version, et cette version est lisible depuis l'application. Une seule cible : les Mac Apple Silicon, la seule cible macOS que la chaîne compile déjà. Aucune version Intel n'est produite. Le binaire en ligne de commande pour macOS Apple Silicon continue d'être publié à côté de l'application, dont il devient le compagnon sur le même poste ; sa cible Linux, elle, sort de la publication, faute d'entrée qu'un poste Linux neuf puisse encore franchir - retrait de portée assumé, énoncé dans « Hors scope global ». Rien n'est téléchargé au premier lancement : un Mac neuf, réseau coupé juste après l'installation, doit pouvoir ouvrir l'application et ne se plaindre que de la base.

**Rien de nouveau ne sort de la machine** : les deux dates voyagent avec des chemins de zone relatifs et des horodatages, la déclaration d'une machine avec un nom et une plateforme. Aucune table ne gagne de colonne où un contenu, un prompt ou un chemin absolu pourrait entrer.

## Décisions de test

**Ce qui fait un bon test ici** : un comportement observable depuis l'extérieur - une machine vue ou muette, un dépôt présent ou absent du catalogue, une date affichée ou remplacée par « rien de récent », une couleur allumée ou éteinte. Jamais la forme interne d'un agrégat ou l'ordre des appels.

**Modules profonds testables en isolation** :

- *La liste des dossiers surveillés* - lire, ajouter, retirer, signaler un dossier illisible, et surtout la règle du doublon : déplier le `~`, résoudre les liens symboliques, normaliser la barre finale, refuser un dossier contenu dans un dossier surveillé ou le contenant, et nommer le coupable dans le refus. Fonction pure sur des dossiers temporaires. Prior art : `daemon/tests/config.rs`, qui éprouve déjà le chargement de la configuration et la qualité de ses messages d'erreur.
- *Le dépouillement des journaux* - fonction pure sur du texte, sans réseau, avec des journaux réels versionnés comme échantillons. Prior art direct : `daemon/tests/journal.rs`, qui fabrique des lignes d'assistant et vérifie ce qu'on en tire.
- *La marque de progression du dépouillement* - fonction pure : à partir d'une marque et d'un ensemble de journaux datés, dire lesquels restent à dépouiller. Aucun ne doit revenir deux fois ; un journal écrit après la marque doit revenir ; une marque absente doit rendre les trente derniers jours et rien de plus. C'est ce qui rend observable la promesse « jamais deux fois le même journal ».
- *L'héritage d'une zone parente* - la dernière touche d'une zone est le plus récent de ce qui la vise et de ce qui vise les zones qu'elle contient, jugé par préfixe de chemin **à la lecture**, jamais par des lignes d'ancêtre tenues à l'écriture. Prior art direct et à reprendre tel quel : `supabase/migrations/20260804000001_activite.sql`, où l'état des modules fait déjà exactement cet héritage par `starts_with(r.module_path, m.path || '/')`, avec le cas particulier des parcelles en « . » qui ne reçoivent rien de leurs sous-dossiers. Un seul enregistrement par zone touchée, donc aucune dérive possible entre une zone et ses parents.
- *La règle des 30 jours et la mise en mots des deux dates* - fonctions pures recevant un `maintenant` en paramètre, jamais l'horloge réelle. Prior art : `web/lib/figement.test.ts` et `web/lib/fraicheur.test.ts`, qui appliquent déjà exactement cette discipline.
- *La déclaration d'une machine et sa RLS* - test d'intégration contre la vraie pile Supabase locale : une machine déclarée deux fois ne se dédouble pas, une machine révoquée n'écrit plus. Prior art : `daemon/tests/appairage.rs` et le contexte partagé de `daemon/tests/common`.
- *La non-contamination de la carte par le dépouillement* - test d'intégration : dépouiller un journal ancien, puis vérifier que l'état des modules ne rend rien et que les deux dates, elles, sont là. Prior art : `daemon/tests/activite.rs`.
- *La survie des deux dates à une cartographie* - test d'intégration, et c'est le plus important du lot : poser les deux dates d'une zone, relancer une cartographie complète du dépôt, relire les dates. Le test échoue si l'agrégat vit dans la table des zones. Même prior art que la cartographie elle-même, contre la pile Supabase locale.
- *La monotonie des deux dates* (FR-088) - test d'intégration, à ranger juste après le précédent par ordre d'importance : poser une date fraîche sur une zone, y appliquer une écriture datée d'un mois plus tôt, relire. La date fraîche doit tenir. Le test échoue si l'agrégat est écrit par simple remplacement, et il tombe exactement sur le scénario du premier dépouillement.
- *La borne de rattrapage du direct* (FR-089) - fonction pure : à partir d'une position de lecture, d'un ensemble de journaux et d'un horizon, dire quels événements partent en direct. Un événement plus ancien que l'horizon ne doit jamais en sortir, que son journal ait déjà été vu ou non. C'est ce qui rend observable la promesse « le passé n'est jamais rejoué en direct ». Prior art : `daemon/tests/journal.rs`.
- *La règle du lecteur unique* - test sur le verrou seul : le prendre, tenter de le reprendre, vérifier le refus et le message ; le relâcher, vérifier que la reprise passe. Fonction pure sur un fichier temporaire, comme la configuration.

**Ce qui ne se vérifie qu'à la main** : la fenêtre elle-même, le sélecteur de fichiers du système, l'ouverture de la page GitHub, la persistance de la session, l'absence de processus résiduel après fermeture. Une courte liste de vérifications manuelles accompagne la livraison, et elle est jouée avant chaque fusion touchant l'application de bureau.

**Ce que joue l'intégration continue** : seulement ce qui n'a besoin de personne - clippy, les tests hors réseau, le lint et le build web. Les tests qui touchent la base se lancent à la main avant chaque fusion, comme aujourd'hui.

## Risques

**Un seul objet à installer, donc un seul objet à casser.** Aujourd'hui, un web en panne laisse le lecteur collecter. Demain, une application qui refuse de s'ouvrir arrête aussi la collecte. La contrepartie de « un seul geste » est « un seul point de défaillance ».

**Le suivi s'arrête quand l'application est fermée.** Un agent lancé au terminal pendant ce temps ne laissera aucune couleur. Le dépouillement en arrière rattrapera ses dates au lancement suivant, jamais son direct. C'est cohérent avec le modèle retenu - le vivant ne compte que quand on regarde - mais c'est une perte réelle par rapport au démarrage automatique jamais installé.

**Une application non signée effraie.** Le premier lancement affiche un avertissement du système, contournable mais déconcertant. Décision repoussée, et donc risque accepté tel quel pour ce chantier.

**L'autorisation sort de l'application le temps d'un aller-retour.** GitHub refuse les vues web embarquées : la page d'autorisation s'ouvre donc dans le navigateur du système, et le geste que ce PRD cherche à raccourcir gagne un changement d'application. La contrepartie est acceptée parce qu'elle ne coûte qu'une fois, au premier lancement, et parce que le retour ramène l'utilisateur dans la fenêtre sans qu'il ait à la rouvrir. Le risque résiduel est le retour lui-même : s'il n'aboutit pas sur l'origine locale fixe, il faudra un schéma d'adresse propre à l'application, ce que l'À CLARIFIER de F3 garde ouvert.

**Le port fixe est un pari sur la disponibilité.** Un port arrêté rend la session et l'URL de retour stables, mais il fait dépendre le démarrage d'une ressource qu'un autre programme peut prendre. Le choix assumé est de le dire et de s'arrêter plutôt que de glisser sur un autre port en silence : une session perdue et un retour d'autorisation invalide seraient bien plus déroutants qu'un message.

**Deux dates stockées, c'est la première entorse à « l'état ne se stocke pas ».** Elle est justifiée et bornée, mais elle ouvre une porte : la prochaine demande d'agrégat s'y appuiera. Il faudra la refuser sur ses propres mérites, pas sur le précédent.

**Le paquet grossit d'un service d'interface.** Embarquer l'interface plutôt que pointer sur le site hébergé ajoute un exécutable et son moteur au paquet, et un troisième processus à surveiller au démarrage. C'est le prix assumé d'une origine unique et locale : aucun domaine distant ne reçoit l'autorisation d'appeler les commandes du disque. Le risque n'est pas la taille, c'est un service d'interface qui refuse de démarrer et laisse la fenêtre blanche - d'où FR-004.

**Le premier dépouillement lit beaucoup.** Plus d'un gigaoctet de journaux sur le poste actuel. Mal borné, il fera chauffer le Mac au moment précis où l'utilisateur découvre l'application. La reprise et le non-blocage du direct ne sont pas des raffinements.

**La démonstration est à remonter de zéro.** La base locale ne contient plus que des déchets de tests, et l'identifiant de machine de la configuration actuelle n'y existe plus. Le premier essai de bout en bout se fera donc sur une base neuve, ce qui éprouve le parcours de premier lancement pour de vrai - et le rend obligatoire avant toute validation.

## Notes complémentaires

Faits vérifiés dans le code, dont deux corrigent le brief de décisions :

- **La cartographie efface toutes les lignes de zones d'un dépôt avant de les repousser**, toutes les cinq minutes. C'est ce fait, vérifié dans le lecteur, qui interdit de ranger les deux dates dans la carte des zones et impose une table d'agrégat à part.
- **L'interface web ne peut pas être servie en pages figées** : ses écrans sont des composants serveur asynchrones qui lisent la session dans les cookies et appellent la base au moment de la requête, un écran redirige, un autre dépend d'un identifiant dans l'adresse, et le retour d'autorisation GitHub passe par un gestionnaire de route serveur. L'application de bureau doit donc embarquer un service d'interface, et non un dossier de fichiers.
- **Les événements d'activité sont purgés au bout de sept jours**, par une fonction dédiée planifiée quotidiennement. Le brief présentait l'agrégat de deux dates comme une recommandation d'architecture ; le code en fait une nécessité. Trente jours d'événements bruts n'existent pas.
- **Le catalogue ne dépend pas du lecteur.** La liste des dépôts est lue dans la base et ne filtre pas sur la vivacité de la machine ; l'accueil affiche déjà depuis quand une machine se tait. Ce que le PM a vécu - « on perd l'existence même des repos » - vient de la base locale éteinte et de l'identifiant de machine effacé par une remise à zéro, pas d'un couplage du produit. L'hébergement de la base est donc bien le remède, et aucun écran n'a besoin d'être refait pour ce point : c'est pourquoi ce PRD n'y consacre aucune feature.
- **La cartographie n'examine que les enfants directs d'un dossier surveillé** : un dossier qui est lui-même un dépôt n'est pas vu. Ce détail devient visible dès qu'on ouvre le choix d'un dossier au sélecteur du système : FR-076 impose de le dire à l'utilisateur au lieu d'afficher zéro dépôt sans explication, et l'À CLARIFIER de F7 garde ouverte la seule question de la profondeur.
- **Le fichier de démarrage automatique existe dans le dépôt mais n'a jamais été installé.** Ce PRD le retire du produit (FR-011) plutôt que de le laisser en attente.
- **Le fichier de configuration reste le support de stockage** des dossiers surveillés, conformément à la décision D4 : ce qui change, c'est qu'il cesse d'être un point de passage pour l'utilisateur.
- **L'écran de connexion demande son retour d'autorisation sur l'origine de la page elle-même.** C'est ce fait qui rend le port du service d'interface structurant : l'origine de la fenêtre devient l'URL de retour, et cette URL doit figurer dans la liste blanche du projet Supabase. Un port tiré au hasard au démarrage la rendrait invalide à chaque lancement, et emporterait au passage le cookie de session, lié à l'origine et donc au port. D'où FR-070 et le port arrêté à `51789`.
- **L'adresse de la base est un champ obligatoire de la configuration de ligne de commande**, et il pointe aujourd'hui sur la pile locale du poste. Une reprise naïve de cette configuration ferait donc parler l'application publiée à une base de développement : d'où FR-073, qui exclut ce champ de la migration et fixe l'adresse à la compilation.
- **La chaîne de publication compile aujourd'hui deux cibles** : macOS Apple Silicon et Linux x86_64, pas de cible macOS Intel. L'application de bureau se range sur la seule cible macOS existante, et l'archive du binaire en ligne de commande pour macOS reste publiée à côté d'elle. La cible Linux, elle, sort de la publication (FR-079) : c'est le retrait de portée que le PM doit lire **et ratifier**, et il est écrit noir sur blanc en tête de « Hors scope global ».
- **Le sort du binaire en ligne de commande est tranché**, et ne figure plus parmi les points ouverts : il survit sur macOS comme compagnon de l'application. Il ne relie plus une machine par lui-même (FR-082) ; il lit la configuration et le jeton que l'application a écrits sur le poste, et pose le même verrou qu'elle (FR-081). Sa formule Homebrew continue de le servir sur macOS et perd son volet Linux, dont l'archive n'est plus publiée : c'est pourquoi `Formula/` figure dans les zones touchées.
- **La cartographie a lieu dès le démarrage du lecteur, avant sa première boucle**, puis toutes les cinq minutes - et non au premier intervalle seulement, contrairement à ce que ce PRD affirmait auparavant. Surtout, l'heure de la dernière cartographie est écrite en base avec chaque dépôt : elle survit à la fermeture de l'application, et se lit depuis un autre appareil. L'écran Réglages la lit donc là (FR-059), et ne dit « jamais cartographié » que sur un poste où aucune cartographie n'a jamais abouti (FR-074) - jamais parce qu'un nouveau lancement vient de commencer.
- **Une page de connexion locale non suivie par git** existe sur le poste pour ouvrir une session contre la pile Supabase locale. C'est un outil de développement, hors du parcours décrit ici ; elle ne doit ni être distribuée, ni servir de modèle au parcours de connexion.

Points ouverts hérités du brief de décisions, tous marqués À CLARIFIER dans les features concernées ou repris ici :

- **Hébergement de la base** : quel projet Supabase, quelle région, qui crée le compte. Rien dans ce PRD ne dépend du choix, mais la promesse « le catalogue survit à l'app fermée » n'est tenue qu'une fois l'hébergement en place. Quel que soit le projet retenu, sa liste blanche d'URL de retour devra porter `http://127.0.0.1:51789/auth/callback`, sans quoi F3 ne peut pas aboutir.
- **Signature Apple** : hors scope de ce chantier, à trancher avant toute distribution plus large qu'un poste.
- **Profondeur d'exploration d'un dossier surveillé**, et sort des dépôts d'un dossier qu'on cesse de surveiller : voir les deux À CLARIFIER de F7.
- **Véhicule du retour d'autorisation** si l'origine locale fixe est infirmée à l'essai : voir l'À CLARIFIER de F3.
- **Retrait de la publication du binaire pour Linux** : le brief le rangeait parmi les points ouverts, ce PRD le tranche par FR-079. La décision attend donc la ratification explicite du PM avant le passage en « validé » : voir la note en tête de « Hors scope global ».
