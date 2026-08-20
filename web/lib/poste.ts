/**
 * Les faits du poste, tels que l'écran Réglages les lit (issue #68).
 *
 * FR-059 partage les sources, et tout ce fichier découle de ce partage : les
 * dossiers surveillés, leur compte de dépôts, leur lisibilité et leur
 * autorisation viennent du **pont** de l'application de bureau - ce sont des
 * faits de cette machine, que la base ne porte pas et ne portera pas. L'heure
 * de la dernière cartographie, elle, se lit **en base** : elle y est déjà, et
 * c'est ce qui la rend lisible depuis un autre appareil et par-delà les
 * fermetures de l'application.
 *
 * FR-060 en est la conséquence directe : la même interface ouverte dans un
 * navigateur ordinaire ne trouve aucun pont. Elle doit alors remplacer cette
 * seule section par une mention, jamais afficher une liste vide - qui se
 * lirait « rien n'est surveillé » - ni une erreur. Le reste de l'écran, qui
 * vient de la base, continue de s'afficher.
 *
 * Comme `figement.ts` et `fraicheur.ts`, ces fonctions sont pures : jamais
 * d'horloge lue ici, seulement un `maintenant` reçu en paramètre.
 */

import { dureeTexte } from "./figement";

/* ---------- ce que le pont rend ---------- */

/**
 * Pourquoi un dossier surveillé se lit, ou ne se lit pas. Les quatre cas sont
 * ceux que l'application distingue (`bureau/src/dossiers.rs`) : ils ne se
 * corrigent pas de la même façon, et les confondre renverrait l'utilisateur
 * chercher lui-même lequel il a sous les yeux.
 */
export type Lisibilite = "lisible" | "introuvable" | "autorisation_refusee" | "pas_un_dossier";

export type DossierSurveille = {
  /** Le chemin tel qu'il est écrit dans la configuration - `~/Developer`. */
  chemin: string;
  /** Le même, déplié. Il ne sert qu'à lever le doute que le `~` laisse. */
  emplacement: string;
  lisibilite: Lisibilite;
  /**
   * `null` dès que le dossier ne se lit pas : un zéro se lirait comme un
   * dossier bien ouvert et vide, ce qui est une tout autre affaire (FR-029).
   */
  depots: number | null;
};

/**
 * Ce que le poste dit qu'il y a à cartographier (FR-087).
 *
 * Un fait du poste, et il le reste : quand une cartographie a abouti sans
 * trouver aucun dépôt, la base ne porte AUCUNE heure - il n'y a pas de dépôt
 * pour en porter une. Sans ce fait, l'écran conclurait « jamais cartographié »
 * à un poste qui l'a été. « inconnu » se garde pour un dossier qu'on n'a pas pu
 * ouvrir : il porte peut-être cinquante dépôts.
 */
export type ADecouvrir = "des_depots" | "rien" | "inconnu";

/** La réponse du pont à « lister les dossiers surveillés ». */
export type Surveillance =
  | { etat: "lue"; dossiers: DossierSurveille[]; a_decouvrir?: ADecouvrir }
  | { etat: "sans_configuration"; raison: string };

/**
 * Ce que la fenêtre a obtenu en demandant. `null` dit qu'il n'y a pas de pont
 * à qui demander - le navigateur ordinaire de FR-060. « sans réponse » dit
 * l'inverse : le pont est là, et l'appel n'a rien rendu. Les confondre ferait
 * annoncer « ces réglages n'existent que dans l'application » à quelqu'un qui
 * est justement dedans.
 */
export type ReponseDuPont = Surveillance | { etat: "sans_reponse"; raison: string } | null;

/* ---------- le pont est-il là ? (FR-060) ---------- */

/**
 * Le pont des commandes locales répond-il dans cette fenêtre ?
 *
 * La question ne se pose qu'à la seule marque qui compte : la fonction
 * d'appel existe, ou elle n'existe pas. Ni l'agent du navigateur, ni le nom de
 * l'hôte, ni le port ne disent quoi que ce soit - l'interface est servie sur
 * la même origine locale dans les deux cas, et c'est justement pour cela
 * qu'elle a été choisie. Seul le pont fait la différence.
 */
export function pontOuvert(fenetre: unknown): boolean {
  return typeof invocateur(fenetre) === "function";
}

/**
 * La fonction d'appel du pont, ou `null`. Extraite à part parce que la
 * détection et l'appel doivent juger sur exactement la même chose : un écran
 * qui déciderait « le pont est là » sur un critère et appellerait sur un autre
 * finirait par lancer une erreur dans un navigateur ordinaire.
 */
export function invocateur(fenetre: unknown): unknown {
  if (!fenetre || typeof fenetre !== "object") return null;
  const tauri = (fenetre as { __TAURI__?: { core?: { invoke?: unknown } } }).__TAURI__;
  return tauri?.core?.invoke ?? null;
}

/* ---------- ce que la section des dossiers montre ---------- */

