# Vérifications manuelles - application de bureau

Une fenêtre, un menu, une barre d'adresse absente : rien de tout cela ne se
vérifie sans un écran. Cette liste est ce que le PRD-002 impose de jouer **avant
chaque fusion touchant `bureau/`**. Elle s'allonge avec les tranches ; #57 l'a
commencée, #58 y ajoute la géométrie de la fenêtre, #61 le lecteur embarqué,
#62 son état affiché et sa relance, #68 l'ouverture du pont à l'interface, #70
l'ajout d'un dossier au sélecteur du système.

Ce qui est déjà couvert par les tests automatiques - la sonde de disponibilité,
l'URL fixe de la fenêtre, la lecture et l'écriture de la géométrie, la prise et
la libération du verrou du poste par le lecteur embarqué, le refus d'un second
lecteur et sa mise en mots, l'état montré à chaque lecture, la relance après un
arrêt, les origines auxquelles le pont s'ouvre et ce qu'il rend des dossiers
surveillés - n'a pas à être rejoué ici : `cargo test` dans `bureau/` s'en
charge.

## Avant de commencer

```sh
cd web && npm install && npm run build   # l'interface que l'application sert
cd ../bureau && cargo run
```

L'application sert `web/` construit, pas le serveur de développement : un
`npm run dev` ouvert à côté ne la gêne pas, et le service qu'elle démarre est
le même que celui du site.

## À jouer

- [ ] **La carte s'affiche.** Au lancement, la fenêtre montre l'interface en
      moins de 5 secondes, sans barre d'adresse.
- [ ] **Rien ne mène ailleurs.** Menus, raccourcis clavier, clic droit : aucune
      commande ne permet de charger une autre page que Vibe Map.
- [ ] **Le port pris s'annonce.** Avec `python3 -c 'import socket,time;
      s=socket.socket(); s.bind(("127.0.0.1",51789)); s.listen(); time.sleep(120)'`
      lancé d'abord, l'application affiche un message nommant le port 51789 et un
      bouton « Réessayer », au lieu d'une fenêtre blanche - et aucun service
      d'interface n'est démarré ailleurs.
- [ ] **Réessayer suffit.** Le port libéré, un clic sur « Réessayer » affiche la
      carte, sans quitter l'application.
- [ ] **La même adresse à chaque lancement.** Quitter et rouvrir trois fois ;
      à chaque fois, `http://127.0.0.1:51789` répond dans Safari.
- [ ] **Rien ne vient du site hébergé.** Dans l'inspecteur web de la fenêtre,
      au chargement : aucune page ni aucun script venu d'un domaine distant.
      Seuls les appels à la base sortent de la machine.
- [ ] **Rien ne survit.** Après avoir quitté l'application,
      `lsof -nP -iTCP:51789 -sTCP:LISTEN` ne rend rien.
- [ ] **La fenêtre se retrouve où on l'a laissée.** Redimensionner et déplacer
      la fenêtre, quitter, rouvrir : elle revient à la même taille et au même
      endroit. Ce qu'elle a retenu se lit dans
      `~/Library/Application\ Support/fr.yarma.vibemap.bureau/fenetre.json`.
- [ ] **Un poste neuf s'ouvre proprement.** Après
      `rm -rf ~/Library/Application\ Support/fr.yarma.vibemap.bureau`, la
      fenêtre s'ouvre à 1280 × 860, placée par le système, sans message d'erreur.
- [ ] **Un état abîmé ne bloque rien.** Avec
      `printf '{"largeur": 12' > ~/Library/Application\ Support/fr.yarma.vibemap.bureau/fenetre.json`,
      l'application s'ouvre quand même, à la géométrie par défaut, et réécrit un
      fichier valide en quittant.

## Le lecteur (#61)

Ces vérifications demandent une machine reliée (`vibemap pair <code>`) et une
pile Supabase joignable. Le verrou du poste vit à
`~/.config/vibemap/lecteur.lock` ; `lsof ~/.config/vibemap/lecteur.lock` dit qui
le tient.

- [ ] **La machine bat dès l'ouverture.** L'application ouverte depuis une
      minute, la liste des machines consultée depuis un autre appareil montre
      cette machine vue il y a moins de 90 secondes.
- [ ] **Rien ne survit à la fermeture.** Quitter l'application : le Moniteur
      d'activité ne montre plus aucun processus de lecture,
      `lsof ~/.config/vibemap/lecteur.lock` ne rend rien, et la machine passe
      muette au bout de 90 secondes.
