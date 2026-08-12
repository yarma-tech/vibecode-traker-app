import { describe, it, expect } from "vitest";
import {
  reference,
  LIBELLE_TYPE,
  colonnes,
  titreValide,
  suggestionsEmplacement,
  estDecoupe,
  type Bloc,
} from "./blocs";

// Le tableau manuel (issue #29) : une reference lisible, trois colonnes
// derivees du statut, et l'autocompletion de l'emplacement sur les dossiers
// connus sans jamais refuser une saisie libre (FR-002).

function bloc(partiel: Partial<Bloc> & Pick<Bloc, "id" | "ref" | "statut">): Bloc {
  return {
    titre: "titre",
    type: "feature",
    chemin: "web/app",
    version: 1,
    position: 0,
    created_at: "2026-08-12T10:00:00Z",
    ...partiel,
  };
}

describe("reference — la forme VM-n de l'issue", () => {
  it("prefixe l'entier de VM-", () => {
    expect(reference(7)).toBe("VM-7");
  });

  it("ne reformate pas un grand nombre", () => {
    expect(reference(1234)).toBe("VM-1234");
  });
});

describe("LIBELLE_TYPE — un libelle textuel par type (FR-033)", () => {
  it("porte les quatre types du PRD", () => {
    expect(LIBELLE_TYPE.feature).toBe("Feature");
    expect(LIBELLE_TYPE.correction).toBe("Correction");
    expect(LIBELLE_TYPE.technique).toBe("Technique");
    expect(LIBELLE_TYPE.exploration).toBe("Exploration");
  });
});

describe("titreValide — un titre est requis", () => {
  it("refuse une chaine vide", () => {
    expect(titreValide("")).toBe(false);
  });

  it("refuse une chaine d'espaces", () => {
    expect(titreValide("   ")).toBe(false);
  });

  it("accepte un titre normal", () => {
    expect(titreValide("Refaire la landing")).toBe(true);
  });
});

describe("colonnes — trois colonnes derivees du statut", () => {
  it("range chaque bloc dans sa colonne", () => {
    const a = bloc({ id: "a", ref: 1, statut: "todo" });
    const b = bloc({ id: "b", ref: 2, statut: "doing" });
    const c = bloc({ id: "c", ref: 3, statut: "done" });

    const rangees = colonnes([a, b, c]);

    expect(rangees.todo).toEqual([a]);
    expect(rangees.doing).toEqual([b]);
    expect(rangees.done).toEqual([c]);
  });

  it("trie chaque colonne par position puis par creation, la plus ancienne d'abord", () => {
    const recent = bloc({ id: "recent", ref: 2, statut: "todo", position: 0, created_at: "2026-08-12T12:00:00Z" });
    const ancien = bloc({ id: "ancien", ref: 1, statut: "todo", position: 0, created_at: "2026-08-12T09:00:00Z" });

    const rangees = colonnes([recent, ancien]);

    expect(rangees.todo.map((b) => b.id)).toEqual(["ancien", "recent"]);
  });

  it("une colonne sans bloc reste un tableau vide, jamais absente", () => {
    const rangees = colonnes([]);
    expect(rangees.todo).toEqual([]);
    expect(rangees.doing).toEqual([]);
    expect(rangees.done).toEqual([]);
  });
});

describe("estDecoupe — un bloc decoupe n'a plus d'emplacement propre (#30)", () => {
  it("un bloc avec un chemin est simple", () => {
    expect(estDecoupe(bloc({ id: "a", ref: 1, statut: "todo", chemin: "web/app" }))).toBe(false);
  });

  it("un bloc sans chemin est decoupe", () => {
    expect(estDecoupe(bloc({ id: "a", ref: 1, statut: "todo", chemin: null }))).toBe(true);
  });
});

describe("suggestionsEmplacement — autocompletion sur les dossiers connus", () => {
  const chemins = ["web/app/checkout", "web/app/landing", "daemon/src", "daemon/src/plan.rs"];

  it("propose tout tant que rien n'est saisi", () => {
    expect(suggestionsEmplacement(chemins, "")).toEqual(chemins);
  });

  it("filtre par sous-chaine, insensible a la casse", () => {
    expect(suggestionsEmplacement(chemins, "CHECKOUT")).toEqual(["web/app/checkout"]);
  });

  it("une saisie qui ne correspond a aucun dossier connu ne rend rien, mais reste saisissable ailleurs", () => {
    expect(suggestionsEmplacement(chemins, "web/lib/tva.ts")).toEqual([]);
  });
});