/**
 * Ce que l'écran affiche à la place de la liste, ou la liste elle-même.
 *
 * - « hors application » : pas de pont. La mention de FR-060, jamais un vide.
 * - « sans configuration » : le pont est là, mais il ne trouve pas la
 *   configuration du lecteur. « On ne sait pas ce qui est surveillé » n'est pas
 *   « rien n'est surveillé ».
 * - « aucun » : le pont a lu la configuration, et elle ne surveille rien. C'est
 *   le seul vrai vide de cette section.
 * - « sans réponse » : le pont est là et n'a rien rendu. Rare, mais il ne doit
 *   pas se déguiser en navigateur ordinaire.
 * - « liste » : les dossiers, tels qu'ils sont.
 */
export type SectionDossiers =
  | { quoi: "hors_application" }
  | { quoi: "sans_configuration"; raison: string }
  | { quoi: "sans_reponse"; raison: string }
  | { quoi: "aucun" }
  | { quoi: "liste"; dossiers: DossierSurveille[] };

/**
 * Laquelle des cinq. `reponse` vaut `null` quand il n'y a pas de pont à qui
 * demander - c'est le cas du navigateur ordinaire, et c'est le seul qui bascule
 * sur la mention de FR-060.
 */
export function sectionDossiers(reponse: ReponseDuPont): SectionDossiers {
  if (reponse === null) return { quoi: "hors_application" };
  if (reponse.etat === "sans_reponse") {
    return { quoi: "sans_reponse", raison: reponse.raison };
  }
  if (reponse.etat === "sans_configuration") {
    return { quoi: "sans_configuration", raison: reponse.raison };
  }
  if (reponse.dossiers.length === 0) return { quoi: "aucun" };
  return { quoi: "liste", dossiers: reponse.dossiers };
}

/**
 * Ce que le poste dit qu'il y a à cartographier, ou `null` quand personne n'est
 * là pour le dire (FR-087).
 *
 * `null` hors de l'application - FR-060 retire les faits du poste -, et `null`
 * aussi quand le pont n'a pas su lire la configuration : conclure « rien à
 * cartographier » d'une liste qu'on n'a pas pu lire serait affirmer ce qu'on ne
 * sait pas. L'écran s'en tient alors à ce que la base porte.
 */
export function aDecouvrirDuPoste(reponse: ReponseDuPont): ADecouvrir | null {
  if (reponse === null || reponse.etat !== "lue") return null;
  return reponse.a_decouvrir ?? null;
}

/* ---------- la ligne d'un dossier (FR-028, FR-029) ---------- */

/**
 * Ce qu'une ligne dit d'un dossier : son compte de dépôts, ce qui l'explique
 * quand il est nul, et le signal qui remplace le tout quand le dossier ne se lit
 * pas.
 *
 * `signal` est `null` pour un dossier lisible : c'est ce qui distingue une
 * ligne ordinaire d'une ligne à signaler (FR-029). Le texte porte le sens à
 * lui seul - jamais la couleur seule.
 *
 * `explication` ne paraît que sur un compte nul (FR-076) : c'est le seul cas où
 * le compte, à lui seul, ne dit pas pourquoi. Sur une ligne qui affiche des
 * dépôts, elle serait du bruit.
 */
export type LigneDossier = {
  compte: string | null;
  explication: string | null;
  signal: string | null;
  /**
   * FR-061 : cette ligne porte-t-elle un bouton pour redemander l'autorisation ?
   *
   * Sur elle seule. Un dossier renommé ou un chemin qui n'est pas un dossier ne
   * se réparent pas au sélecteur du système : le bouton y serait un faux espoir,
   * et il ferait douter du vrai geste - retirer la ligne, ou remettre le dossier
   * à sa place.
   */
  redemande: boolean;
};

const SIGNAUX: Record<Exclude<Lisibilite, "lisible">, string> = {
  introuvable: "Dossier introuvable",
  // FR-061 nomme la cause, et non le symptôme : ce n'est pas un accès qui a
  // échoué, c'est une autorisation que le système n'a pas encore donnée. Le mot
  // dit du même coup quel bouton est juste à côté.
  autorisation_refusee: "Autorisation manquante",
  pas_un_dossier: "Pas un dossier",
};

/**
 * FR-061 : ce qui manque, et le geste qui le rend.
 *
 * POURQUOI cette phrase-là : « Autorisation manquante » nomme l'état, pas la
 * cause. Sur macOS, `~/Documents`, `~/Desktop`, `~/Downloads` et les volumes
 * externes sont fermés tant que l'utilisateur n'a pas accordé l'accès, et un
 * accès accordé peut être repris plus tard. Sans cette phrase, un dossier plein
 * de dépôts se lirait comme un dossier vide, et l'utilisateur irait chercher la
 * panne dans ses dépôts.
 *
 * Elle dit aussi ce que le bouton va faire - rouvrir le sélecteur sur ce même
 * dossier -, parce qu'un sélecteur qui s'ouvre sans prévenir se referme sans
 * qu'on ait compris ce qu'il attendait.
 */
