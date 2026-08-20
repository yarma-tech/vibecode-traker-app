/**
 * Les faits du poste, tels que l'écran Réglages les lit (issue #68).
 *
 * FR-059 partage les sources, et tout ce fichier découle de ce partage : les
 * dossiers surveillés, leur compte de dépôts, leur lisibilité et leur
 * autorisation viennent du **pont** de l'application de bureau - ce sont des
 * faits de cette machine, que la base ne porte pas et ne portera pas. L'heure
 * de la dernière cartographie, elle, se lit **en base** : elle y est déjà, et
 * c'est ce qui la rend lisible depuis un autre appareil et par-delà les
 * fermetures de l'application.
 *
 * FR-060 en est la conséquence directe : la même interface ouverte dans un
 * navigateur ordinaire ne trouve aucun pont. Elle doit alors remplacer cette
 * seule section par une mention, jamais afficher une liste vide - qui se
 * lirait « rien n'est surveillé » - ni une erreur. Le reste de l'écran, qui
 * vient de la base, continue de s'afficher.
 *
 * Comme `figement.ts` et `fraicheur.ts`, ces fonctions sont pures : jamais
 * d'horloge lue ici, seulement un `maintenant` reçu en paramètre.
 */

import { dureeTexte } from "./figement";

/* ---------- ce que le pont rend ---------- */

/**
 * Pourquoi un dossier surveillé se lit, ou ne se lit pas. Les quatre cas sont
 * ceux que l'application distingue (`bureau/src/dossiers.rs`) : ils ne se
 * corrigent pas de la même façon, et les confondre renverrait l'utilisateur
 * chercher lui-même lequel il a sous les yeux.
 */
export type Lisibilite = "lisible" | "introuvable" | "autorisation_refusee" | "pas_un_dossier";

export type DossierSurveille = {
  /** Le chemin tel qu'il est écrit dans la configuration - `~/Developer`. */
  chemin: string;
  /** Le même, déplié. Il ne sert qu'à lever le doute que le `~` laisse. */
  emplacement: string;
  lisibilite: Lisibilite;
  /**
   * `null` dès que le dossier ne se lit pas : un zéro se lirait comme un
   * dossier bien ouvert et vide, ce qui est une tout autre affaire (FR-029).
   */
  depots: number | null;
};

/** La réponse du pont à « lister les dossiers surveillés ». */
export type Surveillance =
  | { etat: "lue"; dossiers: DossierSurveille[] }
  | { etat: "sans_configuration"; raison: string };

/**
 * Ce que la fenêtre a obtenu en demandant. `null` dit qu'il n'y a pas de pont
 * à qui demander - le navigateur ordinaire de FR-060. « sans réponse » dit
 * l'inverse : le pont est là, et l'appel n'a rien rendu. Les confondre ferait
 * annoncer « ces réglages n'existent que dans l'application » à quelqu'un qui
 * est justement dedans.
 */
export type ReponseDuPont = Surveillance | { etat: "sans_reponse"; raison: string } | null;

/* ---------- le pont est-il là ? (FR-060) ---------- */

/**
 * Le pont des commandes locales répond-il dans cette fenêtre ?
 *
 * La question ne se pose qu'à la seule marque qui compte : la fonction
 * d'appel existe, ou elle n'existe pas. Ni l'agent du navigateur, ni le nom de
 * l'hôte, ni le port ne disent quoi que ce soit - l'interface est servie sur
 * la même origine locale dans les deux cas, et c'est justement pour cela
 * qu'elle a été choisie. Seul le pont fait la différence.
 */
export function pontOuvert(fenetre: unknown): boolean {
  return typeof invocateur(fenetre) === "function";
}

/**
 * La fonction d'appel du pont, ou `null`. Extraite à part parce que la
 * détection et l'appel doivent juger sur exactement la même chose : un écran
 * qui déciderait « le pont est là » sur un critère et appellerait sur un autre
 * finirait par lancer une erreur dans un navigateur ordinaire.
 */
export function invocateur(fenetre: unknown): unknown {
  if (!fenetre || typeof fenetre !== "object") return null;
  const tauri = (fenetre as { __TAURI__?: { core?: { invoke?: unknown } } }).__TAURI__;
  return tauri?.core?.invoke ?? null;
}

/* ---------- ce que la section des dossiers montre ---------- */

