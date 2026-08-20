import { describe, it, expect } from "vitest";
import {
  SEUIL_DESSOUS,
  accesClavier,
  ancrerInfobulle,
  annoncerParcelle,
  cheminParle,
  nomParcelle,
  ouvertureVisee,
  poidsEnMots,
} from "./parcelle";
import { AUCUNE_TOUCHE } from "./touches";

/**
 * Un instant fixe, jamais l'horloge : ces fonctions ne jugent qu'un écart, et
 * un test qui lirait l'heure réelle deviendrait rouge un jour sans que rien
 * n'ait changé (même convention que `touches.test.ts` et `figement.test.ts`).
 */
const MAINTENANT = Date.parse("2026-08-20T12:00:00.000Z");
const JOUR_MS = 24 * 60 * 60 * 1000;

function ilYA(ms: number): string {
  return new Date(MAINTENANT - ms).toISOString();
}

const IL_Y_A_20_MIN = ilYA(20 * 60 * 1000);
const IL_Y_A_2_JOURS = ilYA(2 * JOUR_MS);

const DEUX_DATES = { ecriture: IL_Y_A_2_JOURS, lecture: IL_Y_A_20_MIN };

const ZONE = { chemin: "web/lib", loc: 1240, fichiers: 12 };

const AU_REPOS = {
  ouvrable: false,
  dit: "inactif",
  branches: [] as string[],
  dates: DEUX_DATES,
  maintenant: MAINTENANT,
};

describe("accesClavier - une parcelle muette reste joignable (FR-083)", () => {
  it("ne désactive JAMAIS une parcelle, même sans sous-dossier", () => {
    // Le défaut que cette tranche corrige : `disabled` sort la parcelle du
    // parcours de tabulation, donc ses deux dates deviennent injoignables sans
    // souris. Le refus d'ouvrir passe par `aria-disabled`, qui dit la même
    // chose sans retirer le focus.
    expect(accesClavier(false).disabled).toBe(false);
    expect(accesClavier(true).disabled).toBe(false);
  });

  it("annonce non activable la parcelle qui ne descend nulle part", () => {
    expect(accesClavier(false)["aria-disabled"]).toBe(true);
  });

  it("n'annonce rien de tel sur une parcelle qui s'ouvre", () => {
    expect(accesClavier(true)["aria-disabled"]).toBe(false);
  });
});

describe("ouvertureVisee - l'activation d'une parcelle muette ne mène nulle part", () => {
  it("une parcelle sans sous-dossier ne déclenche aucune navigation", () => {
    expect(ouvertureVisee("web/lib", false)).toBeNull();
  });

  it("une parcelle avec sous-dossiers s'ouvre comme avant", () => {
    expect(ouvertureVisee("web/lib", true)).toBe("web/lib");
  });
});

describe("annoncerParcelle - le nom accessible porte le chemin et les deux dates", () => {
  it("dit le chemin, le poids, l'état puis les deux dates", () => {
    expect(annoncerParcelle(ZONE, AU_REPOS).nomAccessible).toBe(
      "web/lib, 1.2k lignes, inactif, modifié il y a 2 j, relu il y a 20 min",
    );
  });

  it("porte les deux dates même sur une parcelle qui ne s'ouvre pas", () => {
    // C'est tout l'enjeu : cette parcelle-là est celle qu'on ne peut pas lire
    // à l'écran, et la seule qui n'avait aucun autre moyen de se dire.
    const { nomAccessible } = annoncerParcelle(ZONE, AU_REPOS);

    expect(nomAccessible).toContain("modifié il y a 2 j");
    expect(nomAccessible).toContain("relu il y a 20 min");
  });

  it("n'invite à ouvrir que ce qui s'ouvre", () => {
    expect(annoncerParcelle(ZONE, AU_REPOS).nomAccessible).not.toContain("ouvrir");
    expect(
      annoncerParcelle(ZONE, { ...AU_REPOS, ouvrable: true }).nomAccessible,
    ).toMatch(/, ouvrir$/);
  });

  it("garde les dates en DERNIER, après l'invitation à ouvrir ou non", () => {
    // L'ordre ne varie pas d'une parcelle à l'autre : deux voisines s'entendent
    // de la même façon, sans qu'on ait à réapprendre où tomber.
    const { nomAccessible } = annoncerParcelle(ZONE, { ...AU_REPOS, ouvrable: true });

    expect(nomAccessible).toBe(
      "web/lib, 1.2k lignes, inactif, modifié il y a 2 j, relu il y a 20 min, ouvrir",
    );
  });

  it("ajoute le worktree sans chasser les dates", () => {
    expect(
      annoncerParcelle(ZONE, { ...AU_REPOS, branches: ["feat/x", "feat/y"] })
        .nomAccessible,
    ).toBe(
      "web/lib, 1.2k lignes, inactif, worktree feat/x, feat/y, modifié il y a 2 j, relu il y a 20 min",
    );
  });

  it("se tait sur les dates tant que l'horloge du navigateur n'a pas démarré", () => {
    // Sans horloge, on ne peut juger l'âge de rien : annoncer « rien de récent »
    // serait un mensonge démenti à la seconde suivante.
    const { nomAccessible, infobulle } = annoncerParcelle(ZONE, {
      ...AU_REPOS,
      maintenant: null,
    });

    expect(nomAccessible).toBe("web/lib, 1.2k lignes, inactif");
    expect(infobulle).toBe("web/lib · 1.2k lignes · 12 fichiers");
  });

  it("dit « rien de récent » d'une zone que rien n'a touchée, sans l'omettre", () => {
    expect(
      annoncerParcelle(ZONE, { ...AU_REPOS, dates: AUCUNE_TOUCHE }).nomAccessible,
    ).toBe("web/lib, 1.2k lignes, inactif, rien de récent");
  });
});