const AUTORISATION_MANQUANTE =
  "Le système ne laisse pas Vibe Map ouvrir ce dossier, et ses dépôts ne peuvent donc pas être " +
  "cartographiés. Redemandez l'autorisation : le sélecteur du système s'ouvre, et désigner ce " +
  "même dossier l'accorde.";

/**
 * FR-076 : ce qui a été cherché, en clair.
 *
 * POURQUOI cette phrase-là : la cartographie ne regarde que les enfants DIRECTS
 * d'un dossier surveillé. Le geste le plus probable au sélecteur du système -
 * ouvrir le dossier d'un projet et le choisir - désigne donc un dossier qui est
 * lui-même un dépôt, et ne remonte rien. « aucun dépôt » tout seul laisserait
 * chercher la panne ailleurs ; nommer le critère et le geste qui corrige est ce
 * qui fait la différence entre une ligne muette et une réponse.
 */
const RIEN_TROUVE =
  "Un dépôt est un dossier qui contient un .git, parmi les enfants directs de celui-ci. " +
  "Si ce dossier est lui-même un dépôt, ajoutez plutôt le dossier qui le contient.";

/**
 * FR-074 : tant qu'aucune cartographie n'a jamais abouti, un compte de zéro se
 * lirait « ce dossier est vide » alors que rien n'a encore été regardé. La ligne
 * annonce une attente à la place.
 */
const ATTENTE = "en attente de la première cartographie";

/**
 * `silence` est ce que l'écran sait de la dernière cartographie : il change ce
 * qu'une ligne peut honnêtement dire de son compte. Sur un poste jamais
 * cartographié, aucun compte ne veut encore dire quoi que ce soit.
 */
export function ligneDossier(dossier: DossierSurveille, silence: Silence): LigneDossier {
  if (dossier.lisibilite !== "lisible" || dossier.depots === null) {
    // Pas de compte du tout : un « 0 dépôt » sur une ligne signalée
    // enverrait chercher des dépôts absents d'un dossier qui, lui, est absent.
    // Le signal vaut avant tout le reste, y compris avant l'attente : un dossier
    // introuvable le restera, cartographie ou pas.
    const cause = signalable(dossier.lisibilite);
    const manque = cause === "autorisation_refusee";
    return {
      compte: null,
      // FR-061 : le seul des trois signaux qui appelle une explication, parce
      // qu'il est le seul dont la cause est invisible - le dossier est là, plein,
      // et rien à l'écran ne dirait pourquoi il ne rend rien.
      explication: manque ? AUTORISATION_MANQUANTE : null,
      signal: SIGNAUX[cause],
      redemande: manque,
    };
  }

  if (silence === "jamais") {
    return { compte: ATTENTE, explication: null, signal: null, redemande: false };
  }

  return {
    compte: compteDepots(dossier.depots),
    explication: dossier.depots === 0 ? RIEN_TROUVE : null,
    signal: null,
    redemande: false,
  };
}

/**
 * Un dossier lisible dont le compte manque quand même : le pont n'en produit
 * pas, mais l'écran ne doit pas se taire si cela arrivait. On le range avec ce
 * qui ne se lit pas, ce qui est vrai de son compte.
 */
function signalable(lisibilite: Lisibilite): Exclude<Lisibilite, "lisible"> {
  return lisibilite === "lisible" ? "pas_un_dossier" : lisibilite;
}

/**
 * Le compte, en clair. Zéro se dit « aucun dépôt » et non « 0 dépôt » : c'est
 * une phrase, pas une case de tableau, et c'est souvent la réponse à
 * « pourquoi ce dépôt n'apparaît pas ».
 */
export function compteDepots(depots: number): string {
  if (depots === 0) return "aucun dépôt";
  return depots === 1 ? "1 dépôt" : `${depots} dépôts`;
}

/* ---------- ajouter un dossier (FR-031, FR-034, FR-036) ---------- */

/**
 * Ce que le pont rend après « ajouter un dossier »
 * (`bureau/src/dossiers.rs`).
 *
 * Le chemin ne part jamais d'ici : c'est l'application qui ouvre le sélecteur
 * du système et demande à l'utilisateur quel dossier il désigne. L'écran
 * demande un geste, il ne désigne rien.
 *
 * `null` dit qu'il n'y avait pas de pont à qui demander - hors de
 * l'application, le bouton n'existe pas.
 */
/**
 * Comment le dossier choisi rencontre un dossier déjà surveillé (FR-075). Les
 * trois se refusent, et ils ne se corrigent pas de la même façon : d'où trois
 * phrases, et non « déjà surveillé » pour tout le monde.
 */
export type Emboitement = "meme" | "contenu" | "contient";

export type ReponseAjout =
  | { issue: "ajoute"; chemin: string; surveillance: Surveillance }
  | { issue: "annule" }
  | { issue: "refuse"; chemin: string; deja: string; cas: Emboitement }
  | { issue: "echoue"; raison: string }
  | null;

/** Ce que l'écran annonce après un ajout, ou rien. */
export type Annonce = { ton: "succes" | "echec"; texte: string };