/**
 * Ce que l'écran affiche à la place de la liste, ou la liste elle-même.
 *
 * - « hors application » : pas de pont. La mention de FR-060, jamais un vide.
 * - « sans configuration » : le pont est là, mais il ne trouve pas la
 *   configuration du lecteur. « On ne sait pas ce qui est surveillé » n'est pas
 *   « rien n'est surveillé ».
 * - « aucun » : le pont a lu la configuration, et elle ne surveille rien. C'est
 *   le seul vrai vide de cette section.
 * - « sans réponse » : le pont est là et n'a rien rendu. Rare, mais il ne doit
 *   pas se déguiser en navigateur ordinaire.
 * - « liste » : les dossiers, tels qu'ils sont.
 */
export type SectionDossiers =
  | { quoi: "hors_application" }
  | { quoi: "sans_configuration"; raison: string }
  | { quoi: "sans_reponse"; raison: string }
  | { quoi: "aucun" }
  | { quoi: "liste"; dossiers: DossierSurveille[] };

/**
 * Laquelle des cinq. `reponse` vaut `null` quand il n'y a pas de pont à qui
 * demander - c'est le cas du navigateur ordinaire, et c'est le seul qui bascule
 * sur la mention de FR-060.
 */
export function sectionDossiers(reponse: ReponseDuPont): SectionDossiers {
  if (reponse === null) return { quoi: "hors_application" };
  if (reponse.etat === "sans_reponse") {
    return { quoi: "sans_reponse", raison: reponse.raison };
  }
  if (reponse.etat === "sans_configuration") {
    return { quoi: "sans_configuration", raison: reponse.raison };
  }
  if (reponse.dossiers.length === 0) return { quoi: "aucun" };
  return { quoi: "liste", dossiers: reponse.dossiers };
}

/* ---------- la ligne d'un dossier (FR-028, FR-029) ---------- */

/**
 * Ce qu'une ligne dit d'un dossier : son compte de dépôts, et le signal qui le
 * remplace quand le dossier ne se lit pas.
 *
 * `signal` est `null` pour un dossier lisible : c'est ce qui distingue une
 * ligne ordinaire d'une ligne à signaler (FR-029). Le texte porte le sens à
 * lui seul - jamais la couleur seule.
 */
export type LigneDossier = { compte: string | null; signal: string | null };

const SIGNAUX: Record<Exclude<Lisibilite, "lisible">, string> = {
  introuvable: "Dossier introuvable",
  autorisation_refusee: "Accès refusé",
  pas_un_dossier: "Pas un dossier",
};

export function ligneDossier(dossier: DossierSurveille): LigneDossier {
  if (dossier.lisibilite !== "lisible" || dossier.depots === null) {
    // Pas de compte du tout : un « 0 dépôt » sur une ligne signalée
    // enverrait chercher des dépôts absents d'un dossier qui, lui, est absent.
    return { compte: null, signal: SIGNAUX[signalable(dossier.lisibilite)] };
  }
  return { compte: compteDepots(dossier.depots), signal: null };
}

/**
 * Un dossier lisible dont le compte manque quand même : le pont n'en produit
 * pas, mais l'écran ne doit pas se taire si cela arrivait. On le range avec ce
 * qui ne se lit pas, ce qui est vrai de son compte.
 */
function signalable(lisibilite: Lisibilite): Exclude<Lisibilite, "lisible"> {
  return lisibilite === "lisible" ? "pas_un_dossier" : lisibilite;
}

/**
 * Le compte, en clair. Zéro se dit « aucun dépôt » et non « 0 dépôt » : c'est
 * une phrase, pas une case de tableau, et c'est souvent la réponse à
 * « pourquoi ce dépôt n'apparaît pas ».
 */
export function compteDepots(depots: number): string {
  if (depots === 0) return "aucun dépôt";
  return depots === 1 ? "1 dépôt" : `${depots} dépôts`;
}

/* ---------- l'heure de la dernière cartographie (FR-030, FR-086) ---------- */

/**
 * L'heure de la dernière cartographie, telle que la base la porte.
 *
 * « jamais » quand aucun dépôt n'a d'heure : rien n'a jamais été cartographié
 * ici, et une heure vide se lirait comme une panne. Sinon `quand`, en
 * millisecondes, que l'écran met en forme dans le fuseau du lecteur, et `age`,
 * l'ancienneté en clair.
 */
export type Cartographie = { etat: "jamais" } | { etat: "connue"; quand: number; age: string };

