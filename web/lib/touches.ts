/**
 * Les deux dates de dernière touche d'une zone, telles que l'écran les reçoit
 * (F8, issues #75 et #76 - FR-039, FR-040, FR-043).
 *
 * Une zone porte le plus récent de ce qui la vise ET de ce qui vise les zones
 * qu'elle contient. Cet héritage se calcule ENTIÈREMENT en base, à la lecture,
 * par préfixe de chemin : c'est `touches_modules(p_repo_id)` (migration
 * 20260820000001) qui le fait, exactement comme `etat_modules` le fait déjà
 * pour les couleurs (migration 20260804000001).
 *
 * Ce fichier ne refait donc RIEN de ce calcul, et c'est le point : si l'écran
 * recomposait l'héritage de son côté, il existerait deux règles - celle de la
 * base et celle du navigateur - qui finiraient par diverger. FR-043 interdit
 * précisément cette dérive. Ici, on ne fait que ranger la réponse de la base
 * par chemin de zone, et donner un nom à l'absence.
 *
 * Deux absences se ressemblent et n'ont pas le même sens :
 *
 *   - la zone n'a AUCUNE ligne : ni elle ni aucun de ses descendants n'a jamais
 *     été touché. `touches_modules` joint `dernieres_touches`, donc elle ne rend
 *     rien plutôt qu'une ligne de dates nulles ;
 *   - la zone a une ligne, mais une seule des deux dates : elle a été lue sans
 *     jamais être écrite (ou l'inverse). Les deux dates vivent séparément
 *     (FR-039, FR-040), la base ne les mélange jamais.
 *
 * Les deux se ramènent au même objet, `AUCUNE_TOUCHE` pour la première :
 * l'appelant n'a qu'un cas à traiter, « pas de date », et la mise en mots
 * (« modifié il y a 2 j, relu il y a 20 min », FR-041 et FR-042) part de là.
 *
 * Ces dates ne servent QU'À se lire : elles n'entrent pas dans le calcul des
 * couleurs de la carte (FR-044), qui reste celui de `etat_modules` sur sa
 * fenêtre glissante. C'est pourquoi elles voyagent dans un canal à part de
 * `Etat`, jamais fondues dedans.
 *
 * La mise en mots vit ici aussi (issue #77, FR-041 et FR-042), et pas dans
 * l'écran : elle se juge sur un `maintenant` reçu en paramètre, jamais sur
 * l'horloge du poste - un test qui lirait l'heure réelle deviendrait rouge un
 * jour sans que rien n'ait changé (même discipline que `figement.ts` et
 * `fraicheur.ts`). L'arrondi, lui, se réutilise : `dureeTexte` est la seule
 * formule d'ancienneté du produit.
 */

import { dureeTexte } from "./figement";

/** Une ligne de `touches_modules`, telle que la base la rend. */
export type Touche = {
  module_path: string;
  derniere_ecriture: string | null;
  derniere_lecture: string | null;
};

/** Les deux dates d'une zone, chacune de son côté. ISO 8601, ou `null`. */
export type DatesZone = {
  ecriture: string | null;
  lecture: string | null;
};

/** Ce que porte une zone dont rien, ni elle ni ce qu'elle contient, n'a bougé. */
export const AUCUNE_TOUCHE: DatesZone = { ecriture: null, lecture: null };

/**
 * La plus récente de deux dates, `null` comptant pour « pas de date » et jamais
 * pour « le début des temps ». Même sémantique que le `greatest` de Postgres,
 * qui ignore les nuls (`noter_dernieres_touches`).
 */
function plusRecente(a: string | null, b: string | null): string | null {
  if (a === null) return b;
  if (b === null) return a;
  return Date.parse(b) > Date.parse(a) ? b : a;
}

/**
 * Range la réponse de `touches_modules` par chemin de zone.
 *
 * La base rend au plus une ligne par zone (`group by m.path`), mais deux lignes
 * du même chemin se fondent quand même ici, chaque date de son côté, plutôt que
 * la dernière arrivée n'écrase la précédente : un écran qui perdrait une date
 * sur un doublon serait faux sans jamais le dire.
 */
export function indexerTouches(touches: Touche[]): Map<string, DatesZone> {
  const parZone = new Map<string, DatesZone>();

  for (const touche of touches) {
    const deja = parZone.get(touche.module_path) ?? AUCUNE_TOUCHE;
    parZone.set(touche.module_path, {
      ecriture: plusRecente(deja.ecriture, touche.derniere_ecriture),
      lecture: plusRecente(deja.lecture, touche.derniere_lecture),
    });
  }

  return parZone;
}

