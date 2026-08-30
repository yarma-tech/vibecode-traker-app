/**
 * La session, d'un lancement à l'autre et jusqu'à la sortie (issue #64,
 * FR-014, FR-015).
 *
 * Deux règles vivent ici, et rien d'autre :
 *
 * - **entrer une seule fois** : la fenêtre recharge à chaque lancement l'origine
 *   locale fixe, et la session posée sous cette origine s'y retrouve. C'est tout
 *   ce que FR-014 demande, et cela ne coûte rien à tenir - à condition que
 *   l'origine ne bouge pas (FR-070) ;
 * - **sortir** : la déconnexion ferme la session ET arrête le lecteur, parce
 *   que plus aucune session ouverte ne justifie que cette machine émette.
 *
 * Ces fonctions sont pures : ni horloge, ni navigateur, ni pont. Ce sont les
 * écrans qui exécutent ce qu'elles décident - c'est ce qui rend éprouvable, sans
 * quitter puis rouvrir l'application, une promesse qui ne se vit qu'ainsi.
 */

/* ---------- où la session est écrite, et qui la retrouve (FR-014) ---------- */

/**
 * Le dépôt sous lequel une session écrite depuis cette origine se range.
 *
 * `@supabase/ssr` range la session dans les cookies du navigateur, et un
 * cookie appartient à un **hôte**. `127.0.0.1` et `localhost` désignent la même
 * machine et n'ont pourtant rien en commun ici : une session posée sous l'un
 * est introuvable sous l'autre. C'est exactement le piège que
 * `origineDeLaRequete` évite dans `lib/autorisation.ts`, où Next propose
 * `localhost` alors que le client a demandé `127.0.0.1`.
 *
 * Le port, lui, n'entre pas dans la portée d'un cookie - il n'a jamais fait
 * partie de son domaine. Ce que le port fixe tient est ailleurs, et c'est tout
 * aussi structurant : l'adresse de retour d'autorisation déclarée une fois pour
 * toutes chez le fournisseur d'identité, et une fenêtre qui recharge la MÊME
 * origine à chaque lancement plutôt qu'une adresse tirée au hasard
 * (`docs/VIGILANCE.md`).
 *
 * Une origine qu'on ne sait pas lire ne se range nulle part : rendre `null`
 * plutôt qu'une chaîne vide évite de faire coïncider deux illisibles.
 */
export function depotDeSession(origine: string): string | null {
  try {
    return new URL(origine).hostname || null;
  } catch {
    return null;
  }
}

/**
 * Une session écrite depuis `origineDuDepot` se relit-elle depuis
 * `origineDeLaLecture` ?
 */
export function memeDepotDeSession(origineDuDepot: string, origineDeLaLecture: string): boolean {
  const depot = depotDeSession(origineDuDepot);
  return depot !== null && depot === depotDeSession(origineDeLaLecture);
}

/* ---------- entrer, et repartir (FR-015) ---------- */

/**
 * Ce que la déconnexion doit faire, en plus de fermer la session.
 *
 * `arreterLeLecteur` est la seconde moitié de FR-015, et elle ne se voit pas à
 * l'écran : l'écran de connexion reviendrait tout aussi bien avec un lecteur
 * resté en marche, qui continuerait de tenir le poste et d'envoyer l'activité
 * de cette machine au nom d'une session fermée. La règle est donc posée ici, une
 * fois, plutôt que laissée à l'ordre des appels d'un gestionnaire de clic.
 *
 * Hors de l'application - le site hébergé dans un navigateur ordinaire - il n'y
 * a aucun lecteur à arrêter : il n'y a pas de pont, et la machine qui lit les
 * dossiers n'est pas celle qui affiche la page.
 */
export type EtapesDeDeconnexion = { arreterLeLecteur: boolean };

export function etapesDeDeconnexion(dansLApplication: boolean): EtapesDeDeconnexion {
  return { arreterLeLecteur: dansLApplication };
}

/**
 * Ce qui suit une session qui vient de s'ouvrir, dans l'ordre où cela se fait.
 *
 * `relancerLeLecteur` couvre le dernier critère de l'issue #64 : après une
 * déconnexion, le lecteur est arrêté et le poste rendu. Se reconnecter doit le
 * remettre en marche sans que l'application ait eu à se fermer - sinon la carte
 * reviendrait au-dessus d'une machine muette, et rien ne le dirait.
 *
 * `revenirAuPremierPlan` est FR-072 : à cet instant, c'est le navigateur du
 * système qui est devant.
 *
 * Les deux ne valent que dans l'application : dans un navigateur ordinaire, il
 * n'y a ni lecteur ni fenêtre à ramener.
 */
export type SuiteDeLaConnexion = {
  relancerLeLecteur: boolean;
  revenirAuPremierPlan: boolean;
};

export function suiteDeLaConnexion(dansLApplication: boolean): SuiteDeLaConnexion {
  return {
    relancerLeLecteur: dansLApplication,
    revenirAuPremierPlan: dansLApplication,
  };
}
