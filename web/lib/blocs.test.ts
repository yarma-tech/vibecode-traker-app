import { describe, it, expect } from "vitest";
import {
  reference,
  LIBELLE_TYPE,
  TYPES,
  colonnes,
  titreValide,
  suggestionsEmplacement,
  estDecoupe,
  peutSortirDeTermine,
  filtrerParType,
  libelleType,
  libelleCopier,
  messageCopie,
  indexSuivantFiltre,
  estToucheFiltre,
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

describe("peutSortirDeTermine — la seule sortie de Termine (F8, FR-025)", () => {
  it("un bloc simple termine peut en sortir", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "done", chemin: "web/app" }))).toBe(true);
  });

  it("un bloc a faire ou en cours n'a rien a en sortir", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "todo", chemin: "web/app" }))).toBe(false);
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "doing", chemin: "web/app" }))).toBe(false);
  });

  it("un bloc decoupe ne se sort jamais lui-meme : son statut est derive de ses issues (#30), le geste vit sur elles", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "done", chemin: null }))).toBe(false);
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

// Filtrer le tableau par type (issue #34, FR-031). La fonction ne filtre que
// des blocs : une issue n'a pas de type propre (elle n'apparait jamais comme
// une carte autonome, FR-021 - le type est une propriete de ce qu'on lit dans
// une colonne, donc du bloc, jamais de ce qu'il contient).
describe("filtrerParType — le filtre du tableau par type (FR-031)", () => {
  const feature = bloc({ id: "f", ref: 1, statut: "todo", type: "feature" });
  const correction = bloc({ id: "c", ref: 2, statut: "doing", type: "correction" });
  const technique = bloc({ id: "t", ref: 3, statut: "done", type: "technique", chemin: null });
  const exploration = bloc({ id: "e", ref: 4, statut: "doing", type: "exploration" });
  const tous = [feature, correction, technique, exploration];

  it("sans filtre (null), rend tous les blocs inchanges", () => {
    expect(filtrerParType(tous, null)).toEqual(tous);
  });

  it("garde uniquement les blocs du type demande", () => {
    expect(filtrerParType(tous, "correction")).toEqual([correction]);
  });

  it("filtre a travers les trois statuts a la fois : le filtre porte sur le type, pas la colonne", () => {
    // technique est en "done", exploration en "doing" : le filtre ne les
    // regroupe pas par colonne, seulement par type.
    expect(filtrerParType(tous, "technique")).toEqual([technique]);
    expect(filtrerParType(tous, "exploration")).toEqual([exploration]);
  });

  it("un type sans aucun bloc correspondant rend un tableau vide, jamais tout le monde", () => {
    expect(filtrerParType([feature, correction], "technique")).toEqual([]);
  });
});

// Copier une reference pour lancer un agent (F11, #35). La reference seule
// (« VM-7 ») ne dit rien hors du contexte visuel de sa carte : un lecteur
// d'ecran qui parcourt les boutons de la page doit entendre a la fois la
// forme VM-n et le titre du travail qu'elle designe (WCAG 2.5.3 — le nom
// accessible doit contenir le texte visible du bouton, ici « VM-7 »).
describe("libelleCopier — le nom accessible du bouton de copie (FR-034)", () => {
  it("contient la reference visible et le titre du travail", () => {
    expect(libelleCopier(7, "Refaire la landing")).toBe(
      "Copier VM-7, référence de « Refaire la landing »",
    );
  });

  it("garde la forme VM-n meme pour une grande reference", () => {
    expect(libelleCopier(1234, "Titre")).toContain("VM-1234");
  });
});

// Une copie doit se voir (consigne #35) : un bouton muet laisse croire qu'il
// n'a rien fait et l'utilisateur colle dans le vide. Le message sert a la
// fois d'annonce pour un lecteur d'ecran (aria-live) et de texte affiche.
describe("messageCopie — l'annonce apres une tentative de copie (FR-034)", () => {
  it("annonce le succes avec la reference et le titre", () => {
    expect(messageCopie(7, "Refaire la landing", true)).toBe(
      "Référence VM-7 de « Refaire la landing » copiée.",
    );
  });

  it("annonce l'echec et explique le repli au clavier", () => {
    expect(messageCopie(7, "Refaire la landing", false)).toBe(
      "Copie automatique impossible : sélectionnez VM-7 et copiez-le avec votre clavier.",
    );
  });
});

describe("libelleType — un libelle textuel meme pour un type inconnu (FR-033)", () => {
  it("rend le libelle connu pour les quatre types du PRD", () => {
    for (const type of TYPES) {
      expect(libelleType(type)).toBe(LIBELLE_TYPE[type]);
    }
  });

  it("retombe sur la valeur brute si la base renvoie un type que ce front ne connait pas encore (contrainte check qui a evolue cote base sans que le web ait suivi)", () => {
    expect(libelleType("chore")).toBe("chore");
  });
});

// Le groupe de filtres (F10, FR-031) exprime UN seul choix a la fois : il se
// tient au clavier par un tabindex glissant (patron WAI-ARIA des barres
// d'outils), pas cinq arrets de Tab pour cinq boutons qui ne peuvent de toute
// facon jamais etre actifs ensemble. Ce que ces fonctions decident : quel
// bouton devient l'arret unique du groupe apres une fleche (#35 - le
// correctif demande par la relecture sur le compte de Tab avant la premiere
// reference).
describe("estToucheFiltre — reconnait les touches qui font circuler le groupe", () => {
  it("reconnait les quatre touches de navigation d'un groupe a tabindex glissant", () => {
    expect(estToucheFiltre("ArrowRight")).toBe(true);
    expect(estToucheFiltre("ArrowLeft")).toBe(true);
    expect(estToucheFiltre("Home")).toBe(true);
    expect(estToucheFiltre("End")).toBe(true);
  });

  it("ignore toute autre touche - Tab et Entree restent au comportement natif du bouton", () => {
    expect(estToucheFiltre("Tab")).toBe(false);
    expect(estToucheFiltre("Enter")).toBe(false);
    expect(estToucheFiltre(" ")).toBe(false);
  });
});

describe("indexSuivantFiltre — le tabindex glissant du groupe de filtres (FR-035)", () => {
  const TOTAL = 5; // Tous + les quatre types (TYPES)

  it("ArrowRight avance d'un cran", () => {
    expect(indexSuivantFiltre(0, TOTAL, "ArrowRight")).toBe(1);
    expect(indexSuivantFiltre(3, TOTAL, "ArrowRight")).toBe(4);
  });

  it("ArrowRight sur le dernier boucle vers le premier", () => {
    expect(indexSuivantFiltre(4, TOTAL, "ArrowRight")).toBe(0);
  });

  it("ArrowLeft recule d'un cran", () => {
    expect(indexSuivantFiltre(2, TOTAL, "ArrowLeft")).toBe(1);
  });

  it("ArrowLeft sur le premier boucle vers le dernier", () => {
    expect(indexSuivantFiltre(0, TOTAL, "ArrowLeft")).toBe(4);
  });

  it("Home revient toujours au premier", () => {
    expect(indexSuivantFiltre(3, TOTAL, "Home")).toBe(0);
    expect(indexSuivantFiltre(0, TOTAL, "Home")).toBe(0);
  });

  it("End va toujours au dernier", () => {
    expect(indexSuivantFiltre(0, TOTAL, "End")).toBe(4);
    expect(indexSuivantFiltre(4, TOTAL, "End")).toBe(4);
  });
});
