import { describe, it, expect } from "vitest";
import { demoDemande, montrerPremierLancement, repoSansActivite } from "./ecrans";

// Les trois écrans « avant que le produit soit plein » (issue #12) se
// choisissent sur deux signaux seulement : l'état des machines pour l'accueil,
// l'activité du repo pour le plan. Un paramètre de développement (`?demo=`)
// force chacun sans avoir à provoquer la vraie panne (critère 6).

describe("demoDemande — lit l'écran forcé du paramètre de développement", () => {
  it("sans paramètre, aucun écran forcé", () => {
    expect(demoDemande(undefined)).toBe(null);
  });

  it("reconnaît l'onboarding", () => {
    expect(demoDemande("onboarding")).toBe("onboarding");
  });

  it("reconnaît le chargement", () => {
    expect(demoDemande("chargement")).toBe("chargement");
  });

  it("reconnaît le repo sans activité", () => {
    expect(demoDemande("sans-activite")).toBe("sans-activite");
  });

  it("une valeur inconnue ne force rien", () => {
    expect(demoDemande("nimporte")).toBe(null);
  });
});

describe("montrerPremierLancement — l'accueil bascule en onboarding", () => {
  it("aucune machine appairée : on montre l'onboarding", () => {
    expect(montrerPremierLancement(0, null)).toBe(true);
  });

  it("au moins une machine : on montre l'accueil normal", () => {
    expect(montrerPremierLancement(2, null)).toBe(false);
  });

  it("le paramètre de dev force l'onboarding même avec des machines", () => {
    expect(montrerPremierLancement(2, "onboarding")).toBe(true);
  });

  it("un autre écran forcé ne déclenche pas l'onboarding", () => {
    expect(montrerPremierLancement(2, "chargement")).toBe(false);
  });
});

describe("repoSansActivite — le plan est plein mais sans couleur", () => {
  it("ni état ni événement : le repo est sans activité", () => {
    expect(repoSansActivite(0, 0, null)).toBe(true);
  });

  it("un module actif : le repo a de l'activité", () => {
    expect(repoSansActivite(1, 0, null)).toBe(false);
  });

  it("un événement au journal : le repo a de l'activité", () => {
    expect(repoSansActivite(0, 3, null)).toBe(false);
  });

  it("le paramètre de dev force l'état sans activité, données présentes ou non", () => {
    expect(repoSansActivite(5, 9, "sans-activite")).toBe(true);
  });

  it("un autre écran forcé ne déclenche pas l'état sans activité", () => {
    expect(repoSansActivite(5, 9, "chargement")).toBe(false);
  });
});
