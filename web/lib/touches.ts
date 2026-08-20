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
 */

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
