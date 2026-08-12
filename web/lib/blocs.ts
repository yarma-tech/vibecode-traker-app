/**
 * Le tableau manuel (issue #29) : la reference lisible d'un bloc, ses trois
 * colonnes, la validation de la saisie et l'autocompletion de l'emplacement.
 *
 * Ces fonctions sont pures : la base decide de l'etat (statut, reference), ces
 * fonctions ne font que le lire et le presenter, jamais l'inventer.
 */

export type StatutBloc = "todo" | "doing" | "done";
export type TypeBloc = "feature" | "correction" | "technique" | "exploration";

/** Un bloc tel que le tableau le lit. La base porte encore d'autres colonnes
 *  `prd_*` (`prd_statut`, `prd_maj`, `prd_valide_le`) ; l'ecran n'en a besoin
 *  que de celles-ci - les autres ne servent qu'a la conversion cote daemon
 *  (#37), jamais a l'affichage. */
export type Bloc = {
  id: string;
  ref: number;
  type: TypeBloc;
  titre: string;
  statut: StatutBloc;
  version: number;
  chemin: string | null;
  position: number;
  created_at: string;
  /** Provenance PRD (#37, FR-039 a FR-042). `null` = saisi a la main - la
   *  seule provenance que #29 a jamais connue. Rattachee par cette cle,
   *  jamais par le titre (FR-040) : c'est pour ca qu'un titre de feature
   *  peut changer d'une lecture a l'autre sans jamais faire de doublon. */
  prd_cle: string | null;
  /** Lue dans le document, affichee telle quelle (FR-042) - AUCUN geste du
   *  tableau ne l'ecrit : ni un input dans une carte, ni un PATCH direct
   *  (la base la protege elle-meme, `prd_champs_proteges`, migration #37). */
  prd_priorite: string | null;
  prd_a_clarifier: boolean;
  /** FR-041 : la feature a disparu du document, mais son bloc est conserve -
   *  jamais supprime. Se distingue sur la carte, jamais retiree du tableau. */
  prd_absent: boolean;
  /** FR-039 : ce bloc d'exploration portait un travail humain (une issue, une
   *  fermeture) au moment ou son PRD est passe `validé` - il a donc ete
   *  conserve plutot que retire, marque ici pour que la carte le dise. */
  prd_converti: boolean;
};

/** La reference d'un bloc, telle qu'on la copie pour lancer un agent. */
export function reference(ref: number): string {
  return `VM-${ref}`;
}

/**
 * Le nom accessible du bouton de copie d'une reference (F11, FR-034). La
 * reference seule (« VM-7 ») n'apprend rien hors du contexte visuel de sa
 * carte a qui ne la voit pas ; ce libelle porte donc aussi le titre du
 * travail. Il commence par le texte visible du bouton (« VM-7 ») pour tenir
 * WCAG 2.5.3 (Label in Name) : un utilisateur de commande vocale qui dit
 * « clique sur VM-7 » doit atteindre ce bouton.
 */
export function libelleCopier(ref: number, titre: string): string {
  return `Copier ${reference(ref)}, référence de « ${titre} »`;
}

/**
 * L'annonce faite apres une tentative de copie (F11, FR-034) : succes ou
 * echec, mais jamais silencieuse - un bouton qui ne dit rien laisse croire
 * qu'il n'a rien fait, et l'utilisateur colle ensuite dans le vide. Sert a la
 * fois de texte affiche (une copie reussie doit se voir) et de contenu d'une
 * region `aria-live` pour un lecteur d'ecran.
 */
export function messageCopie(ref: number, titre: string, succes: boolean): string {
  const vmRef = reference(ref);
  return succes
    ? `Référence ${vmRef} de « ${titre} » copiée.`
    : `Copie automatique impossible : sélectionnez ${vmRef} et copiez-le avec votre clavier.`;
}

/** Le libelle textuel d'un type : la couleur ne le dit jamais seule (FR-033). */
export const LIBELLE_TYPE: Record<TypeBloc, string> = {
  feature: "Feature",
  correction: "Correction",
  technique: "Technique",
  exploration: "Exploration",
};

/** Les quatre types, dans l'ordre ou le formulaire de creation et le filtre
 *  les presentent tous les deux — une seule liste, jamais deux qui pourraient
 *  diverger. */
export const TYPES: TypeBloc[] = ["feature", "correction", "technique", "exploration"];

/**
 * Le libelle d'un type, avec une roue de secours : la contrainte `check` de
 * la base peut evoluer (un cinquieme type ajoute cote SQL) sans que ce front
 * ait ete redeploye en meme temps. `LIBELLE_TYPE[type]` rendrait alors
 * `undefined` - un badge vide, la seule chose que FR-033 interdit
 * explicitement. Retomber sur la valeur brute garde toujours un texte lisible,
 * jamais la couleur seule, meme pour un type que ce code n'a jamais rencontre.
 */
export function libelleType(type: string): string {
  return (LIBELLE_TYPE as Record<string, string>)[type] ?? type;
}

/**
 * L'origine PRD affichee sur une carte (#37) : `prd_cle` complete
 * (`<date>/<id>/Fn` pour une feature, `<date>/<id>` pour une exploration)
 * sans sa date en tete, qui n'apprend rien a la lecture d'une carte - la
 * meme date apparait deja, plus lisible, dans `prd_maj`/`prd_valide_le`
 * quand ils sont affiches. Rend la cle complete telle quelle si elle ne
 * porte pas de `/` (defensif : ne devrait jamais arriver, `prd_cle` a
 * toujours au moins un `/` par construction cote daemon).
 */
export function origineCourte(prdCle: string): string {
  const premiereBarre = prdCle.indexOf("/");
  return premiereBarre === -1 ? prdCle : prdCle.slice(premiereBarre + 1);
}

