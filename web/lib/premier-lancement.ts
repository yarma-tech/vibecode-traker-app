/**
 * Ce que dit l'écran de premier lancement, et où il envoie (issue #83).
 *
 * FR-023 et FR-024 : plus aucun code d'appairage à créer, plus aucune commande
 * à recopier dans un terminal. FR-025 : tant qu'aucune machine n'est déclarée,
 * l'écran dirige vers l'installation puis l'ouverture de l'application de
 * bureau, jamais vers un terminal. FR-058 : il porte un lien vers la page des
 * publications GitHub du dépôt, atteignable en un clic.
 *
 * POURQUOI ce texte vit ici et non dans le composant : c'est le contenu de
 * l'écran qui porte l'exigence, pas son balisage. Écrit ici, il s'éprouve mot à
 * mot - aucun code, aucune commande, un lien qui mène bien aux publications -
 * sans avoir à rendre un arbre React pour le lire.
 */

/** Le dépôt, tel que GitHub le nomme. */
export const DEPOT = "yarma-tech/vibecode-traker-app";

/**
 * La page des publications du dépôt : l'emplacement où l'intégration continue
 * dépose l'application (FR-067), et le seul que ce PRD retienne (FR-058).
 *
 * La liste, et non la dernière en date : `/releases/latest` ne mène nulle part
 * tant qu'aucune version n'est publiée, là où `/releases` dit toujours la
 * vérité, fût-elle « rien encore ».
 */
export const PUBLICATIONS = `https://github.com/${DEPOT}/releases`;

/** Une étape de l'écran : le geste à faire, et ce qui s'ensuit. */
export type Etape = {
  readonly geste: string;
  readonly suite: string;
};

/**
 * Les trois étapes, dans l'ordre où on les fait. Aucune ne passe par un
 * terminal : la première télécharge, la deuxième ouvre, la troisième n'est pas
 * un geste du tout - c'est la carte qui s'allume seule.
 */
export const ETAPES: readonly Etape[] = [
  {
    geste: "Télécharge l'application de bureau",
    suite:
      "Vibe Map pour macOS, depuis la page des publications du dépôt. Elle embarque le lecteur : rien d'autre à installer.",
  },
  {
    geste: "Glisse-la dans Applications, puis ouvre-la",
    suite:
      "Elle te demande de te connecter, puis relie cette machine à ton compte toute seule. Rien à recopier d'un écran à l'autre.",
  },
  {
    geste: "La carte s'allume toute seule",
    suite:
      "Ici même, dès le premier signe de vie de la machine. Rien à cliquer, tu peux laisser cet onglet ouvert.",
  },
];
