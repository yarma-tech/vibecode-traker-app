/**
 * Le tableau manuel (issue #29) : la reference lisible d'un bloc, ses trois
 * colonnes, la validation de la saisie et l'autocompletion de l'emplacement.
 *
 * Ces fonctions sont pures : la base decide de l'etat (statut, reference), ces
 * fonctions ne font que le lire et le presenter, jamais l'inventer.
 */

export type StatutBloc = "todo" | "doing" | "done";
export type TypeBloc = "feature" | "correction" | "technique" | "exploration";

/** Un bloc tel que le tableau le lit. La base porte davantage de colonnes
 *  (les `prd_*`, inertes tant que #36 ne les remplit pas) ; l'ecran n'en a
 *  besoin que de celles-ci. */
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
};

/** La reference d'un bloc, telle qu'on la copie pour lancer un agent. */
export function reference(ref: number): string {
  return `VM-${ref}`;
}

/** Le libelle textuel d'un type : la couleur ne le dit jamais seule (FR-033). */
export const LIBELLE_TYPE: Record<TypeBloc, string> = {
  feature: "Feature",
  correction: "Correction",
  technique: "Technique",
  exploration: "Exploration",
};

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
