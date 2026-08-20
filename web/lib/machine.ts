/**
 * La machine, telle que la fenêtre la voit (issue #65, FR-017 à FR-022,
 * FR-055, FR-080).
 *
 * L'application déclare ce Mac elle-même, une session ouverte suffit, et il n'y
 * a plus de code d'appairage nulle part. Ce module ne fait pas la déclaration -
 * elle se joue dans `bureau/src/machine.rs`, avec le trousseau du système et la
 * base - : il dit ce que la fenêtre en montre.
 *
 * ## Ce que la fenêtre ne voit jamais
 *
 * Le jeton de la machine. Il va du réseau au trousseau du système sans passer
 * par ici, et aucun des cas ci-dessous n'a de champ où le loger. Deux identités
 * coexistent sur ce poste - la session de l'utilisateur, le jeton de la
 * machine - et elles ne se mélangent pas.
 *
 * ## Un bandeau qui ne parle que quand ça cloche
 *
 * Même règle que `bandeauDuLecteur` : une machine reprise ou déclarée n'appelle
 * aucun geste, et l'annoncer par-dessus chaque écran serait du bruit. La
 * déclaration se voit là où elle compte - dans la liste des machines. Ce qui se
 * dit ici, ce sont les trois silences qu'aucun autre écran ne rattraperait : un
 * trousseau refusé, une machine révoquée, une identité qui ne répond plus.
 */

/** Ce que le pont rend (`bureau/src/machine.rs`). Aucun jeton, jamais. */
export type EtatMachine =
  | { etat: "reprise"; machine_id: string; label: string }
  | { etat: "declaree"; machine_id: string; label: string }
  | { etat: "revoquee"; machine_id: string; label: string }
  | { etat: "inconnue"; machine_id: string }
  | { etat: "trousseau_refuse"; machine_id: string; raison: string }
  | { etat: "echec"; raison: string };

const ETATS = [
  "reprise",
  "declaree",
  "revoquee",
  "inconnue",
  "trousseau_refuse",
  "echec",
] as const;

/**
 * Ce que la fenêtre fait de la réponse du pont.
 *
 * `null` quand il n'y a rien à lire - hors de l'application, ou avant la
 * première réponse. Une réponse qu'on ne sait pas lire devient un échec, et non
 * `null` : la fenêtre annoncerait alors « tout va bien » à un poste dont on ne
 * sait rien.
 */
export function lireEtatMachine(reponse: unknown): EtatMachine | null {
  if (reponse === null || reponse === undefined) return null;
  if (typeof reponse !== "object") {
    return { etat: "echec", raison: "la réponse du poste n'a pas pu être lue." };
  }

  const lu = reponse as { etat?: unknown };
  const connu = ETATS.find((etat) => etat === lu.etat);
  if (connu === undefined) {
    return { etat: "echec", raison: "le poste a répondu quelque chose d'inattendu." };
  }

  return reponse as EtatMachine;
}

/**
 * La machine est-elle en état de faire tourner le lecteur ?
 *
 * C'est la seule question que l'écran a besoin de poser pour savoir s'il peut
 * (re)mettre le lecteur en marche : sans jeton au trousseau, il ne démarrerait
 * pas, et le relancer en boucle ne ferait qu'ouvrir des boîtes de dialogue.
 */
export function laMachineEstPrete(etat: EtatMachine | null): boolean {
  return etat !== null && (etat.etat === "reprise" || etat.etat === "declaree");
}

/** Ce que la fenêtre affiche à propos de la machine, ou rien. */
export type AnnonceMachine =
  | { visible: false }
  | { visible: true; titre: string; explication: string; reessayer: boolean };

/**
 * FR-080 : le refus du trousseau est un passage ATTENDU, pas une anomalie.
 *
 * macOS redemande son autorisation dès que le programme change, et l'application
 * est un exécutable différent du binaire en ligne de commande : tout poste déjà
 * relié avant ce chantier rencontrera cette boîte de dialogue une fois. Le
 * message doit donc dire pourquoi elle apparaît, et proposer de réessayer - et
 * surtout dire ce qui N'A PAS eu lieu : aucune machine n'a été déclarée à la
 * place, la liste n'a gagné personne.
 */
const TROUSSEAU_REFUSE =
  "Vibe Map range le jeton de cette machine dans le trousseau du système, à la même place que " +
  "le programme en ligne de commande. macOS demande son autorisation à chaque nouvelle version " +
  "de l'application : c'est attendu. Aucune machine n'a été déclarée à la place. Accordez " +
  "l'accès, puis réessayez.";

export function annonceDeLaMachine(etat: EtatMachine | null): AnnonceMachine {
  if (etat === null) return { visible: false };

  if (etat.etat === "reprise" || etat.etat === "declaree") {
    // Rien à dire, et rien à faire : la machine bat, et la liste des machines
    // le montre déjà.
    return { visible: false };
  }

  if (etat.etat === "trousseau_refuse") {
    return {
      visible: true,
      titre: "Le jeton de cette machine n'a pas pu être lu",
      explication: `${etat.raison} ${TROUSSEAU_REFUSE}`,
      reessayer: true,
    };
  }

  if (etat.etat === "revoquee") {
    return {
      visible: true,
      titre: `« ${etat.label} » a été révoquée`,
      explication:
        "Cette machine n'envoie plus rien, et Vibe Map ne la remplace pas par une autre : ce " +
        "serait défaire ce que vous venez de décider. Levez la révocation depuis la liste des " +
        "machines, puis réessayez.",
      reessayer: true,
    };
  }

  if (etat.etat === "inconnue") {
    return {
      visible: true,
      titre: "Cette machine n'est plus reconnue",
      explication:
        "L'identifiant que ce Mac conserve ne correspond à aucune machine du compte : elle a " +
        "été supprimée, ou la base a été remise à zéro. Rien n'a été redéclaré en silence. " +
        "Réessayez pour voir si elle revient.",
      reessayer: true,
    };
  }

  return {
    visible: true,
    titre: "Cette machine n'a pas pu être déclarée",
    explication: `${etat.raison} Tant qu'elle ne l'est pas, rien ne part de ce Mac.`,
    reessayer: true,
  };
}
