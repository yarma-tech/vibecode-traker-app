/**
 * Decouper un bloc en issues (issue #30) : l'avancement d'un bloc decoupe, le
 * tri de ses issues pour l'ouverture sans quitter le tableau (FR-020), et la
 * regle qui dit quand l'emplacement d'une nouvelle issue est requis (FR-006).
 *
 * Comme `blocs.ts`, ces fonctions sont pures : la base decide de l'etat
 * (statut, reference, descente du chemin), ces fonctions ne font que le lire
 * et le presenter.
 */

import type { Bloc, StatutBloc } from "./blocs";

/** Une issue telle que le tableau la lit, a l'interieur du bloc qu'elle
 *  decoupe. Elle n'apparait jamais seule dans une colonne (FR-021). */
export type Issue = {
  id: string;
  ref: number;
  bloc_id: string;
  titre: string;
  chemin: string;
  statut: StatutBloc;
  version: number;
  position: number;
  created_at: string;
};

/** L'avancement d'un bloc decoupe : combien de ses issues sont terminees sur
 *  le total (FR-019). Un bloc sans issue n'a rien a afficher : `{0, 0}`. */
export function avancement(issues: Issue[]): { fait: number; total: number } {
  return {
    fait: issues.filter((i) => i.statut === "done").length,
    total: issues.length,
  };
}

/** Le texte affiche sur la ligne d'un bloc decoupe : `12 / 17`. */
export function texteAvancement(issues: Issue[]): string {
  const { fait, total } = avancement(issues);
  return `${fait} / ${total}`;
}

/** L'ordre d'ouverture d'un bloc : `position` d'abord, la creation la plus
 *  ancienne ensuite - identique au tri des colonnes (`blocs.ts`). */
function parOrdre(a: Issue, b: Issue): number {
  if (a.position !== b.position) return a.position - b.position;
  return a.created_at.localeCompare(b.created_at);
}

/** Les issues d'un bloc donne, triees pour l'ouverture en place (FR-020). */
export function issuesDuBloc(issues: Issue[], blocId: string): Issue[] {
  return issues.filter((i) => i.bloc_id === blocId).sort(parOrdre);
}

/**
 * L'emplacement d'une nouvelle issue est requis, sauf dans un seul cas : la
 * toute premiere issue d'un bloc encore simple (qui porte donc un chemin)
 * hérite ce chemin par la base (FR-006) - le formulaire n'a rien a en
 * demander. Des la seconde, le bloc n'a plus de chemin a descendre.
 */
export function cheminRequisPourNouvelleIssue(bloc: Bloc, issuesExistantes: Issue[]): boolean {
  const premiere = issuesExistantes.length === 0;
  return !(premiere && bloc.chemin !== null);
}