- [ ] **Un signal ne laisse rien derrière.** L'application ouverte,
      `kill -TERM <pid de Vibe Map>` : `lsof ~/.config/vibemap/lecteur.lock` ne
      rend rien ensuite. Même chose avec `kill -9`.
- [ ] **Deux ouvertures, un seul lecteur.** L'application déjà ouverte, la
      relancer depuis le Finder : un seul lecteur tourne.
- [ ] **Le poste tenu s'annonce.** Avec `vibemap` lancé au terminal, ouvrir
      l'application : la fenêtre dit qu'un lecteur tourne déjà, nomme
      « vibemap » et son processus, et propose « Afficher la carte ». Le
      Moniteur d'activité n'en montre qu'un seul.
- [ ] **Et l'inverse.** L'application ouverte, lancer `vibemap` au terminal : il
      refuse en nommant « l'application de bureau ».
- [ ] **Aucun démarrage automatique.** Session du Mac ouverte, application
      jamais lancée depuis le démarrage : au bout de deux minutes, aucun
      processus de lecture ne tourne et la machine reste muette.
- [ ] **Le poste tenu se relance une fois l'autre arrêté.** Dans la suite du
      point précédent, `vibemap` toujours lancé au terminal : l'arrêter par
      Ctrl-C, puis cliquer sur « Relancer le lecteur » dans la fenêtre. Elle dit
      « Le lecteur tourne », `lsof ~/.config/vibemap/lecteur.lock` nomme
      l'application, et la machine est de nouveau vue il y a moins de
      90 secondes.
- [ ] **Le refus du trousseau se dit.** Après une recompilation
      (`cargo build` puis `cargo run`), macOS redemande l'autorisation
      d'accéder au jeton : refuser la boîte de dialogue. La fenêtre affiche
      « Le jeton de cette machine n'a pas pu être lu », la raison rendue par le
      système, et un bouton « Relancer le lecteur ». Le rejouer en accordant
      cette fois l'autorisation : le lecteur repart sans quitter l'application.
- [ ] **L'attente du trousseau se voit.** Même situation, boîte de dialogue
      laissée ouverte : la fenêtre dit « Démarrage du lecteur… » et propose
      « Afficher la carte » plutôt que de rester muette.
- [ ] **Le pont ne répond qu'à la fenêtre.** Ouvrir
      `http://127.0.0.1:51789` dans Safari, puis dans la console :
      `window.__TAURI__` est `undefined`, et aucune commande du pont n'est
      joignable. L'origine est la même que celle de la fenêtre, et c'est bien
      cela qu'il faut vérifier : le pont s'ouvre à une origine **dans le
      webview de l'application**, jamais à un navigateur qui charge la même
      adresse.

## Le pont, l'écran Réglages et le bandeau (#68)

Ces vérifications demandent une machine reliée, une pile Supabase joignable et
une cartographie déjà passée. La configuration du lecteur vit à
`~/.config/vibemap/config.toml` ; ses `roots` sont les dossiers surveillés.

- [ ] **Les dossiers surveillés s'affichent.** Dans la fenêtre, ouvrir
      Réglages : chaque dossier de `roots` est listé, avec le nombre de dépôts
      trouvés. Le compte doit être celui des enfants **directs** portant un
      `.git` - `ls -d ~/Developer/*/.git | wc -l` donne le même nombre.
- [ ] **Un dossier renommé se signale.** Renommer un dossier surveillé
      (`mv ~/Developer ~/Developer-renomme`), rouvrir Réglages : sa ligne porte
      « Dossier introuvable », sans compte, et les autres dossiers restent
      affichés normalement. Remettre le nom ensuite.
- [ ] **Le même écran dans Safari.** Ouvrir `http://127.0.0.1:51789/reglages`
      dans Safari : la section des dossiers est remplacée par la mention
      « Ces réglages n'existent que dans l'application Vibe Map », les comptes
      s'affichent normalement, l'heure de la dernière cartographie aussi - et
      aucune erreur ni liste vide n'apparaît.
- [ ] **Rien ne sort de la machine.** Dans l'inspecteur web de la fenêtre,
      onglet Réseau, en ouvrant Réglages : aucune requête ne porte un chemin de
      dossier ni un compte de dépôts. Les seuls appels sortants sont ceux de la
      base, pour les comptes et `repos`.