/**
 * Ce que l'écran fait de la réponse : ce qu'il annonce, et la liste qu'il
 * affiche désormais.
 *
 * `surveillance` vaut `null` quand la liste ne bouge pas - un sélecteur refermé
 * ou une écriture refusée laissent l'écran exactement tel qu'il était. La
 * remplacer par une liste vide dirait « plus rien n'est surveillé », ce qui
 * serait faux.
 */
export type Suite = { annonce: Annonce | null; surveillance: Surveillance | null };

export type SuiteAjout = Suite;

export function suiteDeLAjout(reponse: ReponseAjout): SuiteAjout {
  // Pas de pont, ou un sélecteur refermé sans choix : il ne s'est rien passé,
  // et un écran qui annoncerait quelque chose ferait passer un geste repris
  // pour un incident.
  if (reponse === null || reponse.issue === "annule") {
    return { annonce: null, surveillance: null };
  }

  if (reponse.issue === "refuse") {
    // Rien n'a été écrit, et la liste affichée est déjà la bonne : c'est même
    // tout le sens du refus. La remplacer ferait clignoter une section qui n'a
    // pas bougé.
    return {
      annonce: { ton: "echec", texte: refusDAjout(reponse) },
      surveillance: null,
    };
  }

  if (reponse.issue === "echoue") {
    return {
      annonce: {
        ton: "echec",
        texte: `Le dossier n'a pas pu être ajouté : ${reponse.raison}`,
      },
      surveillance: null,
    };
  }

  // FR-034 : le dossier est cartographié sans attendre le tour suivant. Le dire
  // est ce qui évite d'aller vérifier dans un fichier, ou de recommencer parce
  // que les dépôts ne sont pas déjà là.
  return {
    annonce: {
      ton: "succes",
      texte: `« ${reponse.chemin} » est surveillé. Ses dépôts apparaissent d'ici une minute.`,
    },
    surveillance: reponse.surveillance,
  };
}

/**
 * Le refus, en clair (FR-037, FR-075).
 *
 * Trois exigences tiennent ensemble dans chacune de ces phrases : elle NOMME le
 * dossier déjà surveillé qui s'oppose - « déjà surveillé » sans dire lequel
 * enverrait ouvrir le fichier de configuration, ce que FR-036 interdit -, elle
 * dit pourquoi c'en est un, et elle dit quoi faire. Les trois cas ne se
 * corrigent pas de la même façon : le même dossier n'appelle aucun geste, un
 * dossier contenu s'échange contre un autre choix, un dossier qui en contient un
 * déjà surveillé demande de retirer d'abord la racine étroite.
 */
export function refusDAjout(refus: {
  chemin: string;
  deja: string;
  cas: Emboitement;
}): string {
  if (refus.cas === "meme") {
    return `« ${refus.deja} » est déjà surveillé : ses dépôts sont déjà cartographiés. Rien n'a été ajouté.`;
  }
  if (refus.cas === "contenu") {
    return (
      `« ${refus.chemin} » est déjà couvert par « ${refus.deja} », qui est surveillé : ses dépôts ` +
      `remontent déjà de là. Choisissez un dossier situé en dehors de « ${refus.deja} ».`
    );
  }
  return (
    `« ${refus.chemin} » contient « ${refus.deja} », déjà surveillé : les mêmes dépôts seraient ` +
    `cartographiés deux fois. Retirez d'abord « ${refus.deja} » de la surveillance.`
  );
}

/* ---------- retirer un dossier (FR-032, FR-035, FR-036) ---------- */

/**
 * Ce que le pont rend après « retirer un dossier » (`bureau/src/dossiers.rs`).
 *
 * `null` dit qu'il n'y avait pas de pont à qui demander - hors de
 * l'application, la liste elle-même n'est pas là.
 */
export type ReponseRetrait =
  | { issue: "retire"; chemin: string; surveillance: Surveillance }
  | { issue: "inconnu"; chemin: string; surveillance: Surveillance }
  | { issue: "echoue"; raison: string }
  | null;

/**
 * FR-035, et c'est tout le sujet de cette tranche : retirer un dossier n'efface
 * rien de ce qui a déjà été observé.
 *
 * POURQUOI l'annonce doit le dire noir sur blanc : « retiré » tout seul se lit
 * comme un effacement, et personne ne retire un dossier de bon coeur s'il croit
 * emporter avec lui des semaines de carte. La phrase nomme donc le sort exact de
 * ses dépôts - au catalogue, figés à leur dernière heure connue, ce que la
 * fraîcheur (`fraicheur.ts`) rend déjà lisible sur l'accueil - et le geste qui
 * revient en arrière.
 */
const RIEN_N_EST_EFFACE =
  "Ses dépôts restent au catalogue, figés à leur dernière heure de cartographie ; rajoutez ce " +
  "dossier pour qu'ils reprennent.";

