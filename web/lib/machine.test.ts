import { describe, it, expect } from "vitest";

import {
  annonceDeLaMachine,
  laMachineEstPrete,
  lireEtatMachine,
  type EtatMachine,
} from "./machine";

// La machine se déclare elle-même (issue #65). Ce qui se joue en base est
// éprouvé dans `daemon/tests/declaration.rs`, et la décision du poste dans
// `bureau/tests/machine.rs`. Ce qui se vérifie ici, c'est ce que la fenêtre en
// montre - et ce qu'elle ne montre jamais.

const REPRISE: EtatMachine = {
  etat: "reprise",
  machine_id: "11111111-1111-1111-1111-111111111111",
  label: "MacBook de Yarma",
};

describe("ce que la fenêtre lit du poste", () => {
  it("hors de l'application, il n'y a rien à lire", () => {
    expect(lireEtatMachine(null)).toBe(null);
    expect(lireEtatMachine(undefined)).toBe(null);
  });

  it("une réponse qu'on ne sait pas lire devient un échec, jamais un silence", () => {
    // Le silence se lirait « tout va bien » sur un poste dont on ne sait rien.
    expect(lireEtatMachine("bonjour")).toMatchObject({ etat: "echec" });
    expect(lireEtatMachine({ etat: "on_ne_sait_pas" })).toMatchObject({ etat: "echec" });
    expect(lireEtatMachine({})).toMatchObject({ etat: "echec" });
  });

  it("les six cas du poste se lisent tels quels", () => {
    for (const etat of [
      "reprise",
      "declaree",
      "revoquee",
      "inconnue",
      "trousseau_refuse",
      "echec",
    ]) {
      expect(lireEtatMachine({ etat })).toMatchObject({ etat });
    }
  });
});

describe("le lecteur ne repart que sur une machine prête", () => {
  it("une machine reprise ou déclarée porte son jeton", () => {
    expect(laMachineEstPrete(REPRISE)).toBe(true);
    expect(laMachineEstPrete({ ...REPRISE, etat: "declaree" })).toBe(true);
  });

  it("aucun autre cas ne fait repartir le lecteur", () => {
    // Sans jeton au trousseau, le lecteur ne démarrerait pas : le relancer ne
    // ferait qu'ouvrir des boîtes de dialogue du système en boucle.
    expect(laMachineEstPrete(null)).toBe(false);
    expect(laMachineEstPrete({ ...REPRISE, etat: "revoquee" })).toBe(false);
    expect(laMachineEstPrete({ etat: "inconnue", machine_id: REPRISE.machine_id })).toBe(false);
    expect(
      laMachineEstPrete({
        etat: "trousseau_refuse",
        machine_id: REPRISE.machine_id,
        raison: "accès refusé",
      }),
    ).toBe(false);
    expect(laMachineEstPrete({ etat: "echec", raison: "la base ne répond pas" })).toBe(false);
  });
});

describe("ce que la fenêtre annonce (FR-021, FR-080)", () => {
  it("une machine reprise ou déclarée n'annonce rien", () => {
    // La déclaration se voit là où elle compte : dans la liste des machines. Un
    // bandeau permanent par-dessus chaque écran serait du bruit.
    expect(annonceDeLaMachine(REPRISE)).toEqual({ visible: false });
    expect(annonceDeLaMachine({ ...REPRISE, etat: "declaree" })).toEqual({ visible: false });
    expect(annonceDeLaMachine(null)).toEqual({ visible: false });
  });

  it("le refus du trousseau est annoncé comme attendu, avec de quoi réessayer", () => {
    const annonce = annonceDeLaMachine({
      etat: "trousseau_refuse",
      machine_id: REPRISE.machine_id,
      raison: "le trousseau du système est inaccessible.",
    });

    expect(annonce.visible).toBe(true);
    if (!annonce.visible) return;

    expect(annonce.reessayer).toBe(true);
    // FR-080 : le message dit que rien n'a été déclaré à la place. C'est la
    // moitié de l'exigence qu'un simple « accès refusé » laisserait de côté -
    // et celle qui rassure l'utilisateur devant sa liste de machines.
    expect(annonce.explication).toContain("Aucune machine n'a été déclarée à la place");
    expect(annonce.explication).toContain("réessayez");
    expect(annonce.explication).toContain("le trousseau du système est inaccessible.");
  });

  it("une machine révoquée s'annonce, et ne se fait pas remplacer", () => {
    const annonce = annonceDeLaMachine({ ...REPRISE, etat: "revoquee" });

    expect(annonce.visible).toBe(true);
    if (!annonce.visible) return;

    expect(annonce.titre).toContain("MacBook de Yarma");
    expect(annonce.explication).toContain("ne la remplace pas");
    expect(annonce.reessayer).toBe(true);
  });

  it("une identité qui ne répond plus se dit, sans redéclaration silencieuse", () => {
    const annonce = annonceDeLaMachine({ etat: "inconnue", machine_id: REPRISE.machine_id });

    expect(annonce.visible).toBe(true);
    if (!annonce.visible) return;

    expect(annonce.explication).toContain("Rien n'a été redéclaré en silence");
    expect(annonce.reessayer).toBe(true);
  });

  it("chaque annonce visible nomme ce qui cloche et dit quoi faire", () => {
    const cas: EtatMachine[] = [
      { etat: "revoquee", machine_id: REPRISE.machine_id, label: "MacBook de Yarma" },
      { etat: "inconnue", machine_id: REPRISE.machine_id },
      { etat: "trousseau_refuse", machine_id: REPRISE.machine_id, raison: "accès refusé." },
      { etat: "echec", raison: "la base ne répond pas." },
    ];

    for (const etat of cas) {
      const annonce = annonceDeLaMachine(etat);
      expect(annonce.visible, `${etat.etat} doit s'annoncer`).toBe(true);
      if (!annonce.visible) continue;

      expect(annonce.titre.length, `${etat.etat} : un titre vide ne dit rien`).toBeGreaterThan(0);
      expect(
        annonce.explication.length,
        `${etat.etat} : une explication vide ne dit rien`,
      ).toBeGreaterThan(0);
      expect(annonce.reessayer, `${etat.etat} : un cul-de-sac n'est pas une réponse`).toBe(true);
    }
  });
});