/**
 * Les deux dates d'une zone, héritage compris - puisque c'est déjà ce que la
 * base a mis dans sa ligne. Une zone absente de l'index n'a rien reçu :
 * `AUCUNE_TOUCHE`, jamais une date reconstituée depuis celle d'un descendant.
 */
export function touchesDeLaZone(
  parZone: Map<string, DatesZone>,
  chemin: string,
): DatesZone {
  return parZone.get(chemin) ?? AUCUNE_TOUCHE;
}

/** FR-042 : au-delà de trente jours, une date ne dit plus rien d'utile. */
export const SEUIL_RIEN_DE_RECENT_MS = 30 * 24 * 60 * 60 * 1000;

/** Ce que dit une date qui a passé le seuil, ou qu'on n'a jamais eue. */
const RIEN_DE_RECENT = "rien de récent";

/**
 * L'âge d'une date TANT QU'ELLE COMPTE, `null` dès qu'elle ne dit plus rien :
 * inconnue, illisible, ou passé les trente jours. Un seul endroit tranche
 * FR-042, pour les deux dates et pour les deux usages (le mot, et la simple
 * question « est-elle récente ? ») - deux seuils écrits deux fois finiraient
 * par diverger.
 *
 * Une date « du futur » (horloges désynchronisées entre le poste qui a écrit
 * et le navigateur qui lit) donne un âge négatif : elle reste la plus récente
 * possible, jamais une raison de dire qu'il ne s'est rien passé. `dureeTexte`
 * la ramène ensuite à un écart nul.
 */
function ageSiRecente(date: string | null, maintenant: number): number | null {
  if (!date) return null;

  const instant = Date.parse(date);
  if (Number.isNaN(instant)) return null;

  const ecoule = maintenant - instant;
  return ecoule > SEUIL_RIEN_DE_RECENT_MS ? null : ecoule;
}

/**
 * FR-042 : cette date compte-t-elle encore ? Jugée SEULE, sans jamais regarder
 * l'autre - une zone écrite il y a quarante jours mais relue il y a vingt
 * minutes a une date morte et une date vivante, pas deux du même bord.
 *
 * Exactement trente jours compte encore : le seuil est « plus de 30 jours ».
 */
export function estRecente(date: string | null, maintenant: number): boolean {
  return ageSiRecente(date, maintenant) !== null;
}

/** Une date en mots, seule : « modifié il y a 2 j » ou « rien de récent en … ». */
function uneDateEnMots(
  date: string | null,
  maintenant: number,
  verbe: string,
  matiere: string,
): string {
  const ecoule = ageSiRecente(date, maintenant);
  return ecoule === null
    ? `${RIEN_DE_RECENT} en ${matiere}`
    : `${verbe} il y a ${dureeTexte(ecoule)}`;
}

/**
 * FR-041 et FR-042 : les deux dates d'une zone, en clair, membre par membre -
 * `["modifié il y a 2 j", "relu il y a 20 min"]`.
 *
 * Chaque date est jugée séparément : celle qui a passé les trente jours dit
 * « rien de récent en écriture » (ou « en lecture ») pendant que l'autre
 * s'affiche normalement. QUAND les deux sont dans ce cas, la zone entière dit
 * « rien de récent », et rend UN seul membre : deux dates muettes ne valent pas
 * deux mentions d'une absence, elles valent une phrase, sans aucune date.
 *
 * L'ordre écriture puis lecture ne varie pas : sur une carte, deux parcelles
 * voisines se comparent d'un coup d'œil, elles ne se relisent pas.
 *
 * Rendues séparées, et pas seulement collées, parce qu'une parcelle de treemap
 * est étroite : l'écran doit pouvoir passer à la ligne ENTRE les deux dates
 * plutôt qu'au milieu de l'une d'elles.
 */
export function datesEnMotsSeparees(dates: DatesZone, maintenant: number): string[] {
  const ecriture = estRecente(dates.ecriture, maintenant);
  const lecture = estRecente(dates.lecture, maintenant);

  if (!ecriture && !lecture) return [RIEN_DE_RECENT];

  return [
    uneDateEnMots(dates.ecriture, maintenant, "modifié", "écriture"),
    uneDateEnMots(dates.lecture, maintenant, "relu", "lecture"),
  ];
}

/**
 * Les mêmes deux dates d'un seul tenant - « modifié il y a 2 j, relu il y a
 * 20 min » : la forme de FR-041, celle que porte l'infobulle et celle que lit
 * une synthèse vocale, où rien ne passe à la ligne.
 */
export function datesEnMots(dates: DatesZone, maintenant: number): string {
  return datesEnMotsSeparees(dates, maintenant).join(", ");
}