/**
 * Ce que l'écran fait de la réponse. Même forme que pour l'ajout : ce qu'il
 * annonce, et la liste qu'il affiche désormais.
 *
 * `surveillance` accompagne les deux issues où le poste a relu sa liste - le
 * retrait fait, et le retrait sans objet. Ce second cas la rend justement parce
 * que la liste affichée avait vieilli : c'est la seule explication possible d'un
 * dossier qu'on croyait surveillé et qui ne l'est plus.
 */
export function suiteDuRetrait(reponse: ReponseRetrait): Suite {
  if (reponse === null) return { annonce: null, surveillance: null };

  if (reponse.issue === "echoue") {
    return {
      annonce: {
        ton: "echec",
        texte: `Le dossier n'a pas pu être retiré : ${reponse.raison}`,
      },
      // Rien n'a été écrit : la liste affichée est encore la bonne, et la
      // remplacer ferait clignoter une section qui n'a pas bougé.
      surveillance: null,
    };
  }

  if (reponse.issue === "inconnu") {
    return {
      annonce: {
        ton: "echec",
        texte:
          `« ${reponse.chemin} » n'était plus dans la liste des dossiers surveillés : rien n'a ` +
          "été retiré. La liste ci-dessous est celle du poste.",
      },
      surveillance: reponse.surveillance,
    };
  }

  return {
    annonce: {
      ton: "succes",
      texte: `« ${reponse.chemin} » n'est plus surveillé. ${RIEN_N_EST_EFFACE}`,
    },
    surveillance: reponse.surveillance,
  };
}

/* ---------- redemander une autorisation manquante (FR-061) ---------- */

/**
 * Ce que le pont rend après « redemander l'autorisation »
 * (`bureau/src/dossiers.rs`).
 *
 * Cinq issues, cinq gestes différents. Le chemin ne part toujours pas d'ici :
 * l'écran nomme la LIGNE dont il redemande l'autorisation, et c'est
 * l'application qui rouvre le sélecteur du système sur ce dossier.
 */
export type ReponseReautorisation =
  | { issue: "annulee" }
  | { issue: "inconnu"; chemin: string; surveillance: Surveillance }
  | { issue: "autre_dossier"; attendu: string; choisi: string }
  | { issue: "accordee"; chemin: string; surveillance: Surveillance }
  | { issue: "refusee"; chemin: string; surveillance: Surveillance }
  | null;

/**
 * Le détour par les réglages du système, nommé pas à pas.
 *
 * POURQUOI il faut le donner : quand le sélecteur lui-même ne suffit plus -
 * l'autorisation est refusée au niveau du système, et non du dossier -, il n'y a
 * plus rien à faire depuis l'application. Sans ce chemin-là, l'utilisateur
 * recommencerait indéfiniment le même geste sans effet.
 */
const REGLAGES_DU_SYSTEME =
  "Ouvrez Réglages Système, puis Confidentialité et sécurité, et autorisez Vibe Map à accéder " +
  "aux fichiers et dossiers.";

export function suiteDeLaReautorisation(reponse: ReponseReautorisation): Suite {
  // Pas de pont, ou un sélecteur refermé sans choix : l'autorisation n'a pas
  // bougé, et un écran qui annoncerait quelque chose ferait passer un geste
  // repris pour un incident.
  if (reponse === null || reponse.issue === "annulee") {
    return { annonce: null, surveillance: null };
  }

  if (reponse.issue === "autre_dossier") {
    return {
      annonce: {
        ton: "echec",
        texte:
          `Le sélecteur a rendu « ${reponse.choisi} », et non « ${reponse.attendu} » : le système ` +
          `n'accorde l'accès qu'au dossier désigné. Redemandez, et désignez « ${reponse.attendu} ».`,
      },
      surveillance: null,
    };
  }

  if (reponse.issue === "inconnu") {
    return {
      annonce: {
        ton: "echec",
        texte:
          `« ${reponse.chemin} » n'est plus dans la liste des dossiers surveillés : il n'y a plus ` +
          "d'autorisation à lui redemander. La liste ci-dessous est celle du poste.",
      },
      surveillance: reponse.surveillance,
    };
  }

  if (reponse.issue === "refusee") {
    return {
      annonce: {
        ton: "echec",
        texte: `« ${reponse.chemin} » ne s'ouvre toujours pas. ${REGLAGES_DU_SYSTEME}`,
      },
      surveillance: reponse.surveillance,
    };
  }

  // FR-034 : le dossier repart en cartographie sans attendre le tour suivant,
  // comme après un ajout. Le dire évite de recommencer parce que les dépôts ne
  // sont pas déjà là.
  return {
    annonce: {
      ton: "succes",
      texte: `« ${reponse.chemin} » est de nouveau lisible. Ses dépôts apparaissent d'ici une minute.`,
    },
    surveillance: reponse.surveillance,
  };
}

/**
 * L'appel lui-même n'a pas abouti : le pont a levé une erreur au lieu de rendre
 * une issue.
 *
 * POURQUOI une phrase à part plutôt qu'une issue de plus : les cinq issues sont
 * ce que le POSTE répond, et le poste n'a rien répondu. Les confondre ferait
 * annoncer « le système refuse » là où c'est la commande qui n'est pas passée -
 * et enverrait l'utilisateur régler des autorisations qui n'y sont pour rien.
 */
