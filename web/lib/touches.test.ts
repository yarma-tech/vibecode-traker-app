import { describe, it, expect } from "vitest";
import {
  AUCUNE_TOUCHE,
  SEUIL_RIEN_DE_RECENT_MS,
  datesEnMots,
  datesEnMotsSeparees,
  estRecente,
  indexerTouches,
  touchesDeLaZone,
  type Touche,
} from "./touches";

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

/**
 * La mise en mots (FR-041) et la règle des trente jours (FR-042). Un instant
 * fixe, jamais l'horloge : ces fonctions ne jugent qu'un écart.
 */
const MAINTENANT = Date.parse("2026-08-20T12:00:00.000Z");

const JOUR_MS = 24 * 60 * 60 * 1000;

/** Une date posée `ms` avant `MAINTENANT`. */
function ilYA(ms: number): string {
  return new Date(MAINTENANT - ms).toISOString();
}

const IL_Y_A_20_MIN = ilYA(20 * 60 * 1000);
const IL_Y_A_2_JOURS = ilYA(2 * JOUR_MS);
const IL_Y_A_29_JOURS = ilYA(29 * JOUR_MS);
const IL_Y_A_30_JOURS = ilYA(30 * JOUR_MS);
const IL_Y_A_31_JOURS = ilYA(31 * JOUR_MS);
const IL_Y_A_40_JOURS = ilYA(40 * JOUR_MS);

describe("estRecente - la règle des trente jours, date par date (FR-042)", () => {
  it("le seuil vaut trente jours", () => {
    expect(SEUIL_RIEN_DE_RECENT_MS).toBe(30 * JOUR_MS);
  });

  it("vingt-neuf jours comptent encore", () => {
    expect(estRecente(IL_Y_A_29_JOURS, MAINTENANT)).toBe(true);
  });

  it("trente jours pile comptent encore : le seuil est « PLUS de 30 jours »", () => {
    expect(estRecente(IL_Y_A_30_JOURS, MAINTENANT)).toBe(true);
  });

  it("trente et un jours ne comptent plus", () => {
    expect(estRecente(IL_Y_A_31_JOURS, MAINTENANT)).toBe(false);
  });

  it("une date inconnue ne compte pas : « jamais » n'est pas « récent »", () => {
    expect(estRecente(null, MAINTENANT)).toBe(false);
  });

  it("une date du futur reste la plus récente possible", () => {
    // Deux horloges désynchronisées ne doivent pas faire dire « rien de récent »
    // d'une zone qu'un agent vient de toucher.
    expect(estRecente(ilYA(-60_000), MAINTENANT)).toBe(true);
  });
});

describe("datesEnMots - les deux dates en clair (FR-041)", () => {
  it("les deux récentes se disent l'une après l'autre, écriture d'abord", () => {
    expect(
      datesEnMots({ ecriture: IL_Y_A_2_JOURS, lecture: IL_Y_A_20_MIN }, MAINTENANT),
    ).toBe("modifié il y a 2 j, relu il y a 20 min");
  });

  it("écrite il y a quarante jours, relue il y a vingt minutes : seule l'écriture se tait", () => {
    // Le cas qui prouve que les deux dates sont jugées SÉPARÉMENT. Si elles
    // étaient jugées ensemble, la lecture d'il y a vingt minutes disparaîtrait.
    expect(
      datesEnMots({ ecriture: IL_Y_A_40_JOURS, lecture: IL_Y_A_20_MIN }, MAINTENANT),
    ).toBe("rien de récent en écriture, relu il y a 20 min");
  });

  it("relue il y a quarante jours, écrite il y a deux jours : seule la lecture se tait", () => {
    expect(
      datesEnMots({ ecriture: IL_Y_A_2_JOURS, lecture: IL_Y_A_40_JOURS }, MAINTENANT),
    ).toBe("modifié il y a 2 j, rien de récent en lecture");
  });

  it("les deux au-delà du seuil : la zone entière se tait, sans donner de date", () => {
    const dit = datesEnMots(
      { ecriture: IL_Y_A_40_JOURS, lecture: IL_Y_A_31_JOURS },
      MAINTENANT,
    );

    expect(dit).toBe("rien de récent");
    expect(dit).not.toMatch(/il y a/);
  });

  it("une zone jamais touchée se tait de la même façon", () => {
    expect(datesEnMots(AUCUNE_TOUCHE, MAINTENANT)).toBe("rien de récent");
  });

  it("une seule date connue : l'autre se tait pour elle seule", () => {
    // FR-039 et FR-040 : une zone lue sans jamais être écrite existe. On ne
    // comble pas l'absente avec l'autre, ce qui ferait dire « modifié » d'une
    // zone que personne n'a modifiée.
    expect(datesEnMots({ ecriture: null, lecture: IL_Y_A_20_MIN }, MAINTENANT)).toBe(
      "rien de récent en écriture, relu il y a 20 min",
    );
    expect(datesEnMots({ ecriture: IL_Y_A_2_JOURS, lecture: null }, MAINTENANT)).toBe(
      "modifié il y a 2 j, rien de récent en lecture",
    );
  });

  it("à la frontière, trente jours se disent encore et trente et un se taisent", () => {
    expect(
      datesEnMots({ ecriture: IL_Y_A_30_JOURS, lecture: null }, MAINTENANT),
    ).toBe("modifié il y a 30 j, rien de récent en lecture");

    expect(
      datesEnMots({ ecriture: IL_Y_A_31_JOURS, lecture: null }, MAINTENANT),
    ).toBe("rien de récent");
  });

  it("séparées, chaque date est un membre à part, dans l'ordre", () => {
    // La parcelle passe à la ligne ENTRE les deux dates, jamais au milieu de
    // l'une d'elles : c'est pour cela que la forme séparée existe.
    expect(
      datesEnMotsSeparees(
        { ecriture: IL_Y_A_2_JOURS, lecture: IL_Y_A_20_MIN },
        MAINTENANT,
      ),
    ).toEqual(["modifié il y a 2 j", "relu il y a 20 min"]);
  });

  it("séparées, deux dates muettes ne font qu'un seul membre", () => {
    expect(datesEnMotsSeparees(AUCUNE_TOUCHE, MAINTENANT)).toEqual(["rien de récent"]);
  });

  it("réutilise l'arrondi de `dureeTexte`, sans en refaire un second", () => {
    // Une seule formule d'ancienneté dans le produit : secondes, minutes,
    // heures, jours. Deux arrondis finiraient par se contredire d'un écran à
    // l'autre.
    expect(datesEnMots({ ecriture: ilYA(30_000), lecture: null }, MAINTENANT)).toContain(
      "modifié il y a 30 s",
    );
    expect(
      datesEnMots({ ecriture: ilYA(2 * 60 * 60 * 1000), lecture: null }, MAINTENANT),
    ).toContain("modifié il y a 2 h");
  });
});
