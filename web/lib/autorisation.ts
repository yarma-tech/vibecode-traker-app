/**
 * Entrer avec GitHub depuis la fenêtre (issue #63, FR-012, FR-013, FR-016,
 * FR-071, FR-072).
 *
 * Tout ce qui se décide dans ce chemin vit ici, et sans réseau : l'écran de
 * connexion, l'échangeur de `app/auth/callback` et le relais n'appellent que
 * ces fonctions. C'est ce qui rend éprouvable un aller-retour dont la moitié
 * se joue dans un autre programme.
 *
 * ## POURQUOI le retour passe par le navigateur du système
 *
 * GitHub refuse les vues web embarquées : la page d'autorisation ne peut pas
 * s'afficher dans la fenêtre de l'application (PRD-002, Risques). Elle s'ouvre
 * donc dans le navigateur habituel, et le retour vise l'origine locale fixe de
 * l'application - la même que celle de sa fenêtre.
 *
 * ## POURQUOI la fenêtre, et elle seule, peut finir l'échange
 *
 * Le vérificateur PKCE est écrit par le client qui PART. Dans l'application,
 * c'est la fenêtre, et il reste dans ses cookies à elle. Le navigateur du
 * système, qui revient avec le code, n'en a aucun : il ne peut rien échanger,
 * et sa demande de session serait refusée. Le code lui est donc repris au
 * passage - c'est le relais - et c'est la fenêtre qui l'échange, avec le
 * vérificateur qu'elle est seule à avoir.
 *
 * Cela vaut aussi comme garde : un code déposé au relais ne sert à personne
 * d'autre qu'à la fenêtre qui a ouvert le flux.
 *
 * ## POURQUOI deux chemins de retour, et non un seul qui devine
 *
 * Le départ SAIT lequel des deux clients ira chercher l'autorisation : c'est
 * lui qui décide de sortir dans le navigateur du système. Il le dit donc dans
 * l'adresse de retour qu'il demande - `CHEMIN_DE_RETOUR` pour un navigateur
 * ordinaire, `CHEMIN_DE_RETOUR_FENETRE` pour la fenêtre - et le retour n'a
 * plus rien à deviner.
 *
 * Ce fut d'abord deviné, en cherchant un cookie de vérificateur sur le client
 * qui revenait, et c'était faux : un cookie n'a pas de port dans sa portée
 * (`docs/VIGILANCE.md`). Le site servi en développement sur
 * `127.0.0.1:3000` pose donc ses vérificateurs sur le MÊME hôte que l'origine
 * locale fixe de l'application, et un navigateur qui a un jour ouvert ce site
 * revenait avec un vérificateur qui n'était pas le sien. L'échange était tenté
 * là, refusé - « PKCE code verifier not found in storage » -, et la fenêtre
 * recevait un échec au lieu de son code. `@supabase/ssr` laisse en outre
 * derrière lui un cookie d'index (`…-flows-code-verifier`) qui survit aux flux
 * terminés : la marque cherchée n'en était pas une.
 */

/**
 * L'origine locale fixe de l'application (FR-070). Elle est écrite ici et dans
 * `bureau/src/sonde.rs`, et les deux ne doivent jamais diverger : le port fixe
 * est ce qui permet de déclarer une adresse de retour une fois pour toutes
 * chez le fournisseur d'identité (`docs/VIGILANCE.md`).
 */
export const ORIGINE_LOCALE = "http://127.0.0.1:51789";

/**
 * Là où le fournisseur d'identité renvoie un navigateur ordinaire : celui qui
 * est parti est celui qui revient, il porte son vérificateur, et l'échange se
 * fait là.
 */
export const CHEMIN_DE_RETOUR = "/auth/callback";

/**
 * Là où il renvoie le navigateur du système, quand c'est la fenêtre qui l'a
 * envoyé (FR-072). Rien ne s'y échange : le code y est déposé pour la fenêtre,
 * seule à porter le vérificateur.
 *
 * Ce chemin doit être déclaré chez le fournisseur d'identité au même titre que
 * l'autre (`supabase/config.toml`, et les « Redirect URLs » du projet hébergé).
 */
