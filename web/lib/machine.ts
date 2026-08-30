/**
 * La machine, telle que la fenêtre la voit (issues #65, #66, #67 - FR-017 à
 * FR-022, FR-055 à FR-057, FR-073, FR-080).
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
 * dit ici, ce sont les silences qu'aucun autre écran ne rattraperait : un
 * trousseau refusé, une machine révoquée, une machine qui vient d'être
 * redéclarée.
 *
 * ## Révoquée et redéclarée ne se ressemblent pas
 *
 * Ce sont les deux façons dont l'identité conservée cesse de correspondre, et
 * elles appellent des réponses opposées (issue #66). La révocation est une
 * décision de l'utilisateur : la machine s'arrête, et rien ne la remplace. La
 * disparition n'est la décision de personne - base remise à zéro, machine
 * supprimée, compte changé : la machine est réinscrite, et la carte se
 * repeuple. Les deux messages ne doivent jamais se confondre, sans quoi
 * l'utilisateur lirait « remplacée » là où il a coupé, ou l'inverse.
 */

/** Ce que le pont rend (`bureau/src/machine.rs`). Aucun jeton, jamais. */
export type EtatMachine =
  | { etat: "reprise"; machine_id: string; label: string }
  | { etat: "declaree"; machine_id: string; label: string }
  | { etat: "revoquee"; machine_id: string; label: string }
  | { etat: "redeclaree"; machine_id: string; label: string }
  | { etat: "trousseau_refuse"; machine_id: string; raison: string }
  | { etat: "echec"; raison: string };

const ETATS = [
  "reprise",
  "declaree",
  "revoquee",
  "redeclaree",
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
  return (
    etat !== null &&
    (etat.etat === "reprise" || etat.etat === "declaree" || etat.etat === "redeclaree")
  );
}

/**
 * Ce que la fenêtre affiche à propos de la machine, ou rien.
 *
 * `ton` sépare ce qui appelle un geste de ce qui n'en appelle aucun : une
 * redéclaration est un fait accompli, pas une panne, et l'annoncer comme une
 * alerte apprendrait à l'utilisateur à ignorer le bandeau. C'est lui qui décide
 * du rôle ARIA - un lecteur d'écran interrompt sur `alert`, jamais sur
 * `status`.
 */
export type AnnonceMachine =
  | { visible: false }
  | {
      visible: true;
      ton: "alerte" | "information";
      titre: string;
      explication: string;
      reessayer: boolean;
    };

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
      ton: "alerte",
      titre: "Le jeton de cette machine n'a pas pu être lu",
      explication: `${etat.raison} ${TROUSSEAU_REFUSE}`,
      reessayer: true,
    };
  }

  if (etat.etat === "revoquee") {
    return {
      visible: true,
      ton: "alerte",
      titre: `« ${etat.label} » a été révoquée`,
      explication:
        "Cette machine n'envoie plus rien, et Vibe Map ne la remplace pas par une autre : ce " +
        "serait défaire ce que vous venez de décider. Levez la révocation depuis la liste des " +
        "machines, puis réessayez.",
      reessayer: true,
    };
  }

  if (etat.etat === "redeclaree") {
    // FR-056 : le seul cas visible qui n'appelle aucun geste. La machine bat de
    // nouveau ; ce qui se dit ici, c'est POURQUOI la liste des machines et la
    // carte ne sont plus tout à fait celles d'hier - sans quoi l'utilisateur
    // verrait son historique disparaître sans une explication.
    return {
      visible: true,
      ton: "information",
      titre: `« ${etat.label} » a été redéclarée`,
      explication:
        "L'identifiant que ce Mac conservait ne correspondait plus à aucune machine du compte : " +
        "elle avait été supprimée, ou la base remise à zéro. Vibe Map l'a réinscrite sous une " +
        "nouvelle identité, sans rien vous demander. Vos dossiers surveillés sont inchangés, et " +
        "la carte se repeuple à mesure que le lecteur les relit.",
      reessayer: false,
    };
  }

  return {
    visible: true,
    ton: "alerte",
    titre: "Cette machine n'a pas pu être déclarée",
    explication: `${etat.raison} Tant qu'elle ne l'est pas, rien ne part de ce Mac.`,
    reessayer: true,
  };
}
