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
export type CommandeLocale =
  | "dossiers_surveilles"
  | "ajouter_un_dossier"
  | "retirer_un_dossier"
  | "redemander_l_autorisation"
  | "etat_du_lecteur"
  | "relancer_le_lecteur"
  | "arreter_le_lecteur"
  | "ouvrir_l_autorisation"
  | "declarer_la_machine"
  | "revenir_au_premier_plan";

/**
 * Les commandes ne reçoivent presque jamais rien - ce sont des gestes nommés.
 * Deux font exception, et pour la même raison : la fenêtre détient quelque
 * chose que l'application ne peut pas obtenir autrement.
 *
 * - `ouvrir_l_autorisation` reçoit l'adresse d'autorisation à ouvrir dehors :
 *   la fenêtre est seule à savoir la demander à Supabase, et l'application est
 *   seule à pouvoir ouvrir le navigateur du système (FR-071). Elle n'accepte
 *   pas n'importe quelle adresse pour autant : `bureau/src/autorisation.rs` dit
 *   laquelle, et refuse le reste.
 * - `declarer_la_machine` reçoit la session de l'utilisateur, qui vit dans les
 *   cookies de cette fenêtre et nulle part ailleurs (FR-017). Rien ne remonte
 *   en sens inverse : le jeton de la machine va du réseau au trousseau du
 *   système sans jamais passer par ici.
 */
type Invoquer = (commande: CommandeLocale, arguments_?: Record<string, unknown>) => Promise<unknown>;

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
