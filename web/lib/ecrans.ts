/**
 * Le choix des écrans « avant que le produit soit plein » (issue #12) :
 * premier lancement, chargement, repo sans activité - et, depuis FR-085
 * (issue #59), base injoignable, qui n'est pas un vide mais une absence de
 * réponse.
 *
 * La règle est simple et se décide sans réseau : l'accueil bascule en onboarding
 * tant qu'aucune machine n'est appairée ; le plan d'un repo montre l'état sans
 * activité quand il n'a ni module coloré ni ligne au journal. Un paramètre de
 * développement (`?demo=`) force chaque écran, pour les atteindre sans provoquer
 * la vraie panne (critère 6). Ces fonctions pures portent cette bascule, une
 * seule fois, pour que l'accueil et l'écran d'un repo ne puissent se contredire.
 */

/** Les écrans que le paramètre `?demo=` sait forcer en développement. */
export type EcranDemo = "onboarding" | "chargement" | "sans-activite" | "injoignable";

/**
 * L'écran forcé lu dans le paramètre `?demo=`, ou `null` si rien de connu n'y
 * est demandé. Une valeur étrangère ne force rien : on retombe sur le réel.
 */
export function demoDemande(param: string | undefined): EcranDemo | null {
  if (
    param === "onboarding" ||
    param === "chargement" ||
    param === "sans-activite" ||
    param === "injoignable"
  ) {
    return param;
  }
  return null;
}

/**
 * L'accueil doit-il montrer le premier lancement ? Oui tant qu'aucune machine
 * n'est appairée, oui aussi quand le dev le force. La bascule se défait toute
 * seule dès qu'une machine répond : `nbMachines` repasse au-dessus de zéro.
 */
export function montrerPremierLancement(nbMachines: number, demo: EcranDemo | null): boolean {
  return demo === "onboarding" || nbMachines === 0;
}

/**
 * Le repo est-il cartographié mais sans activité ? Vrai quand il n'a ni module
 * dans un état (lu, écrit, conflit) ni événement au journal — la géométrie est
 * pleine, la couleur absente. Le dev peut le forcer, données présentes ou non.
 */
export function repoSansActivite(
  nbEtats: number,
  nbEvenements: number,
  demo: EcranDemo | null,
): boolean {
  return demo === "sans-activite" || (nbEtats === 0 && nbEvenements === 0);
}

/* ---------- la base injoignable (FR-085, issue #59) ---------- */

/**
 * Ce qu'un client Supabase pose dans `error` quand une requête échoue : le
 * message et le code de PostgREST, le nom et le statut des erreurs
 * d'authentification. Aucun de ces champs n'est garanti selon le chemin
 * emprunté, d'où le tout optionnel : on juge sur ce qui est là.
 */
export type ErreurRequete =
  | {
      message?: string | null;
      details?: string | null;
      code?: string | null;
      name?: string | null;
      status?: number | null;
    }
  | null
  | undefined;

/**
 * Les codes que la couche réseau pose quand la connexion n'aboutit pas. Ils
 * n'arrivent jamais *de* la base : ils arrivent à sa place.
 */
const CODES_SYSTEME = [
  "ENOTFOUND",
  "EAI_AGAIN",
  "ECONNREFUSED",
  "ECONNRESET",
  "ETIMEDOUT",
  "EHOSTUNREACH",
  "ENETUNREACH",
  "ENETDOWN",
  "EPIPE",
  "ABORT_ERR",
  "UND_ERR_CONNECT_TIMEOUT",
];

/** Tout ce que l'erreur porte de lisible, ramené à une seule ligne minuscule. */
function trace(erreur: NonNullable<ErreurRequete>): string {
  return [erreur.name, erreur.code, erreur.message, erreur.details]
    .filter((champ): champ is string => typeof champ === "string" && champ !== "")
    .join(" ")
    .toLowerCase();
}

/**
 * La base a-t-elle parlé ? Elle se reconnaît à deux marques, et à elles seules :
 * un statut HTTP, ou un code bien à elle (`42501`, `PGRST301`…). Un code posé
 * par la couche réseau n'en est pas un : il dit précisément que personne n'a
 * décroché.
 */
function aRepondu(erreur: NonNullable<ErreurRequete>): boolean {
  if (typeof erreur.status === "number" && erreur.status > 0) return true;
  const code = (erreur.code ?? "").trim().toUpperCase();
  return code !== "" && !CODES_SYSTEME.includes(code);
}

/**
 * L'erreur dit-elle que la base est injoignable ? Le partage se fait sur une
 * seule question : la base a-t-elle répondu ? Un refus de droits, un jeton
 * périmé, une contrainte violée sont des réponses - la base est là, elle dit
 * non ; l'écran n'a pas à l'annoncer absente. Réseau coupé, hôte en panne,
 * adresse muette ne laissent au contraire rien qui vienne d'elle : c'est cela,
 * une base injoignable.
 */
export function baseInjoignable(erreur: ErreurRequete): boolean {
  if (!erreur) return false;
  return !aRepondu(erreur);
}

/**
 * Les raisons qu'on sait nommer, de la plus précise à la plus vague. L'ordre
 * compte : la première qui reconnaît sa marque l'emporte.
 */
const RAISONS: ReadonlyArray<readonly [readonly string[], string]> = [
  [["enotfound", "eai_again", "getaddrinfo"], "Son adresse ne mène nulle part."],
  [["econnrefused"], "Elle refuse la connexion."],
  [["enetunreach", "enetdown", "ehostunreach"], "Cette machine n'a aucune route vers elle."],
  [["econnreset", "epipe", "socket hang up"], "La connexion s'est coupée en route."],
  [
    ["etimedout", "und_err_connect_timeout", "aborterror", "abort_err"],
    "Elle n'a pas répondu à temps.",
  ],
];

/**
 * La raison, en clair, d'une base injoignable : une phrase du produit, jamais
 * le message brut du client Supabase, que rien ne rend lisible. Quand la trace
 * ne nomme rien de connu, on s'en tient au seul fait certain - l'appel est
 * resté sans réponse - plutôt que d'inventer une cause.
 */
export function raisonInjoignable(erreur: ErreurRequete): string {
  const indices = erreur ? trace(erreur) : "";
  for (const [signes, raison] of RAISONS) {
    if (signes.some((signe) => indices.includes(signe))) return raison;
  }
  return "L'appel est resté sans réponse.";
}

/** L'état d'un écran qui dépend d'une lecture en base. */
export type EtatRequete = "injoignable" | "vide" | "peuple";

/**
 * Lequel des trois états s'applique à une lecture ? L'ordre est tout : une base
 * qui n'a pas répondu ne rend pas « zéro ligne », elle ne rend rien - la
 * confondre avec un vide ferait annoncer « aucun dépôt » à un compte qui en a
 * cinquante. Une base joignable qui rend zéro ligne, elle, est bel et bien
 * vide, et garde son état vide existant.
 */
export function etatDeRequete(resultat: {
  data: readonly unknown[] | null | undefined;
  error?: ErreurRequete;
}): EtatRequete {
  if (baseInjoignable(resultat.error)) return "injoignable";
  return resultat.data && resultat.data.length > 0 ? "peuple" : "vide";
}

/**
 * La première erreur qui dit la base injoignable, ou `null`. Un écran lit
 * souvent plusieurs requêtes d'un coup : il bascule dès que l'une d'elles est
 * restée sans réponse, et c'est cette erreur-là qui porte la raison à afficher.
 */
export function premiereInjoignable(erreurs: readonly ErreurRequete[]): ErreurRequete {
  return erreurs.find((erreur) => baseInjoignable(erreur)) ?? null;
}