export const CHEMIN_DE_RETOUR_FENETRE = "/auth/callback/fenetre";

/** Ce que le navigateur du système affiche une fois le code repris. */
export const PAGE_DE_RETOUR = "/auth/retour";

/**
 * Combien de temps la fenêtre attend le retour avant de le dire. Une
 * autorisation se donne en quelques secondes ; cinq minutes couvrent une
 * connexion à GitHub faite au passage, et au-delà il vaut mieux dire que rien
 * n'est revenu que laisser un bouton mort.
 */
export const ATTENTE_MAXIMALE_MS = 5 * 60 * 1000;

/**
 * Combien de temps un retour déposé au relais reste bon à prendre. Assez pour
 * que la fenêtre le relève au passage suivant, jamais assez pour qu'un code
 * d'hier ouvre une session aujourd'hui.
 */
export const DUREE_DU_RELAIS_MS = 5 * 60 * 1000;

/* ---------- l'adresse de retour (FR-071) ---------- */

/**
 * L'adresse à laquelle l'autorisation doit revenir.
 *
 * Dans l'application : l'origine locale fixe, TOUJOURS, quelle que soit celle
 * de la page. La fenêtre charge déjà cette origine-là, mais le dire ici est ce
 * qui empêche un service d'interface déplacé - ou une page ouverte depuis une
 * autre adresse de la boucle locale - de demander un retour qui n'est pas
 * déclaré chez le fournisseur. Un retour non déclaré ne revient pas : il
 * retombe sur `site_url`, ailleurs, et le code se perd sans un mot.
 *
 * Hors de l'application, c'est le site hébergé : l'origine de la page, celle
 * du navigateur qui est parti et qui reviendra.
 *
 * Le CHEMIN diffère lui aussi, et c'est ce qui dit au retour à qui il a
 * affaire : la fenêtre envoie dehors, et ce qui revient de dehors ne s'échange
 * pas sur place.
 */
export function urlDeRetour(origineDeLaPage: string, dansLApplication: boolean): string {
  const origine = dansLApplication ? ORIGINE_LOCALE : origineDeLaPage.replace(/\/+$/, "");
  const chemin = dansLApplication ? CHEMIN_DE_RETOUR_FENETRE : CHEMIN_DE_RETOUR;
  return `${origine}${chemin}`;
}

/**
 * L'origine sur laquelle la réponse de l'échangeur doit rester : celle que le
 * client a demandée, jamais celle que le serveur croit servir.
 *
 * POURQUOI la distinction. Next fabrique `request.url` à partir de son propre
 * nom d'hôte : servi sur `127.0.0.1:51789`, il rend malgré tout
 * `http://localhost:51789/…`. Une redirection bâtie là-dessus quitterait
 * l'origine locale fixe - et `localhost` n'est PAS `127.0.0.1` : la session
 * qui vient d'être posée y serait invisible, et la fenêtre de l'application,
 * qui n'admet que l'origine fixe, refuserait tout bonnement d'y aller (FR-070,
 * FR-072). Le seul témoin fiable est l'en-tête `Host` de la requête.
 *
 * Un `Host` qui n'a pas la forme d'une autorité est écarté : il vient du
 * client, et une redirection est exactement ce qu'on ne veut pas lui laisser
 * choisir.
 */
export function origineDeLaRequete(
  urlDeLaRequete: string,
  hote: string | null | undefined,
  protocoleTransmis?: string | null,
): string {
  const vue = new URL(urlDeLaRequete);
  const protocole = protocoleTransmis?.split(",")[0]?.trim();
  const schema = protocole === "http" || protocole === "https" ? `${protocole}:` : vue.protocol;

  const propose = hote?.trim() ?? "";
  return AUTORITE.test(propose) ? `${schema}//${propose}` : vue.origin;
}