describe("annoncerParcelle - l'infobulle porte les mêmes dates que le nom", () => {
  it("ajoute les deux dates au chemin, au poids et au nombre de fichiers", () => {
    expect(annoncerParcelle(ZONE, AU_REPOS).infobulle).toBe(
      "web/lib · 1.2k lignes · 12 fichiers · modifié il y a 2 j, relu il y a 20 min",
    );
  });

  it("dit les mêmes dates que le nom accessible, jamais une version à part", () => {
    const { nomAccessible, infobulle } = annoncerParcelle(ZONE, AU_REPOS);
    const dates = "modifié il y a 2 j, relu il y a 20 min";

    expect(nomAccessible).toContain(dates);
    expect(infobulle).toContain(dates);
  });

  it("garde le chemin brut, celui qu'on retrouvera dans son éditeur", () => {
    expect(
      annoncerParcelle({ chemin: "web/.", loc: 40, fichiers: 3 }, AU_REPOS).infobulle,
    ).toMatch(/^web\/\. · /);
  });
});

describe("cheminParle - ce qu'un lecteur d'écran entend du chemin", () => {
  it("dit le chemin entier, et pas seulement son dernier segment", () => {
    // « lib » seul, entendu sans la carte sous les yeux, ne désigne rien.
    expect(cheminParle("web/lib")).toBe("web/lib");
  });

  it("nomme le dossier dont on écoute les fichiers posés à plat", () => {
    expect(cheminParle("web/.")).toBe("fichiers de web");
    expect(cheminParle(".")).toBe("fichiers à la racine");
  });

  it("contient le nom affiché sur la parcelle (WCAG 2.5.3)", () => {
    // Qui commande l'écran à la voix nomme ce qu'il VOIT : le nom accessible
    // doit contenir le texte visible, sinon « clique sur lib » n'atteint rien.
    for (const chemin of ["web/lib", "web/.", ".", "daemon"]) {
      expect(cheminParle(chemin)).toContain(nomParcelle(chemin));
    }
  });
});

describe("poidsEnMots - le poids d'une zone en clair", () => {
  it("accorde le singulier plutôt que d'écrire « 1 lignes »", () => {
    expect(poidsEnMots(1)).toBe("1 ligne");
    expect(poidsEnMots(2)).toBe("2 lignes");
    expect(poidsEnMots(0)).toBe("0 lignes");
  });

  it("passe au millier avec une décimale, puis l'abandonne au-delà de dix mille", () => {
    expect(poidsEnMots(1240)).toBe("1.2k lignes");
    expect(poidsEnMots(12400)).toBe("12k lignes");
  });
});

describe("annoncerParcelle - un seul fichier ne s'écrit pas « 1 fichiers »", () => {
  it("accorde le singulier dans l'infobulle", () => {
    expect(
      annoncerParcelle({ chemin: "web/lib", loc: 40, fichiers: 1 }, AU_REPOS).infobulle,
    ).toContain("· 1 fichier ·");
  });
});