export function echecDeLaRedemande(chemin: string, raison: string): Suite {
  return {
    annonce: {
      ton: "echec",
      texte: `L'autorisation de « ${chemin} » n'a pas pu être redemandée : ${raison}`,
    },
    surveillance: null,
  };
}

/* ---------- l'heure de la dernière cartographie (FR-030, FR-086) ---------- */

/**
 * L'heure de la dernière cartographie, telle que la base la porte.
 *
 * « jamais » quand aucun dépôt n'a d'heure : rien n'a jamais été cartographié
 * ici, et une heure vide se lirait comme une panne. Sinon `quand`, en
 * millisecondes, que l'écran met en forme dans le fuseau du lecteur, et `age`,
 * l'ancienneté en clair.
 */
export type Cartographie = { etat: "jamais" } | { etat: "connue"; quand: number; age: string };

/**
 * FR-086 : un agrégat, et l'écran l'établit comme tel - la PLUS RÉCENTE des
 * heures de cartographie des dépôts, jamais une heure par dépôt ni celle d'un
 * dépôt pris au hasard.
 *
 * Les heures illisibles sont ignorées plutôt que de contaminer le maximum :
 * une seule date invalide rendrait l'agrégat entier `NaN`, et l'écran
 * annoncerait « jamais cartographié » à un poste qui l'a été mille fois.
 */
export function derniereCartographie(
  heures: ReadonlyArray<string | null | undefined>,
  maintenant: number,
): Cartographie {
  const connues = instantsConnus(heures);
  if (connues.length === 0) return { etat: "jamais" };

  const quand = Math.max(...connues);
  return { etat: "connue", quand, age: dureeTexte(maintenant - quand) };
}

/**
 * Les heures qu'on sait lire, en millisecondes. Une seule règle de lecture pour
 * l'agrégat et pour le silence : deux règles finiraient par diverger, et l'écran
 * dirait « jamais cartographié » à côté d'une heure affichée.
 */
function instantsConnus(heures: ReadonlyArray<string | null | undefined>): number[] {
  return heures
    .filter((heure): heure is string => typeof heure === "string" && heure !== "")
    .map((heure) => Date.parse(heure))
    .filter((instant) => !Number.isNaN(instant));
}

/* ---------- les trois silences (FR-074, FR-087, issue #69) ---------- */

/**
 * Ce que l'écran sait de la dernière cartographie.
 *
 * Trois silences qui se ressemblent et ne disent pas la même chose - c'est tout
 * l'objet de cette tranche :
 *
 * - « injoignable » : la base n'a pas répondu (FR-085), donc on ne SAIT pas.
 *   Conclure « jamais » ferait annoncer une panne du poste pour une panne de
 *   réseau.
 * - « jamais » : aucune cartographie n'a jamais abouti ici.
 * - « sans_depot » : une cartographie a abouti et n'a trouvé aucun dépôt. La
 *   base n'a alors aucune heure à donner, et « jamais » serait un contresens
 *   (FR-087).
 * - « connue » : une heure existe, et c'est la plus récente qui s'affiche.
 */
export type Silence = "injoignable" | "jamais" | "sans_depot" | "connue";

export type FaitsCartographie = {
  /** Les heures de cartographie des dépôts, telles que la base les porte. */
  heures: ReadonlyArray<string | null | undefined>;
  /** La base a-t-elle refusé de répondre ? (FR-085) */
  baseInjoignable: boolean;
  /**
   * Ce que le poste dit qu'il y a à cartographier. `null` quand personne n'est
   * là pour le dire - un navigateur ordinaire, où FR-060 retire les faits du
   * poste : l'écran s'en tient alors à ce que la base porte.
   */
  aDecouvrir: ADecouvrir | null;
};

/**
 * Lequel des quatre. L'ordre est tout :
 *
 * 1. une base muette d'abord, parce qu'elle rend toutes les autres questions
 *    sans réponse ;
 * 2. une heure connue ensuite, parce qu'elle est un fait et qu'aucun fait local
 *    ne la contredit - une cartographie en cours se signale À CÔTÉ d'elle, elle
 *    ne la remplace pas (FR-074) ;
 * 3. le fait du poste enfin, qui est la seule chose capable de distinguer « rien
 *    trouvé » de « jamais rien cherché » ;
 * 4. et « jamais » en dernier, comme la conclusion qu'on ne tire qu'après avoir
 *    écarté les trois autres.
 */
export function silenceCartographie(faits: FaitsCartographie): Silence {
  if (faits.baseInjoignable) return "injoignable";
  if (instantsConnus(faits.heures).length > 0) return "connue";
  if (faits.aDecouvrir === "rien") return "sans_depot";
  return "jamais";
}

