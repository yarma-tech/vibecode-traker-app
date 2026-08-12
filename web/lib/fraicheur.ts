/**
 * La fraîcheur de l'espace projet (F14, issue #39).
 *
 * « Un tableau figé est indiscernable d'un tableau stable, et un outil qui
 * ment une fois n'est plus consulté » (PRD-001, F14) : avant de dire « rien à
 * faire », l'espace projet vérifie d'abord qu'il a des nouvelles récentes du
 * dépôt.
 *
 * Deux colonnes existent déjà et suffisent, rien de neuf en base (conception
 * du 2026-08-11, section 7) : `repos.scanned_at` (la dernière cartographie
 * complète, largement plus espacée que deux minutes) et
 * `machines.last_seen_at` (le battement de cœur du daemon, bien plus
 * fréquent). Aucune des deux seule ne suffit : `scanned_at` seul ferait
 * passer le tableau en « muet » entre deux cartographies alors que le daemon
 * répond très bien (son battement est plus récent) ; `last_seen_at` seul
 * ignorerait une cartographie qui vient de réussir sur une machine dont le
 * tout premier battement n'est pas encore arrivé. Le signal retenu est donc
 * le plus récent des deux.
 *
 * Comme `figement.ts`, ces fonctions sont pures : jamais d'horloge globale
 * lue ici, seulement un `maintenant` reçu en paramètre - un test qui
 * dépendrait de l'heure réelle deviendrait rouge un jour sans que rien n'ait
 * changé.
 */

import { dureeTexte } from "./figement";

/** Frais si le dernier signal date de moins de deux minutes (spec, section 7). */
export const SEUIL_FRAICHEUR_MS = 2 * 60 * 1000;

/**
 * L'âge du dernier signal reçu pour ce dépôt, en millisecondes : le plus
 * récent de `scanned_at` et `last_seen_at`, comparé à `maintenant`. Peut être
 * négatif si l'horloge de la machine qui a écrit l'un des deux avance sur
 * celle du navigateur (deux Macs désynchronisés) - un signal « du futur »
 * reste alors le plus frais possible, jamais une raison de le dire muet.
 */
export function ageSignal(
  scannedAt: string,
  dernierBattement: string | null,
  maintenant: number,
): number {
  const dates = [Date.parse(scannedAt)];
  if (dernierBattement) dates.push(Date.parse(dernierBattement));
  return maintenant - Math.max(...dates);
}

/**
 * FR-049 : l'espace projet est-il frais ? Le seul juge est l'âge du dernier
 * signal - jamais le contenu du tableau, qui pourrait être vide aussi bien
 * parce que tout est fait que parce que plus rien ne remonte (FR-050, voir
 * `etatTableau`).
 */
export function estFrais(
  scannedAt: string,
  dernierBattement: string | null,
  maintenant: number,
): boolean {
  return ageSignal(scannedAt, dernierBattement, maintenant) < SEUIL_FRAICHEUR_MS;
}

/**
 * FR-049 : la durée à afficher après « Muet depuis » - « 3 h » pour un daemon
 * arrêté depuis trois heures. Un appel légitime même quand `estFrais` rend
 * vrai (un âge négatif se ramène silencieusement à 0 par `dureeTexte`), mais
 * qui n'a alors aucune raison d'être affiché par l'appelant.
 */
export function dureeDepuisSignal(
  scannedAt: string,
  dernierBattement: string | null,
  maintenant: number,
): string {
  return dureeTexte(ageSignal(scannedAt, dernierBattement, maintenant));
}

/**
 * Le compte de cartes d'un tableau, par colonne - tout ce dont `etatTableau`
 * a besoin pour distinguer ses trois états vides (FR-050, FR-051). Brut,
 * explorations comprises : c'est aussi ce que chaque colonne affiche déjà
 * (« Rien ici pour l'instant » se décide sur `rangees[statut].length`, jamais
 * sur un total qui les exclurait) - une exploration reste une carte bien
 * visible dans sa colonne, même si FR-047 l'exclut du compteur affiché à côté
 * du titre de la colonne.
 */
export type CompteColonnes = { todo: number; doing: number; done: number };

/**
 * Les trois états vides du tableau (PRD F14, FR-050, FR-051), et un
 * quatrième qui n'en est pas un : un tableau qui montre encore du travail
 * (« normal »). Le dessin exact de chacun reste à trancher en revue humaine
 * (issue #43, conception section 13) - cette fonction ne fait que choisir
 * LEQUEL montrer, jamais son gabarit visuel.
 *
 * - « neuf » : aucune carte n'a jamais été créée ici. Indépendant de la
 *   fraîcheur - un dépôt qui vient d'être cartographié a un tableau vierge
 *   que le daemon soit bavard ou non.
 * - « acheve » ou « muet » : plus aucune carte en attente ni en cours - tout
 *   ce qui reste est en « Terminé » (ou il n'y a jamais rien eu d'autre). Un
 *   tableau qui se tait ressemble EXACTEMENT à un tableau qui a fini
 *   (FR-050) : seule la fraîcheur du signal sépare les deux lectures.
 */
export type EtatTableau = "neuf" | "acheve" | "muet" | "normal";

export function etatTableau(compte: CompteColonnes, frais: boolean): EtatTableau {
  if (compte.todo + compte.doing + compte.done === 0) return "neuf";
  if (compte.todo === 0 && compte.doing === 0) return frais ? "acheve" : "muet";
  return "normal";
}

/**
 * FR-051 : le clin d'œil (« Terminé restera vide malgré l'historique du
 * dépôt ») n'accompagne l'état « neuf » que si la fraîcheur le permet - sans
 * quoi cette promesse d'un automatisme vivant serait tenue par un tableau qui
 * ne peut justement plus prouver qu'il l'est.
 */
export function clinDoeilVisible(etat: EtatTableau, frais: boolean): boolean {
  return etat === "neuf" && frais;
}
