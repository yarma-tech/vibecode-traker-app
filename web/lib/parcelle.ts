/**
 * Ce qu'une parcelle du plan annonce, ce qu'elle fait quand on l'active, et où
 * son infobulle se pose (F8, issue #78 - FR-063 et FR-083).
 *
 * Ces règles vivent ici, hors de React, parce qu'elles se cassent en silence :
 * rien à l'écran ne signale qu'une parcelle est sortie du parcours de
 * tabulation, ni qu'une infobulle ne paraît plus qu'à la souris.
 *
 *   - Une parcelle sans sous-dossier ne descend nulle part, mais elle reste
 *     joignable au clavier : ses deux dates sont souvent la seule chose qu'elle
 *     ait à dire, et elle est justement trop petite pour les porter en clair.
 *     Le refus d'ouvrir se rend donc par `aria-disabled`, JAMAIS par l'attribut
 *     `disabled`, qui retire l'élément du parcours et rendrait ces dates
 *     définitivement muettes au clavier (FR-083). `accesClavier` tient les deux
 *     attributs ensemble, pour qu'on ne puisse pas rétablir l'un en oubliant
 *     l'autre.
 *   - L'infobulle ne peut plus être l'attribut `title` du navigateur : le focus
 *     clavier ne l'affiche pas, seul le survol la déclenche (FR-083). Rendue
 *     par l'écran, elle doit se poser quelque part sans déborder du plan :
 *     `ancrerInfobulle` tranche de quel côté de la parcelle elle s'ouvre.
 *
 * La mise en mots des deux dates ne se refait pas ici : elle appartient à
 * `touches.ts` (issue #77), et deux formulations finiraient par diverger.
 */

import { datesEnMots, type DatesZone } from "./touches";

/** Une zone du dépôt, telle que la parcelle la présente. */
export type Zone = {
  chemin: string;
  loc: number;
  fichiers: number;
};

/**
 * Le nom court affiché SUR la parcelle, quand elle est assez grande.
 *
 * Les parcelles en « /. » portent les fichiers posés directement dans un
 * dossier, à côté de ses sous-dossiers.
 */
export function nomParcelle(chemin: string): string {
  if (chemin === ".") return "fichiers à la racine";
  if (chemin.endsWith("/.")) return "fichiers";

  const dernier = chemin.split("/").pop();
  return dernier && dernier.length > 0 ? dernier : chemin;
}

/**
 * Le même chemin, dit à voix haute. Il entre dans le nom accessible, où le seul
 * dernier segment ne suffit pas : « fichiers », entendu sans la carte sous les
 * yeux, ne désigne rien. Ailleurs, le chemin se dit tel qu'il est - c'est celui
 * que l'utilisateur retrouvera dans son éditeur.
 *
 * Le nom court reste contenu dans ce qui est dit (WCAG 2.5.3) : qui commande
 * l'écran à la voix nomme ce qu'il voit.
 */
export function cheminParle(chemin: string): string {
  if (chemin === ".") return "fichiers à la racine";
  if (chemin.endsWith("/.")) return `fichiers de ${chemin.slice(0, -2)}`;
  return chemin;
}

/** Le poids d'une zone en clair. Au-delà du millier, l'unité près ne dit rien. */
export function poidsEnMots(loc: number): string {
  if (loc >= 1000) return `${(loc / 1000).toFixed(loc >= 10000 ? 0 : 1)}k lignes`;
  return loc === 1 ? "1 ligne" : `${loc} lignes`;
}

function fichiersEnMots(fichiers: number): string {
  return fichiers === 1 ? "1 fichier" : `${fichiers} fichiers`;
}

/** Ce qui entoure la zone au moment où on la décrit. */
export type Contexte = {
  /** La parcelle a-t-elle des sous-dossiers où descendre. */
  ouvrable: boolean;
  /** Le mot que porte l'état de la zone : « lu », « écrit », « inactif »… */
  dit: string;
  /** Les branches des worktrees ouverts, s'il y en a. */
  branches: string[];
  /** Les deux dates de dernière touche de la zone. */
  dates: DatesZone;
  /**
   * L'horloge du navigateur, `null` tant qu'elle n'a pas démarré : rendue sur
   * le serveur, elle ne correspondrait pas à celle du poste. Avant ce moment,
   * on ne peut juger l'âge de rien, et la parcelle se tait plutôt que d'annoncer
   * un « rien de récent » qui serait faux à la seconde suivante.
   */
  maintenant: number | null;
};

export type Annonce = {
  /** Le nom accessible du bouton : chemin, poids, état, worktree, dates. */
  nomAccessible: string;
  /** Ce que porte l'infobulle, au survol COMME au focus clavier (FR-083). */
  infobulle: string;
};

/**
 * Les deux textes d'une parcelle. Ils portent tous deux les deux dates : une
 * parcelle trop petite pour les afficher en clair ne doit pas les perdre pour
 * autant, et c'est vrai qu'on la survole ou qu'on la joigne au clavier (FR-063).
 */
