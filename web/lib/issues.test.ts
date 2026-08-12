import { describe, it, expect } from "vitest";
import { avancement, issuesDuBloc, cheminRequisPourNouvelleIssue, type Issue } from "./issues";
import type { Bloc } from "./blocs";

// Decouper un bloc en issues (issue #30) : l'avancement « 12 / 17 » (F6), le
// tri des issues d'un bloc pour l'ouverture sans quitter le tableau (FR-020),
// et la regle qui dit quand l'emplacement d'une nouvelle issue est requis
// (FR-006) : jamais pour la premiere d'un bloc encore simple, toujours
// ensuite.

function issue(partiel: Partial<Issue> & Pick<Issue, "id" | "ref" | "statut">): Issue {
  return {
    bloc_id: "bloc-1",
    titre: "titre",
    chemin: "web/app",
    version: 1,
    position: 0,
    created_at: "2026-08-12T10:00:00Z",
    ...partiel,
  };
}

function bloc(partiel: Partial<Bloc> & Pick<Bloc, "id" | "ref" | "statut">): Bloc {
  return {
    titre: "titre",
    type: "feature",
    chemin: null,
    version: 1,
    position: 0,
    created_at: "2026-08-12T10:00:00Z",
    ...partiel,
  };
}

describe("avancement — le compte d'issues terminees sur le total (FR-019)", () => {
  it("compte les issues done sur le total", () => {
    const issues = [
      issue({ id: "a", ref: 1, statut: "done" }),
      issue({ id: "b", ref: 2, statut: "done" }),
      issue({ id: "c", ref: 3, statut: "doing" }),
    ];
    expect(avancement(issues)).toEqual({ fait: 2, total: 3 });
  });

  it("un bloc sans issue n'a pas d'avancement a afficher", () => {
    expect(avancement([])).toEqual({ fait: 0, total: 0 });
  });

  it("dix-sept issues restent un compte, jamais dix-sept cartes (F6)", () => {
    const issues = Array.from({ length: 17 }, (_, i) =>
      issue({ id: `i${i}`, ref: i, statut: i < 12 ? "done" : "todo" }),
    );
    expect(avancement(issues)).toEqual({ fait: 12, total: 17 });
  });
});

describe("issuesDuBloc — les issues d'un bloc, triees pour l'ouverture (FR-020)", () => {
  it("ne rend que les issues du bloc demande", () => {
    const issues = [
      issue({ id: "a", ref: 1, statut: "todo", bloc_id: "bloc-1" }),
      issue({ id: "b", ref: 2, statut: "todo", bloc_id: "bloc-2" }),
    ];
    expect(issuesDuBloc(issues, "bloc-1").map((i) => i.id)).toEqual(["a"]);
  });

  it("trie par position puis par creation, la plus ancienne d'abord", () => {
    const recente = issue({
      id: "recente",
      ref: 2,
      statut: "todo",
      position: 0,
      created_at: "2026-08-12T12:00:00Z",
    });
    const ancienne = issue({
      id: "ancienne",
      ref: 1,
      statut: "todo",
      position: 0,
      created_at: "2026-08-12T09:00:00Z",
    });
    expect(issuesDuBloc([recente, ancienne], "bloc-1").map((i) => i.id)).toEqual([
      "ancienne",
      "recente",
    ]);
  });
});

describe("cheminRequisPourNouvelleIssue — la descente ne se saisit pas (FR-006)", () => {
  it("n'est pas requis pour la premiere issue d'un bloc encore simple", () => {
    const b = bloc({ id: "bloc-1", ref: 1, statut: "todo", chemin: "web/app/hero" });
    expect(cheminRequisPourNouvelleIssue(b, [])).toBe(false);
  });

  it("est requis des la seconde issue, le bloc n'a plus de chemin a descendre", () => {
    const b = bloc({ id: "bloc-1", ref: 1, statut: "todo", chemin: null });
    const existantes = [issue({ id: "a", ref: 2, statut: "todo", bloc_id: "bloc-1" })];
    expect(cheminRequisPourNouvelleIssue(b, existantes)).toBe(true);
  });

  it("reste requis si le bloc n'a jamais eu de chemin", () => {
    const b = bloc({ id: "bloc-1", ref: 1, statut: "todo", chemin: null });
    expect(cheminRequisPourNouvelleIssue(b, [])).toBe(true);
  });
});
