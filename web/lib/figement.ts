/**
 * L'état gelé d'une machine (issue #11).
 *
 * Au-delà de 90 s sans battement de cœur, une machine est déclarée injoignable :
 * l'écran garde son dernier état connu, mais désaturé et daté. Cette bascule ne
 * se décide pas en base — « depuis combien de temps » dépend de l'instant
 * présent, que seule l'horloge du navigateur tient. La base ne fait que porter
 * l'heure du dernier battement ; ces fonctions pures en tirent le figement.
 *
 * Une seule source de vérité pour le seuil et le calcul : le rail, l'écran d'un
 * repo et la liste des machines s'y accordent, sans pouvoir se contredire.
 */

/** Seuil au-delà duquel une machine est déclarée muette (spec, section 8). */
export const SEUIL_SILENCE_MS = 90_000;

/**
 * L'âge du dernier battement, en millisecondes. `null` quand la machine n'a
 * jamais battu : elle n'a pas d'âge, elle a une absence.
 */
function age(dernierBattement: string | null, maintenant: number): number | null {
  if (!dernierBattement) return null;
  return maintenant - Date.parse(dernierBattement);
}

/**
 * La machine est-elle figée ? Vrai au-delà du seuil de silence, et vrai aussi
 * quand elle n'a jamais battu : « jamais vue » n'est pas « vivante ». C'est le
 * seul juge de la bascule — l'activité du repo n'y entre pas, ce qui distingue
 * une machine muette (figée) d'un repo simplement au repos (critère 7).
 */
export function estFige(dernierBattement: string | null, maintenant: number): boolean {
  const ecoule = age(dernierBattement, maintenant);
  return ecoule === null || ecoule > SEUIL_SILENCE_MS;
}

/**
 * L'heure HH:MM du dernier battement : celle du dernier état connu, gravée sur
 * le plan gelé et sur chaque valeur du bandeau. Rendue dans le fuseau du
 * lecteur, comme le reste des heures de l'écran. `—` sans battement.
 */
export function heureFigement(dernierBattement: string | null): string {
  if (!dernierBattement) return "—";
  return new Date(dernierBattement).toLocaleTimeString("fr-FR", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Depuis combien de temps la machine se tait, en clair : « il y a 30 s », « il
 * y a 2 min », « il y a 2 h », « il y a 2 j ». Sans battement, « jamais vue ».
 */
export function depuisTexte(dernierBattement: string | null, maintenant: number): string {
  const ecoule = age(dernierBattement, maintenant);
  if (ecoule === null) return "jamais vue";

  const secondes = Math.max(0, Math.round(ecoule / 1000));
  if (secondes < 60) return `il y a ${secondes} s`;

  const minutes = Math.round(secondes / 60);
  if (minutes < 60) return `il y a ${minutes} min`;

  const heures = Math.round(minutes / 60);
  return heures < 24 ? `il y a ${heures} h` : `il y a ${Math.round(heures / 24)} j`;
}
