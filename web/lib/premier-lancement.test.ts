import { describe, it, expect } from "vitest";
import { DEPOT, ETAPES, PUBLICATIONS } from "./premier-lancement";

// L'écran de premier lancement ne propose plus de créer un code d'appairage et
// ne montre plus aucune commande à recopier (FR-023, FR-024) : il dirige vers
// l'installation puis l'ouverture de l'application de bureau (FR-025), et porte
// le lien vers la page des publications du dépôt (FR-058). Ce que dit l'écran
// vit dans `premier-lancement.ts` : c'est donc ici que ça s'éprouve, mot à mot.

/** Ce qui trahit une commande de terminal dans un texte destiné à l'écran. */
const MARQUES_DE_TERMINAL = [
  /\$\s/,
  /\bbrew\b/i,
  /\bvibemap\b/i,
  /\bnpm\b/i,
  /\bsudo\b/i,
  /\bterminal\b/i,
  /\bcurl\b/i,
];

/** Le mot « code » comme mot, jamais comme morceau de « codebase ». */
const UN_CODE = /\bcodes?\b/i;

describe("le lien de téléchargement — FR-058", () => {
  it("mène à la page des publications GitHub du dépôt", () => {
    expect(PUBLICATIONS).toBe("https://github.com/yarma-tech/vibecode-traker-app/releases");
  });

  it("nomme le dépôt de ce produit, et pas un autre", () => {
    expect(PUBLICATIONS).toContain(DEPOT);
  });

  it("est une adresse absolue et chiffrée : un clic suffit à y arriver", () => {
    expect(PUBLICATIONS.startsWith("https://github.com/")).toBe(true);
    expect(PUBLICATIONS.endsWith("/releases")).toBe(true);
  });

  it("ne vise pas une publication en particulier, qui n'existe pas encore", () => {
    expect(PUBLICATIONS).not.toContain("/latest");
    expect(PUBLICATIONS).not.toContain("/download");
  });
});

describe("les étapes — FR-023, FR-024, FR-025", () => {
  const textes = ETAPES.flatMap((etape) => [etape.geste, etape.suite]);

  it("il y en a trois, et aucune n'est vide", () => {
    expect(ETAPES).toHaveLength(3);
    for (const texte of textes) expect(texte.trim().length).toBeGreaterThan(0);
  });

  it("aucune ne montre de commande à recopier dans un terminal", () => {
    for (const texte of textes) {
      for (const marque of MARQUES_DE_TERMINAL) {
        expect(texte, `« ${texte} » renvoie encore vers un terminal`).not.toMatch(marque);
      }
    }
  });

  it("aucune ne demande de code d'appairage", () => {
    for (const texte of textes) {
      expect(texte, `« ${texte} » parle encore d'un code`).not.toMatch(UN_CODE);
    }
  });

  it("la première dirige vers le téléchargement de l'application", () => {
    expect(ETAPES[0].geste).toMatch(/télécharge/i);
    expect(`${ETAPES[0].geste} ${ETAPES[0].suite}`).toMatch(/application de bureau|publications/i);
  });

  it("la deuxième dirige vers son ouverture", () => {
    expect(ETAPES[1].geste).toMatch(/ouvre/i);
  });

  it("la troisième ne demande rien : la carte s'allume seule", () => {
    expect(`${ETAPES[2].geste} ${ETAPES[2].suite}`).toMatch(/toute seule|d'elle-même/i);
  });
});
