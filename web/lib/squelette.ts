/**
 * Le squelette de chargement du plan d'un repo (issue #12).
 *
 * Règle de fond : le squelette conserve les PROPORTIONS RÉELLES des parcelles,
 * pour que rien ne saute quand les données arrivent (critère 2). D'où le cache
 * local : chaque visite d'un repo grave la géométrie du plan qu'elle a rendue ;
 * la visite suivante repeint d'abord ces mêmes plaques en gris, puis les données
 * fraîches se posent dessus sans déplacer un pixel. Aucun tourniquet au milieu
 * du contenu, jamais : le squelette EST le plan, en attente de couleur.
 *
 * Sans cache (tout premier passage), on retombe sur un découpage par défaut —
 * un vrai treemap, aux tailles variées — plutôt qu'une grille régulière qui
 * trahirait un faux plan.
 */

import { decouper } from "./treemap";

/** La géométrie d'une plaque, en pourcentages du cadre. Rien d'autre : le
 *  squelette n'a besoin ni du nom ni du poids, seulement de la place. */
export type Plaque = { x: number; y: number; largeur: number; hauteur: number };

/** Préfixe des clés du cache local, une entrée par repo. */
export const PREFIXE_CACHE = "vibemap:plan:";

/**
 * Poids du squelette par défaut, sans cache. Décroissants et irréguliers : le
 * découpage donne alors des plaques de tailles franchement différentes, comme
 * un vrai repo, et non une mosaïque uniforme.
 */
const POIDS_DEFAUT = [38, 22, 14, 9, 7, 5, 3, 2];

/** Le squelette par défaut : un découpage proportionnel des poids ci-dessus. */
export function plaquesDefaut(): Plaque[] {
  return decouper(
    POIDS_DEFAUT.map((valeur, i) => ({ donnee: i, valeur })),
    { x: 0, y: 0, largeur: 100, hauteur: 100 },
  ).map(({ x, y, largeur, hauteur }) => ({ x, y, largeur, hauteur }));
}

/**
 * Les plaques à peindre pour le squelette : celles du cache si on en a, le
 * découpage par défaut sinon. Un cache vide vaut absence de cache — mieux vaut
 * un squelette plausible qu'un plan sans la moindre parcelle.
 */
export function plaquesSquelette(cache: Plaque[] | null): Plaque[] {
  return cache && cache.length > 0 ? cache : plaquesDefaut();
}

/**
 * Lit la géométrie du plan mise en cache pour ce repo. `null` quand il n'y a pas
 * de mémoire (rendu serveur), rien en cache, ou une entrée corrompue : dans tous
 * ces cas l'appelant retombe proprement sur le squelette par défaut.
 */
export function lirePlanCache(repoId: string, memoire: Storage | null): Plaque[] | null {
  if (!memoire) return null;
  const brut = memoire.getItem(PREFIXE_CACHE + repoId);
  if (!brut) return null;
  try {
    const valeur = JSON.parse(brut);
    return Array.isArray(valeur) ? (valeur as Plaque[]) : null;
  } catch {
    return null;
  }
}

/**
 * Grave la géométrie du plan pour ce repo. Sans mémoire (rendu serveur) ou si le
 * stockage refuse (quota, mode privé), on abandonne en silence : le cache est un
 * confort, jamais une dépendance.
 */
export function ecrirePlanCache(repoId: string, plaques: Plaque[], memoire: Storage | null): void {
  if (!memoire) return;
  try {
    memoire.setItem(PREFIXE_CACHE + repoId, JSON.stringify(plaques));
  } catch {
    // Quota atteint ou stockage indisponible : tant pis, le squelette par
    // défaut fera l'affaire à la prochaine visite.
  }
}
