import { describe, it, expect } from "vitest";
import {
  SEUIL_SILENCE_MS,
  estFige,
  heureFigement,
  depuisTexte,
} from "./figement";

// Un instant fixe pour n'éprouver que l'écart, jamais l'heure du jour.
const MAINTENANT = Date.parse("2026-08-09T12:00:00Z");

/** Un battement daté de `secondes` avant `MAINTENANT`. */
function ilYA(secondes: number): string {
  return new Date(MAINTENANT - secondes * 1000).toISOString();
}

describe("estFige — la bascule figé dépend de l'âge du dernier battement", () => {
  it("le seuil de silence est de 90 secondes (spec, section 8)", () => {
    expect(SEUIL_SILENCE_MS).toBe(90_000);
  });

  it("un battement récent n'est pas figé", () => {
    expect(estFige(ilYA(10), MAINTENANT)).toBe(false);
  });

  it("juste sous le seuil, la machine répond encore", () => {
    expect(estFige(ilYA(89), MAINTENANT)).toBe(false);
  });

  it("au-delà de 90 s sans battement, la machine est figée", () => {
    expect(estFige(ilYA(91), MAINTENANT)).toBe(true);
  });

  it("très en retard, la machine reste figée", () => {
    expect(estFige(ilYA(3600), MAINTENANT)).toBe(true);
  });

  it("jamais vue (aucun battement) vaut figée : injoignable, pas vivante", () => {
    expect(estFige(null, MAINTENANT)).toBe(true);
  });

  it("distingue figé d'inactif : un battement frais n'est jamais figé, même sans activité", () => {
    // Critère 7 : l'état gelé (machine muette) ne se confond pas avec un repo
    // simplement au repos (machine à jour). Seule la présence en décide.
    expect(estFige(ilYA(5), MAINTENANT)).toBe(false);
  });
});

describe("heureFigement — l'heure du dernier état connu", () => {
  it("rend l'heure HH:MM du dernier battement", () => {
    // 2026-08-09T12:00:00Z lu dans le fuseau local : on vérifie le format, pas
    // le fuseau (deux chiffres, deux chiffres, séparés d'un « : »).
    expect(heureFigement(ilYA(120))).toMatch(/^\d{2}:\d{2}$/);
  });

  it("sans battement, ne fabrique pas d'heure", () => {
    expect(heureFigement(null)).toBe("—");
  });
});

describe("depuisTexte — depuis combien de temps la machine se tait", () => {
  it("moins d'une minute se dit en secondes", () => {
    expect(depuisTexte(ilYA(30), MAINTENANT)).toBe("il y a 30 s");
  });

  it("quelques minutes se disent en minutes", () => {
    expect(depuisTexte(ilYA(120), MAINTENANT)).toBe("il y a 2 min");
  });

  it("plusieurs heures se disent en heures", () => {
    expect(depuisTexte(ilYA(7200), MAINTENANT)).toBe("il y a 2 h");
  });

  it("au-delà d'un jour, se dit en jours", () => {
    expect(depuisTexte(ilYA(172_800), MAINTENANT)).toBe("il y a 2 j");
  });

  it("sans battement, se dit « jamais vue »", () => {
    expect(depuisTexte(null, MAINTENANT)).toBe("jamais vue");
  });
});
