# Vérifications manuelles - application de bureau

Une fenêtre, un menu, une barre d'adresse absente : rien de tout cela ne se
vérifie sans un écran. Cette liste est ce que le PRD-002 impose de jouer **avant
chaque fusion touchant `bureau/`**. Elle s'allonge avec les tranches ; #57 l'a
commencée, #58 y ajoute la géométrie de la fenêtre.

Ce qui est déjà couvert par les tests automatiques - la sonde de disponibilité,
l'URL fixe de la fenêtre, la lecture et l'écriture de la géométrie - n'a pas à
être rejoué ici : `cargo test` dans `bureau/` s'en charge.

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