export function annoncerParcelle(zone: Zone, contexte: Contexte): Annonce {
  const { ouvrable, dit, branches, dates, maintenant } = contexte;

  const enMots = maintenant !== null ? datesEnMots(dates, maintenant) : "";
  const surimpression = branches.length > 0 ? `, worktree ${branches.join(", ")}` : "";

  const nomAccessible =
    `${cheminParle(zone.chemin)}, ${poidsEnMots(zone.loc)}, ${dit}${surimpression}` +
    (enMots ? `, ${enMots}` : "") +
    (ouvrable ? ", ouvrir" : "");

  const infobulle =
    `${zone.chemin} · ${poidsEnMots(zone.loc)} · ${fichiersEnMots(zone.fichiers)}` +
    (enMots ? ` · ${enMots}` : "");

  return { nomAccessible, infobulle };
}

/**
 * Où mène l'activation d'une parcelle, `null` quand elle ne mène nulle part.
 *
 * Une parcelle sans sous-dossier reste joignable au clavier : elle reçoit donc
 * des activations - Entrée, Espace, un clic - qu'elle doit absorber sans que la
 * page change (FR-063). Le refus est ici, pas dans l'écouteur de l'écran.
 */
export function ouvertureVisee(chemin: string, ouvrable: boolean): string | null {
  return ouvrable ? chemin : null;
}

/** Les attributs qui décident si la parcelle se joint au clavier. */
export type AccesClavier = {
  /**
   * Toujours faux. `disabled` sortirait la parcelle du parcours de tabulation
   * et rendrait ses dates injoignables sans souris : c'est exactement le défaut
   * que FR-083 corrige. Le champ est rendu explicitement plutôt qu'omis, pour
   * qu'un retour en arrière se voie.
   */
  disabled: false;
  /** Vrai quand la parcelle ne descend nulle part : joignable, non activable. */
  "aria-disabled": boolean;
};

export function accesClavier(ouvrable: boolean): AccesClavier {
  return { disabled: false, "aria-disabled": !ouvrable };
}

/** Le cadre d'une parcelle sur le plan, en pourcentages du plan. */
export type Cadre = {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
};

/** De quel bord l'infobulle s'accroche. Deux clés au plus, jamais opposées. */
export type Ancrage = {
  left?: string;
  right?: string;
  top?: string;
  bottom?: string;
};

/**
 * Passé ce point, une parcelle penche vers la droite du plan : son infobulle
 * s'y aligne par la droite, sinon elle sortirait par le bord.
 */
const MILIEU = 50;

/**
 * Une infobulle posée sous une parcelle du bas tomberait hors du plan. Le seuil
 * porte sur le BAS de la parcelle : une grande parcelle qui commence haut mais
 * finit bas ne s'ouvre pas dessous.
 */
export const SEUIL_DESSOUS = 75;

/**
 * Et une infobulle posée au-dessus d'une parcelle qui part du haut sortirait du
 * plan par l'autre bout - elle irait recouvrir le fil d'Ariane et le bandeau qui
 * le surmontent, c'est-à-dire de la lecture, pas du vide.
 */
export const SEUIL_DESSUS = 25;

/** Le jeu entre la parcelle et son infobulle, qui dégage le liseré de focus. */
const JEU = "4px";

function pourcent(valeur: number): string {
  // Le découpage du treemap rend des flottants : deux décimales suffisent au
  // pixel près, et évitent un style qui change à chaque rendu.
  return `${Math.round(valeur * 100) / 100}%`;
}

/**
 * Où poser l'infobulle d'une parcelle, dans le repère du plan.
 *
 * L'infobulle ne peut pas vivre DANS la parcelle : celle-ci coupe ce qui
 * dépasse (`overflow: hidden`, imposé par la requête de conteneur qui règle les
 * dates), et la bulle d'une petite parcelle serait rognée à rien. Elle vit donc
 * à côté, dans le plan, et s'accroche au bord de la parcelle qui laisse de la
 * place : jamais celui par lequel elle sortirait du plan.
 *
 * Trois cas, dans cet ordre, et le dernier est celui qui compte : une parcelle
 * qui occupe toute la hauteur du plan n'a de place ni dessous ni dessus, et une
 * bulle sortie du plan irait recouvrir le fil d'Ariane. Elle se pose alors DANS
 * la parcelle, contre son bord haut - le texte de la parcelle est calé en bas,
 * donc rien n'est masqué.
 */
export function ancrerInfobulle(cadre: Cadre): Ancrage {
  const { x, y, largeur, hauteur } = cadre;

  const horizontal: Ancrage =
    x + largeur / 2 <= MILIEU
      ? { left: pourcent(x) }
      : { right: pourcent(100 - x - largeur) };

  let vertical: Ancrage;
  if (y + hauteur <= SEUIL_DESSOUS) {
    vertical = { top: `calc(${pourcent(y + hauteur)} + ${JEU})` };
  } else if (y >= SEUIL_DESSUS) {
    vertical = { bottom: `calc(${pourcent(100 - y)} + ${JEU})` };
  } else {
    vertical = { top: `calc(${pourcent(y)} + ${JEU})` };
  }

  return { ...horizontal, ...vertical };
}
