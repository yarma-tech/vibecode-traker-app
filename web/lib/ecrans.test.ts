import { describe, it, expect } from "vitest";
import {
  baseInjoignable,
  demoDemande,
  etatDeRequete,
  montrerPremierLancement,
  premiereInjoignable,
  raisonInjoignable,
  repoSansActivite,
} from "./ecrans";

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

  it("reconnaît la base injoignable", () => {
    expect(demoDemande("injoignable")).toBe("injoignable");
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

// La base injoignable (FR-085, issue #59) est un état d'écran de plus, jamais
// un vide. Les erreurs ci-dessous sont celles que le client Supabase rend pour
// de vrai : PostgREST pose un code sur ce que la base refuse, et un code vide
// avec un message de fetch sur ce qui n'a jamais atteint la base ; l'auth pose
// un statut 0 dans le même cas.

/** Réseau coupé, appel parti de Node : la cause porte le code système. */
const RESEAU_COUPE = {
  message: "TypeError: fetch failed",
  details:
    "TypeError: fetch failed\n\nCaused by: Error: getaddrinfo ENOTFOUND base.supabase.co (ENOTFOUND)",
  hint: "",
  code: "",
};

/** Le même appel depuis un navigateur : ni code système, ni détail. */
const RESEAU_COUPE_NAVIGATEUR = { message: "TypeError: Failed to fetch", details: "", code: "" };

/** L'auth échoue à son tour : elle range le fetch perdu dans un statut 0. */
const AUTH_SANS_REPONSE = { name: "AuthRetryableFetchError", message: "Failed to fetch", status: 0 };

/** La base répond, et ce qu'elle répond est un refus de droits. */
const DROITS_REFUSES = {
  message: "permission denied for table repos",
  details: null,
  hint: null,
  code: "42501",
};

describe("baseInjoignable - distingue l'absence de réponse d'un refus", () => {
  it("sans erreur, la base est joignable", () => {
    expect(baseInjoignable(null)).toBe(false);
    expect(baseInjoignable(undefined)).toBe(false);
  });

  it("réseau coupé : la base est injoignable", () => {
    expect(baseInjoignable(RESEAU_COUPE)).toBe(true);
  });

  it("réseau coupé dans un navigateur : la base est injoignable", () => {
    expect(baseInjoignable(RESEAU_COUPE_NAVIGATEUR)).toBe(true);
  });

  it("l'auth restée sans réponse : la base est injoignable", () => {
    expect(baseInjoignable(AUTH_SANS_REPONSE)).toBe(true);
  });

  it("adresse muette : la base est injoignable", () => {
    expect(baseInjoignable({ message: "connect ECONNREFUSED", code: "ECONNREFUSED" })).toBe(true);
  });

  it("un refus de droits n'est pas une base injoignable : la base a répondu", () => {
    expect(baseInjoignable(DROITS_REFUSES)).toBe(false);
  });

  it("un jeton périmé n'est pas une base injoignable", () => {
    expect(baseInjoignable({ message: "JWT expired", code: "PGRST301" })).toBe(false);
  });

  it("une session refusée n'est pas une base injoignable : le statut le prouve", () => {
    expect(
      baseInjoignable({ name: "AuthApiError", message: "Invalid claim", status: 403 }),
    ).toBe(false);
  });
});

describe("raisonInjoignable - dit la raison en clair, jamais l'erreur brute", () => {
  it("nomme une adresse qui ne mène nulle part", () => {
    expect(raisonInjoignable(RESEAU_COUPE)).toBe("Son adresse ne mène nulle part.");
  });

  it("nomme une connexion refusée", () => {
    expect(raisonInjoignable({ code: "ECONNREFUSED", message: "connect ECONNREFUSED" })).toBe(
      "Elle refuse la connexion.",
    );
  });

  it("nomme une attente trop longue", () => {
    expect(raisonInjoignable({ name: "AbortError", message: "The operation was aborted" })).toBe(
      "Elle n'a pas répondu à temps.",
    );
  });

  it("sans marque connue, s'en tient au fait certain", () => {
    expect(raisonInjoignable(RESEAU_COUPE_NAVIGATEUR)).toBe("L'appel est resté sans réponse.");
  });

  it("ne rend jamais le message brut du client", () => {
    expect(raisonInjoignable(RESEAU_COUPE)).not.toContain("fetch");
  });
});

describe("etatDeRequete - injoignable, vide ou peuplé", () => {
  it("des données : l'écran est peuplé", () => {
    expect(etatDeRequete({ data: [{ id: "a" }], error: null })).toBe("peuple");
  });

  it("une base joignable et un compte sans aucun dépôt : c'est l'état vide", () => {
    expect(etatDeRequete({ data: [], error: null })).toBe("vide");
  });

  it("une base joignable qui ne rend rien du tout : c'est encore l'état vide", () => {
    expect(etatDeRequete({ data: null, error: null })).toBe("vide");
  });

  it("réseau coupé : injoignable, jamais vide", () => {
    expect(etatDeRequete({ data: null, error: RESEAU_COUPE })).toBe("injoignable");
  });

  it("l'absence de réponse l'emporte sur des données périmées", () => {
    expect(etatDeRequete({ data: [{ id: "a" }], error: RESEAU_COUPE })).toBe("injoignable");
  });

  it("un refus de droits ne bascule pas en injoignable", () => {
    expect(etatDeRequete({ data: null, error: DROITS_REFUSES })).toBe("vide");
  });
});

describe("premiereInjoignable - un écran lit plusieurs requêtes", () => {
  it("aucune erreur : rien à annoncer", () => {
    expect(premiereInjoignable([null, undefined, DROITS_REFUSES])).toBe(null);
  });

  it("rend l'erreur restée sans réponse, d'où qu'elle vienne", () => {
    expect(premiereInjoignable([null, DROITS_REFUSES, RESEAU_COUPE])).toBe(RESEAU_COUPE);
  });

  it("rend la première des deux : c'est elle qui portera la raison", () => {
    expect(premiereInjoignable([AUTH_SANS_REPONSE, RESEAU_COUPE])).toBe(AUTH_SANS_REPONSE);
  });
});