/** Un titre est requis : ni vide, ni fait uniquement d'espaces. */
export function titreValide(titre: string): boolean {
  return titre.trim().length > 0;
}

/** Un bloc est decoupe des qu'il n'a plus d'emplacement propre : la base l'a
 *  vide au profit de sa premiere issue (#30, FR-006, FR-007). Un bloc ne
 *  porte donc jamais les deux a la fois - c'est ce que cette fonction lit,
 *  jamais ce qu'elle decide. */
export function estDecoupe(bloc: Bloc): boolean {
  return bloc.chemin === null;
}

/** La seule sortie de « Termine » (F8, FR-025) : un bloc simple termine peut
 *  en repartir. Un bloc decoupe, lui, ne se sort jamais directement - son
 *  statut est derive de ses issues (etat_bloc(), #30) et le serveur refuse
 *  de toute facon une ecriture directe dessus (bloc_statut_protege()) ; le
 *  geste de sortie vit alors sur les issues elles-memes. */
export function peutSortirDeTermine(bloc: Bloc): boolean {
  return bloc.statut === "done" && !estDecoupe(bloc);
}

/** Les trois colonnes du tableau, toujours toutes les trois presentes. */
export type Colonnes = Record<StatutBloc, Bloc[]>;

/** L'ordre d'une colonne : `position` d'abord, la creation la plus ancienne
 *  ensuite pour departager — sans quoi deux blocs a `position` egale
 *  sauteraient d'une place a l'autre a chaque relecture. */
function parOrdre(a: Bloc, b: Bloc): number {
  if (a.position !== b.position) return a.position - b.position;
  return a.created_at.localeCompare(b.created_at);
}

/**
 * Le filtre du tableau par type (FR-031). Il ne porte que sur les blocs : une
 * issue n'a pas de colonne `type` (elle n'apparait jamais comme une carte
 * autonome, F6/FR-021), et un bloc decoupe emprunte son type a ce que le
 * tableau lit reellement dans la colonne, pas a ce que ses issues contiennent
 * a l'interieur. Filtrer une issue individuellement n'aurait donc pas de sens
 * : ce que le filtre cache ou montre, c'est toujours une carte entiere.
 *
 * `null` veut dire « tous les types » — pas de filtre, la valeur par defaut.
 */
export function filtrerParType(blocs: Bloc[], type: TypeBloc | null): Bloc[] {
  if (type === null) return blocs;
  return blocs.filter((bloc) => bloc.type === type);
}

/**
 * Le groupe de filtres (FR-031, FR-035) exprime UN seul choix a la fois - un
 * seul bouton peut etre actif. Cinq boutons pour un choix unique n'ont donc
 * besoin que d'un seul arret de Tab : le patron WAI-ARIA des barres d'outils
 * (tabindex glissant) fait porter `tabIndex=0` au seul bouton courant, les
 * autres `tabIndex=-1`, et ce sont les fleches qui font circuler ce curseur
 * a l'interieur du groupe (#35 - avant ce correctif, chaque bouton etait un
 * arret de Tab distinct, ce qui allongeait inutilement le trajet jusqu'a la
 * premiere reference d'une carte).
 */
const TOUCHES_FILTRE = ["ArrowRight", "ArrowLeft", "Home", "End"] as const;
export type ToucheFiltre = (typeof TOUCHES_FILTRE)[number];

/** Distingue les touches qui font circuler le groupe de toutes les autres -
 *  Tab et Entree/Espace gardent leur sens natif (quitter le groupe, activer
 *  le bouton courant) et ne doivent jamais passer par `indexSuivantFiltre`. */
export function estToucheFiltre(touche: string): touche is ToucheFiltre {
  return (TOUCHES_FILTRE as readonly string[]).includes(touche);
}

/**
 * Le prochain arret du tabindex glissant. Boucle aux deux bouts (ArrowRight
 * depuis le dernier revient au premier, et inversement) : dans un groupe
 * aussi court que ce filtre, s'arreter en butee obligerait a repartir en
 * sens inverse pour atteindre l'autre extremite, plus lent qu'un simple tour.
 */
export function indexSuivantFiltre(actif: number, total: number, touche: ToucheFiltre): number {
  switch (touche) {
    case "ArrowRight":
      return (actif + 1) % total;
    case "ArrowLeft":
      return (actif - 1 + total) % total;
    case "Home":
      return 0;
    case "End":
      return total - 1;
  }
}

/**
 * Range les blocs dans leurs trois colonnes. La base calcule le statut (pour
 * un bloc decoupe, elle le derivera de ses issues des #30) : cette fonction
 * ne fait que trier ce qu'elle recoit, elle ne le recalcule jamais.
 */
export function colonnes(blocs: Bloc[]): Colonnes {
  const rangees: Colonnes = { todo: [], doing: [], done: [] };
  for (const bloc of blocs) {
    rangees[bloc.statut].push(bloc);
  }
  rangees.todo.sort(parOrdre);
  rangees.doing.sort(parOrdre);
  rangees.done.sort(parOrdre);
  return rangees;
}

/**
 * Les dossiers connus qui repondent a la saisie, pour l'autocompletion de
 * l'emplacement (FR-002). Une sous-chaine, insensible a la casse — rien de
 * plus : cette liste ne fait que suggerer, la saisie libre d'un chemin de
 * fichier reste toujours acceptee ailleurs, meme absente d'ici.
 */
export function suggestionsEmplacement(chemins: string[], requete: string): string[] {
  const terme = requete.trim().toLowerCase();
  if (!terme) return chemins;
  return chemins.filter((chemin) => chemin.toLowerCase().includes(terme));
}
