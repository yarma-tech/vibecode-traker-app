/**
 * F16 (issue #40) : signaler qu'un emplacement vise le vide, sans jamais le
 * corriger a la place de l'utilisateur (FR-059, FR-060).
 *
 * Une jointure a la lecture, jamais une colonne stockee (conception du
 * 2026-08-11, section 7 - "meme mecanisme que la fraicheur", CONTRIBUTING.md
 * "l'etat ne se stocke pas, il se calcule a la lecture") : `modules` reste la
 * seule source de verite sur ce qui existe encore dans le depot cartographie
 * (migration 20260803000002, "un dossier par ligne"), ce fichier ne fait que
 * la comparer au `chemin` deja charge sur chaque bloc et chaque issue -
 * aucune requete de plus, `cheminsConnus` est deja recupere pour
 * l'autocompletion (`page.tsx`, FR-002).
 */

/** Le dossier qui contient directement `chemin`, "" pour un chemin deja a la
 *  racine - identique en substance a `parent()` cote daemon (`plan.rs`). */
function dossierParent(chemin: string): string {
  const i = chemin.lastIndexOf("/");
  return i === -1 ? "" : chemin.slice(0, i);
}

/**
 * Un chemin "ressemble" a un fichier quand son dernier segment porte un
 * point qui n'est pas en tete - le meme critere que `est_binaire()` cote
 * daemon (`plan.rs`, `Path::extension()`) : ".github" reste un dossier
 * cache, "blocs.ts" un fichier.
 *
 * C'est le piege central de F16 : `modules` ne liste QUE des dossiers
 * (migration 20260803000002). `creer_bloc` accepte pourtant la saisie libre
 * d'un chemin de fichier (FR-002) - un bloc ancre a "web/lib/blocs.ts" ne
 * trouvera donc JAMAIS son chemin exact dans `modules`, meme vivant. Sans
 * cette distinction, tout bloc ancre a un fichier serait signale a tort des
 * la premiere lecture - pire que de ne rien signaler.
 */
function ressembleAUnFichier(chemin: string): boolean {
  const dernierSegment = chemin.slice(chemin.lastIndexOf("/") + 1);
  return dernierSegment.lastIndexOf(".") > 0;
}

/**
 * Le chemin existe-t-il encore, au vu du dernier jeu de `modules` connu ?
 *
 * Un chemin qui ressemble a un fichier n'est jamais lui-meme un module : on
 * verifie alors son dossier PARENT plutot que lui. Un dossier reellement
 * renomme dont le parent reste par ailleurs peuple echapperait ainsi au
 * signalement (un faux negatif assume) - mais confondre un fichier vivant
 * avec un dossier disparu ferait l'inverse, un faux positif systematique sur
 * la moitie des blocs qui ancrent un fichier plutot qu'un dossier. Entre les
 * deux, F16 choisit explicitement de se taire plutot que de mentir (issue
 * #40 : "signaler a tort serait pire que de ne rien signaler").
 *
 * Comparaison insensible a la casse, comme `suggestionsEmplacement`
 * (`blocs.ts`) : un systeme de fichiers insensible a la casse (HFS+/APFS,
 * par defaut sur macOS) peut faire cohabiter une saisie utilisateur et la
 * casse que git a retenue sans que le dossier ait reellement bouge.
 */
function cheminExiste(chemin: string, modulesConnus: string[]): boolean {
  if (chemin === "") return true;
  const cible = (ressembleAUnFichier(chemin) ? dossierParent(chemin) : chemin).toLowerCase();
  if (cible === "") return true;
  return modulesConnus.some((module) => module.toLowerCase() === cible);
}

/**
 * FR-059 : ce chemin est-il un emplacement disparu a signaler sur la carte ?
 *
 * - Un travail termine (`!vivant`) n'attend plus rien de son emplacement -
 *   FR-059 ne parle que du travail VIVANT, jamais de l'historique (le
 *   controle inverse explicitement demande par l'issue).
 * - `chemin === null` : un bloc decoupe (#30) n'a plus de chemin propre, ses
 *   issues portent chacune le leur (`bloc_coherent()`, migration
 *   20260812000003) - rien a comparer, jamais un signal ici. `chemin` est lu
 *   pour ce qu'il est, jamais comme un proxy d'autre chose (piege deja
 *   corrige en #37 pour `estDecoupe`, voir `blocs.ts`).
 * - `modulesConnus` vide : soit ce depot n'a jamais ete cartographie, soit sa
 *   derniere cartographie est en cours d'ecriture (le daemon efface
 *   `modules` avant de le repeupler en entier, deux appels HTTP sans
 *   transaction entre eux - `pousser_plan`, daemon/src/lib.rs). Dans les
 *   deux cas, l'absence de module ne prouve RIEN sur l'existence d'un
 *   dossier precis. Tout signaler ici confondrait "ca n'existe plus" avec
 *   "on ne sait pas encore" - la meme distinction que #39 a deja du trancher
 *   pour la fraicheur (voir `fraicheur.ts`).
 *
 * Aucune exception de `type` : une exploration (#38) ancre son chemin au
 * fichier de PRD qui l'a fait naitre, pas a un dossier de travail - mais un
 * document deplace ou supprime est exactement le cas ou l'utilisateur veut
 * le savoir, et F13 lui laisse deja de quoi agir (renommer, retyper,
 * supprimer). L'exclure aurait ete une exception non demandee par l'issue,
 * pas une simplification.
 */
export function emplacementDisparu(
  chemin: string | null,
  vivant: boolean,
  modulesConnus: string[],
): boolean {
  if (!vivant || chemin === null || modulesConnus.length === 0) return false;
  return !cheminExiste(chemin, modulesConnus);
}