/** Un nom d'hôte, une adresse, entre crochets ou non, avec ou sans port. */
const AUTORITE = /^(?:\[[0-9a-fA-F:.]+\]|[a-zA-Z0-9.-]+)(?::\d{1,5})?$/;

/**
 * Ce que le clic sur « Continuer avec GitHub » déclenche.
 *
 * - `deja_connecte` : il y a une session. On ne redemande pas d'autorisation
 *   pour une porte déjà ouverte (FR-014) - la carte s'affiche.
 * - `au_navigateur_du_systeme` : dans l'application. La fenêtre ne navigue
 *   PAS vers GitHub, qui refuserait la vue embarquée ; elle demande l'adresse
 *   et la fait ouvrir dehors (FR-071).
 * - `dans_cette_page` : navigateur ordinaire. La page part elle-même, comme
 *   n'importe quel site.
 */
export type Depart =
  | { quoi: "deja_connecte" }
  | { quoi: "au_navigateur_du_systeme"; retour: string }
  | { quoi: "dans_cette_page"; retour: string };

export function departAutorisation(etat: {
  sessionOuverte: boolean;
  dansLApplication: boolean;
  origineDeLaPage: string;
}): Depart {
  if (etat.sessionOuverte) return { quoi: "deja_connecte" };

  const retour = urlDeRetour(etat.origineDeLaPage, etat.dansLApplication);
  return etat.dansLApplication
    ? { quoi: "au_navigateur_du_systeme", retour }
    : { quoi: "dans_cette_page", retour };
}

/* ---------- ce que le retour porte (FR-016) ---------- */

/**
 * Pourquoi un retour n'ouvre pas de session. Les trois cas se distinguent
 * parce qu'ils ne se racontent pas pareil : l'un est un refus de l'utilisateur
 * ou de GitHub, l'autre un retour vide, le dernier un échange que le
 * fournisseur a rejeté.
 */
export type CauseDuRefus = "refus_github" | "sans_code" | "echange_refuse";

/** Ce qu'un retour d'autorisation porte, lu de ses seuls paramètres. */
export type Retour =
  | { quoi: "code"; code: string }
  | { quoi: "refus"; raison: string; cause: CauseDuRefus };

const SANS_CODE = "GitHub n'a renvoyé aucun code d'autorisation.";

/**
 * Lit le retour. Un refus l'emporte toujours sur le reste : quand GitHub dit
 * pourquoi il n'a pas autorisé, c'est cette phrase-là qu'il faut afficher, et
 * non « aucun code », qui est vrai mais n'apprend rien.
 */
export function retourDAutorisation(parametres: URLSearchParams): Retour {
  const refus = texte(parametres.get("error_description")) ?? texte(parametres.get("error"));
  if (refus) return { quoi: "refus", raison: refus, cause: "refus_github" };

  const code = texte(parametres.get("code"));
  if (!code) return { quoi: "refus", raison: SANS_CODE, cause: "sans_code" };

  return { quoi: "code", code };
}

/** Un paramètre qui porte quelque chose, ou rien. Une chaîne vide n'est rien. */
function texte(valeur: string | null): string | null {
  const propre = valeur?.trim() ?? "";
  return propre === "" ? null : propre;
}

/* ---------- le relais, entre le navigateur et la fenêtre (FR-072) ---------- */

/** Un retour déposé est-il encore bon à prendre ? */
export function relaisEncoreValable(depose: number, maintenant: number): boolean {
  const age = maintenant - depose;
  return age >= 0 && age < DUREE_DU_RELAIS_MS;
}

/** Ce que le relais rend à la fenêtre qui vient voir. */
export type Reprise =
  | { quoi: "rien" }
  | { quoi: "code"; code: string }
  | { quoi: "refus"; raison: string };

const RELAIS_ILLISIBLE = "Le retour d'autorisation n'a pas pu être lu.";

