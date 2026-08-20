/**
 * Le pont des commandes locales, vu depuis l'interface (PRD-002, issue #68).
 *
 * L'application de bureau sert cette interface depuis la machine, sur son
 * origine locale fixe, et ouvre son pont à cette origine-là : celle de sa
 * propre fenêtre. La même interface ouverte dans un navigateur ordinaire ne
 * trouve rien ici - c'est voulu, et c'est ce que FR-060 demande de dire au lieu
 * d'un écran vide.
 *
 * Ce module est le SEUL endroit de `web/` qui touche au pont. Tout ce qui se
 * décide à partir de sa présence est pur et vit dans `lib/poste.ts` : c'est ce
 * qui rend la bascule de FR-060 éprouvable sans navigateur.
 */

import { invocateur, pontOuvert } from "@/lib/poste";

/** Les commandes que le pont expose, nommées une fois. */
export type CommandeLocale = "dossiers_surveilles" | "etat_du_lecteur" | "relancer_le_lecteur";

type Invoquer = (commande: CommandeLocale) => Promise<unknown>;

/**
 * La fonction d'appel du pont, ou `null` hors de l'application.
 *
 * `window` est absent au rendu serveur : on ne conclut donc rien avant d'être
 * dans le navigateur, et les écrans n'interrogent le pont que depuis un effet.
 */
export function pont(): Invoquer | null {
  if (typeof window === "undefined") return null;
  return pontOuvert(window) ? (invocateur(window) as Invoquer) : null;
}
