/**
 * Le choix des trois écrans « avant que le produit soit plein » (issue #12) :
 * premier lancement, chargement, repo sans activité.
 *
 * La règle est simple et se décide sans réseau : l'accueil bascule en onboarding
 * tant qu'aucune machine n'est appairée ; le plan d'un repo montre l'état sans
 * activité quand il n'a ni module coloré ni ligne au journal. Un paramètre de
 * développement (`?demo=`) force chaque écran, pour les atteindre sans provoquer
 * la vraie panne (critère 6). Ces fonctions pures portent cette bascule, une
 * seule fois, pour que l'accueil et l'écran d'un repo ne puissent se contredire.
 */

/** Les écrans que le paramètre `?demo=` sait forcer en développement. */
export type EcranDemo = "onboarding" | "chargement" | "sans-activite";

/**
 * L'écran forcé lu dans le paramètre `?demo=`, ou `null` si rien de connu n'y
 * est demandé. Une valeur étrangère ne force rien : on retombe sur le réel.
 */
export function demoDemande(param: string | undefined): EcranDemo | null {
  if (param === "onboarding" || param === "chargement" || param === "sans-activite") {
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