- [ ] **Le bandeau du lecteur par-dessus la carte.** L'application ouverte sur
      la carte, lancer `vibemap` au terminal après avoir arrêté le lecteur de
      l'application (ou tuer sa boucle) : un bandeau ambre apparaît en haut de
      n'importe quel écran, dit ce qui cloche, et « Relancer le lecteur » le
      remet en marche sans quitter l'application. Le bandeau disparaît seul.
- [ ] **Pas de bandeau dans Safari.** Le même écran dans Safari n'affiche jamais
      ce bandeau, quel que soit l'état du lecteur : hors de l'application, il
      n'y a pas de pont, et l'interface ne prétend rien savoir du poste.

## Ajouter un dossier au sélecteur du Mac (#70)

Le sélecteur de fichiers est une fenêtre du système : personne ne clique dedans
dans un test. Ce qui suit le choix - l'écriture dans la configuration, la liste
rendue, la reprise du lecteur - est éprouvé par `cargo test --test ajout` ; ce
qui se joue ici est le geste lui-même, du clic au dépôt qui apparaît.

- [ ] **Le sélecteur s'ouvre.** Dans Réglages, cliquer sur « Ajouter un
      dossier » : le sélecteur de dossiers de macOS s'ouvre par-dessus la
      fenêtre, et l'application continue de répondre pendant qu'il est ouvert.
      Le bouton dit « Sélecteur ouvert… » et ne réagit plus.
- [ ] **Refermer ne fait rien.** Annuler le sélecteur : aucun message
      n'apparaît, la liste ne bouge pas, et `~/.config/vibemap/config.toml` n'a
      pas gagné de ligne.
- [ ] **Le dossier choisi est surveillé.** Choisir `~/Sites` (ou tout dossier
      contenant au moins un dépôt) : sa ligne apparaît aussitôt avec son compte
      de dépôts, et le message nomme le dossier. Rien n'a été ouvert d'autre que
      le sélecteur (FR-036).
- [ ] **Le fichier a gagné une ligne, et rien perdu.** `cat
      ~/.config/vibemap/config.toml` : le nouveau dossier y est, écrit `~/…`,
      à côté des anciens ; `supabase_url`, `machine_id`, `label` et les cadences
      sont intacts, commentaires compris.
- [ ] **Les dépôts arrivent en moins d'une minute.** Sans rien fermer ni
      relancer, l'accueil montre les dépôts du dossier ajouté dans la minute
      (FR-034). Le terminal d'où l'application a été lancée montre un nouveau
      « vibemap surveille depuis… » suivi d'une ligne de cartographie : c'est le
      lecteur qui vient de repartir avec la nouvelle liste.
- [ ] **Le poste n'est pas perdu au passage.** Juste après l'ajout,
      `lsof ~/.config/vibemap/lecteur.lock` nomme toujours l'application, et la
      machine reste vue il y a moins de 90 secondes depuis un autre appareil.
- [ ] **Aucun bouton dans Safari.** `http://127.0.0.1:51789/reglages` dans
      Safari : la mention de FR-060 s'affiche, et il n'y a **aucun** bouton
      « Ajouter un dossier » - hors de l'application, il n'y a pas de sélecteur
      à ouvrir.

## Ce que cette liste ne peut pas jouer (#62)

**Tuer le lecteur depuis le Moniteur d'activité.** Le critère d'acceptation du
PRD le demande, et il a été écrit avant que #61 n'embarque le lecteur dans le
processus de l'application. Il n'y a plus de processus de lecture à terminer :
celui qu'on y verrait est Vibe Map elle-même, et le terminer ferme la fenêtre.
Ce qui reste vérifiable de cette exigence - qu'une boucle qui cesse de tourner
se voie à la lecture suivante, et qu'un bouton la relance - est éprouvé par
`cargo test --test lecteur`, et la relance se joue à la main ci-dessus depuis un
poste tenu et depuis un trousseau refusé.

**L'état du lecteur pendant que la carte est affichée.** Ce point est levé par
#68 : le pont est désormais ouvert à l'origine que la fenêtre charge, et `web/`
affiche le bandeau de FR-009 et FR-010 par-dessus n'importe quel écran. Il se
joue ci-dessus, dans « Le pont, l'écran Réglages et le bandeau ».
