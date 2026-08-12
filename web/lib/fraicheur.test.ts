import { describe, it, expect } from "vitest";
import {
  SEUIL_FRAICHEUR_MS,
  ageSignal,
  estFrais,
  dureeDepuisSignal,
  etatTableau,
  clinDoeilVisible,
} from "./fraicheur";

// Un instant fixe pour n'éprouver que l'écart, jamais l'heure du jour - même
// convention que figement.test.ts.
const MAINTENANT = Date.parse("2026-08-12T12:00:00Z");

/** Une date ISO, `secondes` avant `MAINTENANT` (négatif = dans le futur). */
function ilYA(secondes: number): string {
  return new Date(MAINTENANT - secondes * 1000).toISOString();
}

describe("ageSignal — l'âge du plus récent des deux signaux", () => {
  it("retient le battement de la machine quand il est plus récent que le scan", () => {
    // Scan ancien (10 min), battement récent (30 s) : c'est le battement qui
    // fait foi, sans quoi un daemon qui répond très bien entre deux
    // cartographies (toutes les 5 minutes) passerait à tort pour muet.
    expect(ageSignal(ilYA(600), ilYA(30), MAINTENANT)).toBe(30_000);
  });

  it("retient le scan quand il est plus récent que le battement connu", () => {
    // Une cartographie vient de réussir (10 s), le dernier battement connu
    // reste en retard (10 min) : c'est le scan qui fait foi.
    expect(ageSignal(ilYA(10), ilYA(600), MAINTENANT)).toBe(10_000);
  });

  it("retombe sur le scan seul quand la machine n'a jamais battu", () => {
    expect(ageSignal(ilYA(45), null, MAINTENANT)).toBe(45_000);
  });

  it("un scan daté dans le futur (horloges désynchronisées) donne un âge négatif", () => {
    expect(ageSignal(ilYA(-30), null, MAINTENANT)).toBe(-30_000);
  });
});

describe("estFrais — la bascule frais/muet dépend de l'âge du dernier signal", () => {
  it("le seuil de fraîcheur est de deux minutes (spec, section 7)", () => {
    expect(SEUIL_FRAICHEUR_MS).toBe(120_000);
  });

  it("un signal de moins de deux minutes est frais", () => {
    expect(estFrais(ilYA(30), null, MAINTENANT)).toBe(true);
  });

  it("juste sous le seuil, encore frais", () => {
    expect(estFrais(ilYA(119), null, MAINTENANT)).toBe(true);
  });

  it("à deux minutes tapantes, ce n'est plus « moins de deux minutes »", () => {
    expect(estFrais(ilYA(120), null, MAINTENANT)).toBe(false);
  });

  it("au-delà du seuil, muet", () => {
    expect(estFrais(ilYA(121), null, MAINTENANT)).toBe(false);
  });

  it("un daemon arrêté depuis trois heures est muet, pas frais par défaut", () => {
    expect(estFrais(ilYA(3 * 3600), ilYA(3 * 3600), MAINTENANT)).toBe(false);
  });

  it("une machine jamais vue (last_seen_at nul) ne rend pas frais à elle seule le scan ancien", () => {
    expect(estFrais(ilYA(3 * 3600), null, MAINTENANT)).toBe(false);
  });

  it("un battement récent rend frais même si le dernier scan est ancien", () => {
    expect(estFrais(ilYA(3600), ilYA(10), MAINTENANT)).toBe(true);
  });

  it("un scan dans le futur (horloge désynchronisée) est frais, jamais une raison de dire muet", () => {
    expect(estFrais(ilYA(-30), null, MAINTENANT)).toBe(true);
  });
});

describe("dureeDepuisSignal — la durée à afficher après « Muet depuis »", () => {
  it("reprend le format court partagé avec figement.ts", () => {
    expect(dureeDepuisSignal(ilYA(3 * 3600), ilYA(3 * 3600), MAINTENANT)).toBe("3 h");
  });

  it("quelques minutes se disent en minutes", () => {
    expect(dureeDepuisSignal(ilYA(150), null, MAINTENANT)).toBe("3 min");
  });
});

describe("etatTableau — les trois états vides, jamais confondus", () => {
  it("aucune carte nulle part : un tableau neuf, quelle que soit la fraîcheur", () => {
    expect(etatTableau({ todo: 0, doing: 0, done: 0 }, true)).toBe("neuf");
    expect(etatTableau({ todo: 0, doing: 0, done: 0 }, false)).toBe("neuf");
  });

  it("plus rien à faire ni en cours, et un signal frais : un achèvement", () => {
    expect(etatTableau({ todo: 0, doing: 0, done: 5 }, true)).toBe("acheve");
  });

  it("plus rien à faire ni en cours, mais aucun signal récent : muet, jamais un achèvement", () => {
    expect(etatTableau({ todo: 0, doing: 0, done: 5 }, false)).toBe("muet");
  });

  it("du travail encore visible en attente : état normal, la fraîcheur ne change rien", () => {
    expect(etatTableau({ todo: 2, doing: 0, done: 5 }, true)).toBe("normal");
    expect(etatTableau({ todo: 2, doing: 0, done: 5 }, false)).toBe("normal");
  });

  it("du travail encore en cours seul : état normal", () => {
    expect(etatTableau({ todo: 0, doing: 1, done: 0 }, true)).toBe("normal");
  });
});

describe("clinDoeilVisible — le clin d'oeil de FR-051 n'accompagne « neuf » que si frais", () => {
  it("montré sur un tableau neuf avec un signal frais", () => {
    expect(clinDoeilVisible("neuf", true)).toBe(true);
  });

  it("absent d'un tableau neuf sans signal récent", () => {
    expect(clinDoeilVisible("neuf", false)).toBe(false);
  });

  it("absent des deux autres états vides, même frais", () => {
    expect(clinDoeilVisible("acheve", true)).toBe(false);
    expect(clinDoeilVisible("muet", false)).toBe(false);
  });

  it("absent de l'état normal", () => {
    expect(clinDoeilVisible("normal", true)).toBe(false);
  });
});