/** Ce que la ligne de la dernière cartographie affiche. */
export type EtatCartographie =
  | { etat: "injoignable" }
  | { etat: "jamais" }
  | { etat: "sans_depot" }
  | { etat: "connue"; quand: number; age: string };

/**
 * Le silence, et l'heure quand il y en a une. `maintenant` est reçu en
 * paramètre, jamais lu : un test qui dépendrait de l'horloge réelle deviendrait
 * rouge un jour sans que rien n'ait changé.
 */
export function etatCartographie(
  faits: FaitsCartographie,
  maintenant: number,
): EtatCartographie {
  const silence = silenceCartographie(faits);
  if (silence !== "connue") return { etat: silence };
  return derniereCartographie(faits.heures, maintenant);
}

/* ---------- le lecteur, par-dessus l'interface (FR-009, FR-010) ---------- */

/**
 * Ce que le pont rend de l'état du lecteur (`bureau/src/lecteur.rs`).
 *
 * Un fait du poste de plus, que la base ne porte pas : elle ne sait rien d'un
 * verrou pris sur cette machine. C'est pourquoi la carte, qui vient d'elle,
 * peut s'afficher entière pendant que plus rien ne remonte d'ici.
 */
export type CasEchecLecteur = "poste_tenu" | "jeton_refuse" | "arret_inattendu" | "panne";

export type EtatLecteur =
  | { etat: "en_demarrage" }
  | { etat: "en_marche" }
  | { etat: "arrete" }
  | { etat: "en_echec"; cas: CasEchecLecteur; raison: string };

/**
 * Le bandeau à passer par-dessus l'interface, ou rien.
 *
 * POURQUOI un bandeau qui ne parle que quand ça cloche : un lecteur arrêté ne
 * se voit nulle part ailleurs - la carte continue de s'afficher, alimentée par
 * ce que la machine avait déjà envoyé (FR-010). Un bandeau permanent qui
 * annoncerait « le lecteur tourne » par-dessus chaque écran serait, lui, du
 * bruit : la page d'attente de l'application dit déjà les quatre états au
 * lancement (FR-009), et un démarrage en cours n'appelle aucun geste.
 */
export type BandeauLecteur =
  | { visible: false }
  | { visible: true; titre: string; explication: string; relance: boolean };

const RIEN_NE_PART =
  "Cette machine n'envoie plus rien tant qu'il ne tourne pas : ce que vous voyez ici date " +
  "d'avant son arrêt.";

/**
 * `lecteur` vaut `null` hors de l'application, où il n'y a pas de pont à qui
 * demander : l'interface ne prétend alors rien savoir du poste, et ne montre
 * aucun bandeau (FR-060).
 */
export function bandeauDuLecteur(lecteur: EtatLecteur | null): BandeauLecteur {
  if (lecteur === null) return { visible: false };
  if (lecteur.etat === "en_marche" || lecteur.etat === "en_demarrage") {
    return { visible: false };
  }
  if (lecteur.etat === "arrete") {
    return {
      visible: true,
      titre: "Le lecteur est arrêté",
      explication: RIEN_NE_PART,
      relance: true,
    };
  }

  if (lecteur.cas === "poste_tenu") {
    return {
      visible: true,
      titre: "Un lecteur tourne déjà sur cette machine",
      explication:
        `${lecteur.raison} Vibe Map n'en démarre pas un second : la même activité partirait ` +
        "en double. Arrêtez l'autre, puis relancez.",
      relance: true,
    };
  }
  if (lecteur.cas === "arret_inattendu") {
    return {
      visible: true,
      titre: "Le lecteur s'est arrêté tout seul",
      explication: `Il tournait, et il ne tourne plus. ${RIEN_NE_PART}`,
      relance: true,
    };
  }
  if (lecteur.cas === "jeton_refuse") {
    return {
      visible: true,
      titre: "Le jeton de cette machine n'a pas pu être lu",
      explication: `${lecteur.raison} Un refus d'autorisation laisse le lecteur à l'arrêt.`,
      relance: true,
    };
  }
  return {
    visible: true,
    titre: "Le lecteur n'a pas démarré",
    explication: `${lecteur.raison} ${RIEN_NE_PART}`,
    relance: true,
  };
}

/* ---------- ce que le poste sait de lui-même (FR-050, FR-065, FR-068) ---------- */

/**
 * Où en est le dépouillement des trente derniers jours de journaux, tel que le
 * pont le rend (`bureau/src/contexte.rs`).
 *
 * Quatre états qui ne se confondent pas, et c'est tout l'objet de cette
 * tranche : « rien à dépouiller » n'est pas « 0 sur 0 », « jamais commencé »
 * n'est pas « terminé ». Chacun appelle une phrase différente parce qu'aucun ne
 * dit la même chose de ce qui va se passer ensuite.
 */
export type Depouillement =
  | { etat: "jamais" }
  | { etat: "rien_a_depouiller" }
  | { etat: "en_cours"; journaux: number; total: number }
  | { etat: "termine"; quand: string; journaux: number };