/**
 * FR-086 : un agrégat, et l'écran l'établit comme tel - la PLUS RÉCENTE des
 * heures de cartographie des dépôts, jamais une heure par dépôt ni celle d'un
 * dépôt pris au hasard.
 *
 * Les heures illisibles sont ignorées plutôt que de contaminer le maximum :
 * une seule date invalide rendrait l'agrégat entier `NaN`, et l'écran
 * annoncerait « jamais cartographié » à un poste qui l'a été mille fois.
 */
export function derniereCartographie(
  heures: ReadonlyArray<string | null | undefined>,
  maintenant: number,
): Cartographie {
  const connues = heures
    .filter((heure): heure is string => typeof heure === "string" && heure !== "")
    .map((heure) => Date.parse(heure))
    .filter((instant) => !Number.isNaN(instant));

  if (connues.length === 0) return { etat: "jamais" };

  const quand = Math.max(...connues);
  return { etat: "connue", quand, age: dureeTexte(maintenant - quand) };
}

/* ---------- le lecteur, par-dessus l'interface (FR-009, FR-010) ---------- */

/**
 * Ce que le pont rend de l'état du lecteur (`bureau/src/lecteur.rs`).
 *
 * Un fait du poste de plus, que la base ne porte pas : elle ne sait rien d'un
 * verrou pris sur cette machine. C'est pourquoi la carte, qui vient d'elle,
 * peut s'afficher entière pendant que plus rien ne remonte d'ici.
 */
export type CasEchecLecteur = "poste_tenu" | "jeton_refuse" | "arret_inattendu" | "panne";

export type EtatLecteur =
  | { etat: "en_demarrage" }
  | { etat: "en_marche" }
  | { etat: "arrete" }
  | { etat: "en_echec"; cas: CasEchecLecteur; raison: string };

/**
 * Le bandeau à passer par-dessus l'interface, ou rien.
 *
 * POURQUOI un bandeau qui ne parle que quand ça cloche : un lecteur arrêté ne
 * se voit nulle part ailleurs - la carte continue de s'afficher, alimentée par
 * ce que la machine avait déjà envoyé (FR-010). Un bandeau permanent qui
 * annoncerait « le lecteur tourne » par-dessus chaque écran serait, lui, du
 * bruit : la page d'attente de l'application dit déjà les quatre états au
 * lancement (FR-009), et un démarrage en cours n'appelle aucun geste.
 */
export type BandeauLecteur =
  | { visible: false }
  | { visible: true; titre: string; explication: string; relance: boolean };

const RIEN_NE_PART =
  "Cette machine n'envoie plus rien tant qu'il ne tourne pas : ce que vous voyez ici date " +
  "d'avant son arrêt.";

/**
 * `lecteur` vaut `null` hors de l'application, où il n'y a pas de pont à qui
 * demander : l'interface ne prétend alors rien savoir du poste, et ne montre
 * aucun bandeau (FR-060).
 */
export function bandeauDuLecteur(lecteur: EtatLecteur | null): BandeauLecteur {
  if (lecteur === null) return { visible: false };
  if (lecteur.etat === "en_marche" || lecteur.etat === "en_demarrage") {
    return { visible: false };
  }
  if (lecteur.etat === "arrete") {
    return {
      visible: true,
      titre: "Le lecteur est arrêté",
      explication: RIEN_NE_PART,
      relance: true,
    };
  }

  if (lecteur.cas === "poste_tenu") {
    return {
      visible: true,
      titre: "Un lecteur tourne déjà sur cette machine",
      explication:
        `${lecteur.raison} Vibe Map n'en démarre pas un second : la même activité partirait ` +
        "en double. Arrêtez l'autre, puis relancez.",
      relance: true,
    };
  }
  if (lecteur.cas === "arret_inattendu") {
    return {
      visible: true,
      titre: "Le lecteur s'est arrêté tout seul",
      explication: `Il tournait, et il ne tourne plus. ${RIEN_NE_PART}`,
      relance: true,
    };
  }
  if (lecteur.cas === "jeton_refuse") {
    return {
      visible: true,
      titre: "Le jeton de cette machine n'a pas pu être lu",
      explication: `${lecteur.raison} Un refus d'autorisation laisse le lecteur à l'arrêt.`,
      relance: true,
    };
  }
  return {
    visible: true,
    titre: "Le lecteur n'a pas démarré",
    explication: `${lecteur.raison} ${RIEN_NE_PART}`,
    relance: true,
  };
}
