import { describe, it, expect } from "vitest";
import {
  PREFIXE_CACHE,
  plaquesDefaut,
  plaquesSquelette,
  lirePlanCache,
  ecrirePlanCache,
  type Plaque,
} from "./squelette";

// Le squelette de chargement conserve les PROPORTIONS RÉELLES des parcelles :
// rien ne doit sauter quand les données arrivent (critère 2). Deux garanties se
// prouvent ici : le squelette par défaut est un vrai découpage proportionnel, et
// le cache local rend, à la visite suivante, exactement les plaques déjà vues.

/** Une mémoire de test conforme à l'interface `Storage`, adossée à une Map. */
function fausseMemoire(): Storage {
  const sac = new Map<string, string>();
  return {
    get length() {
      return sac.size;
    },
    clear: () => sac.clear(),
    getItem: (cle) => (sac.has(cle) ? (sac.get(cle) as string) : null),
    key: (i) => Array.from(sac.keys())[i] ?? null,
    removeItem: (cle) => sac.delete(cle),
    setItem: (cle, valeur) => {
      sac.set(cle, valeur);
    },
  };
}

describe("plaquesDefaut — un squelette plausible quand le cache est vide", () => {
  it("remplit tout le cadre, sans trou ni débordement", () => {
    const aire = plaquesDefaut().reduce((s, p) => s + p.largeur * p.hauteur, 0);
    expect(aire).toBeCloseTo(100 * 100, 5);
  });

  it("garde des proportions : la plaque la plus lourde occupe la plus grande surface", () => {
    const aires = plaquesDefaut().map((p) => p.largeur * p.hauteur);
    // Les poids par défaut décroissent : la première plaque est la plus grande.
    expect(Math.max(...aires)).toBeCloseTo(aires[0], 9);
  });

  it("chaque plaque tient dans le cadre 100 × 100", () => {
    for (const p of plaquesDefaut()) {
      expect(p.x).toBeGreaterThanOrEqual(-1e-9);
      expect(p.y).toBeGreaterThanOrEqual(-1e-9);
      expect(p.x + p.largeur).toBeLessThanOrEqual(100 + 1e-9);
      expect(p.y + p.hauteur).toBeLessThanOrEqual(100 + 1e-9);
    }
  });
});

describe("plaquesSquelette — le cache prime, le défaut assure", () => {
  it("sans cache, retombe sur le squelette par défaut", () => {
    expect(plaquesSquelette(null)).toEqual(plaquesDefaut());
  });

  it("cache vide, retombe sur le défaut plutôt qu'un plan vide", () => {
    expect(plaquesSquelette([])).toEqual(plaquesDefaut());
  });

  it("cache présent, rend exactement les plaques du cache — rien ne saute", () => {
    const cache: Plaque[] = [
      { x: 0, y: 0, largeur: 60, hauteur: 100 },
      { x: 60, y: 0, largeur: 40, hauteur: 100 },
    ];
    expect(plaquesSquelette(cache)).toEqual(cache);
  });
});

describe("cache local du plan — mémoire des proportions entre deux visites", () => {
  const plaques: Plaque[] = [
    { x: 0, y: 0, largeur: 55, hauteur: 70 },
    { x: 55, y: 0, largeur: 45, hauteur: 70 },
    { x: 0, y: 70, largeur: 100, hauteur: 30 },
  ];

  it("écrit puis relit les mêmes plaques", () => {
    const memoire = fausseMemoire();
    ecrirePlanCache("repo-1", plaques, memoire);
    expect(lirePlanCache("repo-1", memoire)).toEqual(plaques);
  });

  it("sans mémoire (rendu serveur), lit null sans lever", () => {
    expect(lirePlanCache("repo-1", null)).toBe(null);
    expect(() => ecrirePlanCache("repo-1", plaques, null)).not.toThrow();
  });

  it("une entrée corrompue est ignorée plutôt que fatale", () => {
    const memoire = fausseMemoire();
    memoire.setItem(`${PREFIXE_CACHE}repo-1`, "{pas du json");
    expect(lirePlanCache("repo-1", memoire)).toBe(null);
  });

  it("deux repos ne mélangent pas leurs proportions", () => {
    const memoire = fausseMemoire();
    ecrirePlanCache("repo-1", plaques, memoire);
    ecrirePlanCache("repo-2", [{ x: 0, y: 0, largeur: 100, hauteur: 100 }], memoire);
    expect(lirePlanCache("repo-1", memoire)).toEqual(plaques);
    expect(lirePlanCache("repo-2", memoire)).toEqual([
      { x: 0, y: 0, largeur: 100, hauteur: 100 },
    ]);
  });
});
