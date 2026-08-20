import { describe, it, expect } from "vitest";
import { AUCUNE_TOUCHE, indexerTouches, touchesDeLaZone, type Touche } from "./touches";

/**
 * Des dates fixes, jamais l'horloge : ces fonctions ne comparent que des
 * instants entre eux, et un test qui dépendrait de l'heure réelle deviendrait
 * rouge un jour sans que rien n'ait changé (même convention que
 * `figement.test.ts` et `fraicheur.test.ts`).
 */
const IL_Y_A_CINQ_JOURS = "2026-08-15T10:00:00.000Z";
const IL_Y_A_TROIS_JOURS = "2026-08-17T10:00:00.000Z";
const IL_Y_A_DEUX_JOURS = "2026-08-18T10:00:00.000Z";
const HIER = "2026-08-19T10:00:00.000Z";

function ligne(
  module_path: string,
  derniere_ecriture: string | null,
  derniere_lecture: string | null,
): Touche {
  return { module_path, derniere_ecriture, derniere_lecture };
}

describe("touchesDeLaZone — ce que la base a déjà décidé, rendu à l'écran", () => {
  it("rend les deux dates d'une zone, chacune de son côté", () => {
    const index = indexerTouches([ligne("web/lib", HIER, IL_Y_A_TROIS_JOURS)]);

    expect(touchesDeLaZone(index, "web/lib")).toEqual({
      ecriture: HIER,
      lecture: IL_Y_A_TROIS_JOURS,
    });
  });

  it("rend l'héritage que la base a calculé pour l'ancêtre, tel quel", () => {
    // C'est `touches_modules` qui a posé sur « web » la date de « web/lib » :
    // l'écran la relaie sans la retoucher.
    const index = indexerTouches([
      ligne("web", HIER, IL_Y_A_TROIS_JOURS),
      ligne("web/lib", HIER, IL_Y_A_TROIS_JOURS),
    ]);

    expect(touchesDeLaZone(index, "web")).toEqual(
      touchesDeLaZone(index, "web/lib"),
    );
  });

  it("n'invente aucun héritage que la base n'aurait pas rendu", () => {
    // Le test qui verrouille FR-043 côté écran : l'héritage se calcule à la
    // lecture EN BASE, par préfixe de chemin, jamais deux fois. Si ce fichier
    // remontait « web/lib » vers « web » de son côté, il existerait deux règles
    // d'héritage qui finiraient par diverger - exactement la dérive que FR-043
    // interdit. Une zone absente de la réponse n'a rien reçu, point.
    const index = indexerTouches([ligne("web/lib", HIER, null)]);

    expect(touchesDeLaZone(index, "web")).toEqual(AUCUNE_TOUCHE);
  });

  it("une zone jamais touchée n'a pas de date, pas une date vide", () => {
    const index = indexerTouches([]);

    expect(touchesDeLaZone(index, "docs")).toEqual(AUCUNE_TOUCHE);
    expect(touchesDeLaZone(index, "docs").ecriture).toBeNull();
    expect(touchesDeLaZone(index, "docs").lecture).toBeNull();
  });

  it("une zone lue mais jamais écrite n'a que sa date de lecture", () => {
    // FR-039 et FR-040 : les deux dates vivent séparément en base. L'écran ne
    // comble pas l'absente avec l'autre, ce qui ferait dire « modifié » d'une
    // zone que personne n'a jamais modifiée.
    const index = indexerTouches([ligne("docs", null, HIER)]);

    expect(touchesDeLaZone(index, "docs")).toEqual({
      ecriture: null,
      lecture: HIER,
    });
  });

  it("un chemin en « . » se lit comme les autres, sans traitement à part", () => {
    // Les parcelles en « . » n'héritent pas de leurs sous-dossiers, mais c'est
    // la base qui le tranche : ici, un chemin est un chemin.
    const index = indexerTouches([
      ligne("src/.", IL_Y_A_CINQ_JOURS, null),
      ligne("src", HIER, null),
    ]);

    expect(touchesDeLaZone(index, "src/.").ecriture).toBe(IL_Y_A_CINQ_JOURS);
    expect(touchesDeLaZone(index, "src").ecriture).toBe(HIER);
  });
});

describe("indexerTouches — deux lignes du même chemin ne se perdent pas", () => {
  it("garde la plus récente de chaque date, jamais la dernière arrivée", () => {
    const index = indexerTouches([
      ligne("src", IL_Y_A_DEUX_JOURS, HIER),
      ligne("src", HIER, IL_Y_A_CINQ_JOURS),
    ]);

    expect(touchesDeLaZone(index, "src")).toEqual({
      ecriture: HIER,
      lecture: HIER,
    });
  });

  it("un nul ne chasse pas une date en place, et ne compte pas pour le passé", () => {
    const index = indexerTouches([
      ligne("src", IL_Y_A_TROIS_JOURS, null),
      ligne("src", null, HIER),
    ]);

    expect(touchesDeLaZone(index, "src")).toEqual({
      ecriture: IL_Y_A_TROIS_JOURS,
      lecture: HIER,
    });
  });

  it("ne rend que les zones que la base a rendues", () => {
    const index = indexerTouches([
      ligne("web", HIER, null),
      ligne("web/lib", HIER, null),
    ]);

    expect([...index.keys()].sort()).toEqual(["web", "web/lib"]);
  });
});
