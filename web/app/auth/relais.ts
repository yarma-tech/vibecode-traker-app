/**
 * Le relais du retour d'autorisation, entre le navigateur du système et la
 * fenêtre de l'application (FR-072, issue #63).
 *
 * POURQUOI il existe : les deux clients chargent la même origine locale, mais
 * ils n'ont pas les mêmes cookies. Le vérificateur PKCE est resté dans la
 * fenêtre, qui est partie ; le navigateur, qui revient avec le code, ne peut
 * rien en faire. Le code lui est donc repris ici, et la fenêtre vient le
 * chercher - c'est ce qui lui permet de reprendre la main sans que
 * l'utilisateur ait à recopier quoi que ce soit.
 *
 * Une seule place, et à usage unique : il n'y a qu'une fenêtre, qu'un
 * utilisateur et qu'un aller-retour à la fois sur cette machine. Un retour
 * déposé s'y écrase l'ancien - le dernier départ est le seul qui compte - et
 * se retire dès qu'il est lu.
 *
 * Ce qui est déposé ne vaut rien pour personne d'autre : sans le vérificateur
 * PKCE de la fenêtre, un code d'autorisation ne s'échange pas. C'est ce qui
 * rend ce relais sûr sur une machine où d'autres programmes écoutent aussi la
 * boucle locale.
 */

import { relaisEncoreValable, type Retour } from "@/lib/autorisation";

type Depot = { retour: Retour; depose: number };

/**
 * La place vit sur `globalThis` et non dans une variable de module : en
 * développement, un module rechargé repartirait avec une place vide, et le
 * retour déposé une seconde plus tôt serait perdu.
 */
const CLE = Symbol.for("vibemap.relais.autorisation");

type Porteur = { [CLE]?: Depot | null };

function porteur(): Porteur {
  return globalThis as unknown as Porteur;
}

/** Dépose le retour pour la fenêtre. Le précédent, s'il y en avait un, saute. */
export function deposer(retour: Retour, maintenant: number = Date.now()): void {
  porteur()[CLE] = { retour, depose: maintenant };
}

/**
 * Retire le retour déposé, ou rend `null`. Un retour ne se lit qu'une fois :
 * un code d'autorisation ne s'échange qu'une fois, et le laisser en place
 * ferait rejouer indéfiniment un échange déjà refusé.
 */
export function retirer(maintenant: number = Date.now()): Retour | null {
  const depot = porteur()[CLE] ?? null;
  porteur()[CLE] = null;

  if (depot === null) return null;
  return relaisEncoreValable(depot.depose, maintenant) ? depot.retour : null;
}