describe("ancrerInfobulle - l'infobulle ne sort pas du plan", () => {
  it("s'aligne à gauche et s'ouvre dessous quand la parcelle penche à gauche", () => {
    expect(ancrerInfobulle({ x: 0, y: 0, largeur: 30, hauteur: 20 })).toEqual({
      left: "0%",
      top: "calc(20% + 4px)",
    });
  });

  it("s'aligne par la DROITE quand la parcelle penche à droite du plan", () => {
    // Alignée à gauche, elle sortirait par le bord droit : le texte de
    // l'infobulle est bien plus long que la parcelle qui la porte.
    expect(ancrerInfobulle({ x: 88, y: 10, largeur: 12, hauteur: 15 })).toEqual({
      right: "0%",
      top: "calc(25% + 4px)",
    });
  });

  it("bascule AU-DESSUS quand le bas de la parcelle touche le bas du plan", () => {
    expect(ancrerInfobulle({ x: 5, y: 75, largeur: 20, hauteur: 25 })).toEqual({
      left: "5%",
      bottom: "calc(25% + 4px)",
    });
  });

  it("mesure la bascule sur le BAS de la parcelle, pas sur son sommet", () => {
    // Sinon une parcelle qui commence haut et descend bas ouvrirait sa bulle
    // sous le plan.
    expect(ancrerInfobulle({ x: 0, y: 30, largeur: 40, hauteur: 50 }).bottom).toBe(
      "calc(70% + 4px)",
    );
  });

  it("se pose DANS la parcelle qui occupe toute la hauteur du plan", () => {
    // Ni dessous ni dessus : il n'y a de place nulle part hors du plan, et une
    // bulle sortie par le haut recouvrirait le fil d'Ariane. Elle se cale
    // contre le bord haut de la parcelle, dont le texte est en bas.
    expect(ancrerInfobulle({ x: 57, y: 0, largeur: 43, hauteur: 100 })).toEqual({
      right: "0%",
      top: "calc(0% + 4px)",
    });
  });

  it("ne sort jamais du plan, quel que soit le cadre de la parcelle", () => {
    // Le garde-fou : aucun ancrage ne doit pousser la bulle hors du plan, ni
    // par le bas ni par le haut, où elle irait mordre sur le fil d'Ariane.
    for (let y = 0; y <= 90; y += 5) {
      for (let hauteur = 5; y + hauteur <= 100; hauteur += 5) {
        const { top, bottom } = ancrerInfobulle({ x: 0, y, largeur: 10, hauteur });
        const pose = Number((top ?? bottom)!.match(/([\d.]+)%/)![1]);

        expect(pose).toBeGreaterThanOrEqual(0);
        expect(pose).toBeLessThanOrEqual(SEUIL_DESSOUS);
      }
    }
  });

  it("n'accroche jamais deux bords opposés à la fois", () => {
    for (const cadre of [
      { x: 0, y: 0, largeur: 100, hauteur: 100 },
      { x: 49, y: 60, largeur: 3, hauteur: 12 },
      { x: 60, y: 0, largeur: 40, hauteur: 8 },
    ]) {
      const ancrage = ancrerInfobulle(cadre);

      expect(ancrage.left === undefined || ancrage.right === undefined).toBe(true);
      expect(ancrage.top === undefined || ancrage.bottom === undefined).toBe(true);
      expect(ancrage.left ?? ancrage.right).toBeDefined();
      expect(ancrage.top ?? ancrage.bottom).toBeDefined();
    }
  });

  it("arrondit les flottants du découpage plutôt que de les rendre bruts", () => {
    // Le treemap rend des pourcentages à quinze décimales : un style qui change
    // à chaque rendu ferait retravailler le navigateur pour rien.
    expect(ancrerInfobulle({ x: 1 / 3, y: 0, largeur: 10, hauteur: 1 / 3 }).left).toBe(
      "0.33%",
    );
  });

  it("mesure la bascule sur le centre de la parcelle, pas sur son bord gauche", () => {
    // Une parcelle large qui commence à gauche mais déborde à droite reste
    // ancrée à gauche tant que son centre y est.
    expect(ancrerInfobulle({ x: 20, y: 0, largeur: 50, hauteur: 10 }).left).toBe("20%");
    expect(ancrerInfobulle({ x: 45, y: 0, largeur: 50, hauteur: 10 }).right).toBe("5%");
  });
});
