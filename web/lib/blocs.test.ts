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
  origineCourte,
  estExploration,
  compteSansExploration,
  afficherAvancement,
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
    prd_cle: null,
    prd_priorite: null,
    prd_a_clarifier: false,
    prd_absent: false,
    prd_converti: false,
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

// Un bloc est decoupe des qu'il porte au moins une issue - jamais deduit du
// seul chemin (releve en relecture visuelle de #37). Avant #37, un chemin nul
// impliquait TOUJOURS des issues : seule `bloc_coherent()` (#30) le videait,
// et seulement en meme temps que la premiere issue apparaissait - les deux
// etaient donc indissociables. Une feature issue d'un PRD casse cette
// coincidence : elle nait sans emplacement propre (le document n'en donne
// aucun) ET sans la moindre issue, un troisieme etat que #29/#30 n'avaient
// jamais eu a distinguer d'un bloc decoupe.
describe("estDecoupe — un bloc decoupe des qu'il porte au moins une issue (#37)", () => {
  it("zero issue : simple, meme sans chemin propre", () => {
    expect(estDecoupe(0)).toBe(false);
  });

  it("au moins une issue : decoupe", () => {
    expect(estDecoupe(3)).toBe(true);
  });
});

describe("peutSortirDeTermine — la seule sortie de Termine (F8, FR-025)", () => {
  it("un bloc simple termine peut en sortir", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "done" }), 0)).toBe(true);
  });

  it("un bloc a faire ou en cours n'a rien a en sortir", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "todo" }), 0)).toBe(false);
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "doing" }), 0)).toBe(false);
  });

  it("un bloc decoupe (au moins une issue) ne se sort jamais lui-meme : son statut est derive de ses issues (#30), le geste vit sur elles", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "done" }), 2)).toBe(false);
  });

  it("une feature issue d'un PRD, jamais decoupee, terminee par une reference de commit, peut sortir de Termine comme n'importe quel bloc simple - son chemin nul ne signifie pas decoupee (#37)", () => {
    expect(peutSortirDeTermine(bloc({ id: "a", ref: 1, statut: "done", chemin: null }), 0)).toBe(true);
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

// L'origine PRD affichee sur la carte (issue #37, FR-039 a FR-042) : la cle
// complete (`<date>/<id>/Fn`) sert au rattachement cote base, mais la date en
// tete n'apprend rien a la lecture d'une carte - `prd_maj`/`prd_valide_le`,
// pas encore affiches ici, sont l'endroit ou une date aurait un sens.
describe("origineCourte — l'origine PRD affichee sur la carte, sans la date en tete", () => {
  it("retire la date d'une cle de feature (<date>/<id>/Fn)", () => {
    expect(origineCourte("2026-08-10/PRD-001/F3")).toBe("PRD-001/F3");
  });

  it("retire la date d'une cle de document (<date>/<id>, celle d'une exploration)", () => {
    expect(origineCourte("2026-08-10/PRD-001")).toBe("PRD-001");
  });
});

// Constater une exploration ecrite par un agent (issue #38, F13, FR-047 et
// FR-048). Deux origines possibles pour une exploration - un agent (#38) ou
// un PRD brouillon (#36) -, mais rien ne les distingue une fois posees : ni
// colonne dediee, ni protection particuliere. La regle porte sur le TYPE, le
// seul champ que le tableau lit pour ces deux besoins.
describe("estExploration — le type, seul signal que le tableau utilise (#38)", () => {
  it("vrai pour un bloc de type exploration, quelle que soit son origine (agent ou PRD)", () => {
    expect(estExploration(bloc({ id: "e", ref: 1, statut: "doing", type: "exploration" }))).toBe(true);
  });

  it("faux pour les trois autres types", () => {
    expect(estExploration(bloc({ id: "f", ref: 1, statut: "todo", type: "feature" }))).toBe(false);
    expect(estExploration(bloc({ id: "c", ref: 1, statut: "todo", type: "correction" }))).toBe(false);
    expect(estExploration(bloc({ id: "t", ref: 1, statut: "todo", type: "technique" }))).toBe(false);
  });
});

// FR-047 : une exploration reste visible dans sa colonne (elle DOIT
// apparaitre, F13) mais ne compte dans AUCUN total - le seul total affiche
// aujourd'hui est ce compteur, a cote du titre de chaque colonne.
describe("compteSansExploration — une exploration n'entre dans aucun total (FR-047)", () => {
  it("compte les blocs ordinaires normalement", () => {
    const feature = bloc({ id: "f", ref: 1, statut: "todo", type: "feature" });
    const correction = bloc({ id: "c", ref: 2, statut: "todo", type: "correction" });
    expect(compteSansExploration([feature, correction])).toBe(2);
  });

  it("exclut les explorations du compte, sans les retirer de la liste fournie", () => {
    const feature = bloc({ id: "f", ref: 1, statut: "todo", type: "feature" });
    const exploration = bloc({ id: "e", ref: 2, statut: "todo", type: "exploration" });
    expect(compteSansExploration([feature, exploration])).toBe(1);
  });

  it("une colonne faite uniquement d'explorations compte pour zero", () => {
    const e1 = bloc({ id: "e1", ref: 1, statut: "doing", type: "exploration" });
    const e2 = bloc({ id: "e2", ref: 2, statut: "doing", type: "exploration" });
    expect(compteSansExploration([e1, e2])).toBe(0);
  });

  it("une liste vide compte pour zero", () => {
    expect(compteSansExploration([])).toBe(0);
  });
});

// FR-047, second cas : un bloc d'exploration decoupe (une issue lui a ete
// ajoutee a la main, #37 - pour le marquer "non vierge" avant une conversion
// de PRD) ne doit jamais afficher un avancement "X / Y" - ce serait compter
// une exploration dans un reste a faire, exactement ce que FR-047 interdit.
describe("afficherAvancement — une exploration ne montre jamais de X / Y (FR-047)", () => {
  it("un bloc ordinaire decoupe affiche son avancement", () => {
    expect(afficherAvancement(bloc({ id: "f", ref: 1, statut: "doing", type: "feature" }), 3)).toBe(true);
  });

  it("un bloc ordinaire simple n'a rien a afficher", () => {
    expect(afficherAvancement(bloc({ id: "f", ref: 1, statut: "todo", type: "feature" }), 0)).toBe(false);
  });

  it("une exploration decoupee n'affiche jamais d'avancement, meme avec des issues", () => {
    expect(afficherAvancement(bloc({ id: "e", ref: 1, statut: "doing", type: "exploration" }), 2)).toBe(false);
  });
});