/** Ce que la commande de pont « lire le contexte » rend du poste. */
export type FaitsDuPoste = {
  /** Le numéro que l'application porte, fixé à sa compilation (FR-068). */
  version: string;
  /**
   * Le nom de cette machine, identique à celui de la liste des machines.
   * `null` quand le poste ne sait pas le dire : l'écran se tait alors plutôt
   * que d'afficher un nom inventé à la place de celui du Mac.
   */
  machine: string | null;
  depouillement: Depouillement;
};

/**
 * Ce que la fenêtre a obtenu en demandant. `null` dit qu'il n'y a pas de pont à
 * qui demander - le navigateur ordinaire de FR-060.
 */
export type ReponseContexte =
  | FaitsDuPoste
  | { etat: "sans_reponse"; raison: string }
  | null;

/**
 * Ce que la section montre, ou ce qui la remplace.
 *
 * - « hors application » : pas de pont. La section entière s'efface, et la
 *   mention de FR-060 - une seule, celle des dossiers - la couvre. Deux
 *   mentions côte à côte sur le même écran diraient deux fois la même chose.
 * - « sans réponse » : le pont est là et n'a rien rendu. Il ne doit pas se
 *   déguiser en navigateur ordinaire : ici, quelque chose cloche.
 * - « faits » : la version, le nom de la machine, le dépouillement.
 */
export type SectionPoste =
  | { quoi: "hors_application" }
  | { quoi: "sans_reponse"; raison: string }
  | { quoi: "faits"; faits: FaitsDuPoste };

export function sectionPoste(reponse: ReponseContexte): SectionPoste {
  if (reponse === null) return { quoi: "hors_application" };
  if ("etat" in reponse) return { quoi: "sans_reponse", raison: reponse.raison };
  return { quoi: "faits", faits: reponse };
}

/**
 * L'avancement du dépouillement, mis en mots (FR-050, FR-065).
 *
 * `quand` et `age` n'accompagnent que le terme : c'est l'écran qui met l'heure
 * en forme dans le fuseau du lecteur, comme il le fait de la dernière
 * cartographie. `maintenant` est reçu en paramètre, jamais lu : un test qui
 * dépendrait de l'horloge réelle deviendrait rouge un jour sans que rien n'ait
 * changé.
 */
export type Avancement =
  | { etat: "jamais"; texte: string }
  | { etat: "rien"; texte: string }
  | { etat: "en_cours"; texte: string; journaux: number; total: number }
  | { etat: "termine"; texte: string; quand: number | null; age: string | null };

/**
 * FR-065 : sur un Mac où Claude Code n'a jamais tourné, il n'y a rien à
 * dépouiller, et le dire tout de suite est ce qui évite d'attendre un
 * avancement qui ne bougera jamais.
 */
const RIEN_A_DEPOUILLER = "aucun journal à dépouiller sur cette machine";

/** Tant que le lecteur n'a pas ouvert son premier journal, il n'y a pas encore
 * d'avancement à montrer - et ce n'est pas un dépouillement qui n'a rien
 * trouvé. */
const PAS_ENCORE = "en attente du premier dépouillement";

export function avancementDepouillement(
  depouillement: Depouillement,
  maintenant: number,
): Avancement {
  if (depouillement.etat === "jamais") {
    return { etat: "jamais", texte: PAS_ENCORE };
  }

  if (depouillement.etat === "rien_a_depouiller") {
    return { etat: "rien", texte: RIEN_A_DEPOUILLER };
  }

  if (depouillement.etat === "en_cours") {
    const { journaux, total } = depouillement;
    // Un avancement sans dénominateur ne dit rien : « 0 sur 0 » se lirait comme
    // un dépouillement figé, alors qu'il n'y a rien à dépouiller (FR-065). Le
    // poste ne le rend pas sous cette forme ; l'écran ne doit pas pour autant
    // l'afficher si cela arrivait.
    if (total === 0) return { etat: "rien", texte: RIEN_A_DEPOUILLER };
    return {
      etat: "en_cours",
      texte: `${journauxDepouilles(journaux)} sur ${total}`,
      journaux,
      total,
    };
  }

  const fait = journauxDepouilles(depouillement.journaux);
  const quand = Date.parse(depouillement.quand);

  // Une heure qu'on ne sait pas lire n'efface pas le fait : le dépouillement
  // s'est bien terminé, et c'est ce qui compte le plus des deux.
  if (Number.isNaN(quand)) {
    return { etat: "termine", texte: `terminé, ${fait}`, quand: null, age: null };
  }

  const age = dureeTexte(maintenant - quand);
  return { etat: "termine", texte: `terminé il y a ${age}, ${fait}`, quand, age };
}

/**
 * Le compte, en clair. Comme celui des dépôts : une phrase, pas une case de
 * tableau - et le singulier tient, parce qu'un « 1 journaux dépouillés » se
 * remarque plus que le nombre lui-même.
 */
export function journauxDepouilles(journaux: number): string {
  return journaux === 1 ? "1 journal dépouillé" : `${journaux} journaux dépouillés`;
}