/**
 * Ce que la fenêtre fait de la réponse du relais.
 *
 * Une réponse qu'on ne sait pas lire devient un refus, et non « rien » : la
 * fenêtre attendrait alors sans fin un retour déjà arrivé, et l'utilisateur
 * n'aurait rien à lire ni rien à reprendre. Un échec s'annonce, il ne s'avale
 * pas (FR-016).
 */
export function repriseDeLaFenetre(reponse: unknown): Reprise {
  if (reponse === null || reponse === undefined) return { quoi: "rien" };
  if (typeof reponse !== "object") return { quoi: "refus", raison: RELAIS_ILLISIBLE };

  const lu = reponse as { etat?: unknown; code?: unknown; raison?: unknown };
  if (lu.etat === "rien") return { quoi: "rien" };
  if (lu.etat === "code" && typeof lu.code === "string" && lu.code !== "") {
    return { quoi: "code", code: lu.code };
  }
  if (lu.etat === "refus" && typeof lu.raison === "string" && lu.raison !== "") {
    return { quoi: "refus", raison: lu.raison };
  }
  return { quoi: "refus", raison: RELAIS_ILLISIBLE };
}

/** L'attente du retour a-t-elle assez duré pour qu'on en parle ? */
export function attenteExpiree(debut: number, maintenant: number): boolean {
  return maintenant - debut >= ATTENTE_MAXIMALE_MS;
}

/* ---------- l'écran de connexion (FR-012, FR-016) ---------- */

/**
 * Où en est le geste. `ouverture` couvre le temps de demander l'adresse à
 * Supabase et de la faire ouvrir dehors ; `attente` couvre l'aller-retour dans
 * le navigateur, pendant lequel l'application n'a rien d'autre à faire que
 * guetter le retour.
 */
export type PhaseConnexion = "repos" | "ouverture" | "attente";

/**
 * L'écran de connexion, en entier. UN bouton, jamais deux, et rien d'autre à
 * actionner (FR-012) : c'est la forme même qui porte cette exigence, et non
 * une relecture du JSX.
 */
export type EcranConnexion = {
  bouton: { texte: string; actif: boolean };
  /** Ce que l'écran dit pendant l'aller-retour, ou rien. */
  attente: string | null;
  /** La raison de l'échec, ou rien. Jamais tue (FR-016). */
  echec: string | null;
};

const ATTENTE =
  "L'autorisation s'ouvre dans ton navigateur. Reviens ici une fois GitHub validé : " +
  "la fenêtre reprend la main toute seule.";

/**
 * Un échec laisse TOUJOURS le bouton actif : « permettre de recommencer » est
 * la seconde moitié de FR-016, et un écran qui afficherait la raison sans
 * rendre la main serait un cul-de-sac. La règle est posée ici, une fois, plutôt
 * que surveillée dans l'ordre des `setState` de l'écran.
 */
export function ecranDeConnexion(phase: PhaseConnexion, echec: string | null): EcranConnexion {
  if (echec !== null) {
    return {
      bouton: { texte: "Continuer avec GitHub", actif: true },
      attente: null,
      echec: messageDEchec(echec),
    };
  }
  if (phase === "ouverture") {
    return { bouton: { texte: "Ouverture de GitHub…", actif: false }, attente: null, echec: null };
  }
  if (phase === "attente") {
    return { bouton: { texte: "En attente de GitHub…", actif: false }, attente: ATTENTE, echec: null };
  }
  return { bouton: { texte: "Continuer avec GitHub", actif: true }, attente: null, echec: null };
}

/** La raison, dans une phrase du produit. */
export function messageDEchec(raison: string): string {
  return `La connexion a échoué : ${raison}`;
}

/** Ce que l'écran dit quand le retour n'est jamais arrivé. */
export const AUTORISATION_JAMAIS_REVENUE =
  "l'autorisation n'est pas revenue du navigateur. Réessaie, ou vérifie que la page GitHub " +
  "s'est bien ouverte.";
