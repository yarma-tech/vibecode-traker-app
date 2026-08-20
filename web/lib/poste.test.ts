import { describe, it, expect } from "vitest";
import {
  bandeauDuLecteur,
  compteDepots,
  derniereCartographie,
  ligneDossier,
  pontOuvert,
  sectionDossiers,
  type DossierSurveille,
} from "./poste";

// Le partage des sources de FR-059 est ce que ce fichier éprouve : ce qui vient
// du pont (les dossiers, leur compte, leur lisibilité) et ce qui vient de la
// base (l'heure de la dernière cartographie) ne se répondent jamais l'un pour
// l'autre. Et FR-060 : sans pont, la section devient une mention, jamais un
// vide ni une erreur.

/** Ce qu'un dossier bien portant rend. */
function dossier(reste: Partial<DossierSurveille> = {}): DossierSurveille {
  return {
    chemin: "~/Developer",
    emplacement: "/Users/moi/Developer",
    lisibilite: "lisible",
    depots: 12,
    ...reste,
  };
}

describe("pontOuvert - l'interface sait où elle tourne (FR-060)", () => {
  it("dans l'application, le pont répond", () => {
    expect(pontOuvert({ __TAURI__: { core: { invoke: () => {} } } })).toBe(true);
  });

  it("dans un navigateur ordinaire, il n'y a pas de pont", () => {
    expect(pontOuvert({})).toBe(false);
  });

  it("sans fenêtre du tout - rendu côté serveur -, il n'y a pas de pont", () => {
    expect(pontOuvert(undefined)).toBe(false);
    expect(pontOuvert(null)).toBe(false);
  });

  it("un pont à moitié là n'est pas un pont : on ne l'appellera pas", () => {
    expect(pontOuvert({ __TAURI__: {} })).toBe(false);
    expect(pontOuvert({ __TAURI__: { core: {} } })).toBe(false);
    expect(pontOuvert({ __TAURI__: { core: { invoke: "pas une fonction" } } })).toBe(false);
  });
});

describe("sectionDossiers - ce que la section montre", () => {
  it("sans pont : la mention, jamais une liste vide (FR-060)", () => {
    expect(sectionDossiers(null)).toEqual({ quoi: "hors_application" });
  });

  it("avec pont : la liste que le poste rend", () => {
    const dossiers = [dossier()];
    expect(sectionDossiers({ etat: "lue", dossiers })).toEqual({ quoi: "liste", dossiers });
  });

  it("une configuration illisible n'est pas une liste vide", () => {
    expect(
      sectionDossiers({ etat: "sans_configuration", raison: "aucune configuration à /x" }),
    ).toEqual({ quoi: "sans_configuration", raison: "aucune configuration à /x" });
  });

  it("une configuration qui ne surveille rien : le seul vrai vide", () => {
    expect(sectionDossiers({ etat: "lue", dossiers: [] })).toEqual({ quoi: "aucun" });
  });

  it("hors application, aucune section ne prétend savoir ce qui est surveillé", () => {
    const hors = sectionDossiers(null);
    expect(hors.quoi).not.toBe("aucun");
    expect(hors.quoi).not.toBe("liste");
  });

  it("un pont présent mais muet ne se déguise pas en navigateur ordinaire", () => {
    expect(sectionDossiers({ etat: "sans_reponse", raison: "commande inconnue" })).toEqual({
      quoi: "sans_reponse",
      raison: "commande inconnue",
    });
  });
});

describe("ligneDossier - ce qu'une ligne dit (FR-028, FR-029)", () => {
  it("un dossier lisible montre son compte, sans rien à signaler", () => {
    expect(ligneDossier(dossier({ depots: 12 }))).toEqual({ compte: "12 dépôts", signal: null });
  });

  it("un dossier sans aucun dépôt le dit en clair, et reste une ligne ordinaire", () => {
    expect(ligneDossier(dossier({ depots: 0 }))).toEqual({ compte: "aucun dépôt", signal: null });
  });

  it("un dossier introuvable est signalé sur sa ligne", () => {
    const ligne = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }));
    expect(ligne.signal).toBe("Dossier introuvable");
  });

  it("un dossier introuvable n'affiche aucun compte : zéro se lirait « vide »", () => {
    expect(ligneDossier(dossier({ lisibilite: "introuvable", depots: null })).compte).toBe(null);
  });

  it("un accès refusé se distingue d'un dossier disparu : les deux se corrigent autrement", () => {
    const refuse = ligneDossier(dossier({ lisibilite: "autorisation_refusee", depots: null }));
    const disparu = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }));
    expect(refuse.signal).toBe("Accès refusé");
    expect(refuse.signal).not.toBe(disparu.signal);
  });

  it("un chemin qui n'est pas un dossier se signale aussi", () => {
    expect(ligneDossier(dossier({ lisibilite: "pas_un_dossier", depots: null })).signal).toBe(
      "Pas un dossier",
    );
  });

  it("un compte manquant sur un dossier dit lisible ne s'affiche pas comme zéro", () => {
    expect(ligneDossier(dossier({ depots: null })).compte).toBe(null);
  });
});

describe("compteDepots - une phrase, pas une case de tableau", () => {
  it("aucun dépôt", () => {
    expect(compteDepots(0)).toBe("aucun dépôt");
  });

  it("un seul dépôt reste au singulier", () => {
    expect(compteDepots(1)).toBe("1 dépôt");
  });

  it("plusieurs dépôts", () => {
    expect(compteDepots(7)).toBe("7 dépôts");
  });
});

// L'heure de la dernière cartographie ne vient PAS du pont : elle est en base,
// avec le catalogue (FR-059), et l'écran l'établit comme un agrégat (FR-086).

const MAINTENANT = Date.parse("2026-08-20T12:00:00Z");

describe("derniereCartographie - la plus récente, une seule (FR-030, FR-086)", () => {
  it("aucune heure : rien n'a jamais été cartographié ici", () => {
    expect(derniereCartographie([], MAINTENANT)).toEqual({ etat: "jamais" });
  });

  it("des dépôts sans heure ne valent pas une cartographie", () => {
    expect(derniereCartographie([null, undefined, ""], MAINTENANT)).toEqual({ etat: "jamais" });
  });

  it("deux dépôts cartographiés à des moments différents : la plus récente gagne", () => {
    const cartographie = derniereCartographie(
      ["2026-08-20T11:00:00Z", "2026-08-20T11:55:00Z"],
      MAINTENANT,
    );
    expect(cartographie).toEqual({
      etat: "connue",
      quand: Date.parse("2026-08-20T11:55:00Z"),
      age: "5 min",
    });
  });

  it("l'ordre des dépôts n'y change rien : c'est un agrégat, pas le premier venu", () => {
    const heures = ["2026-08-20T11:55:00Z", "2026-08-20T11:00:00Z"];
    expect(derniereCartographie(heures, MAINTENANT)).toEqual(
      derniereCartographie([...heures].reverse(), MAINTENANT),
    );
  });

  it("une heure illisible est ignorée, jamais propagée à l'agrégat", () => {
    const cartographie = derniereCartographie(["pas une date", "2026-08-20T11:00:00Z"], MAINTENANT);
    expect(cartographie).toEqual({
      etat: "connue",
      quand: Date.parse("2026-08-20T11:00:00Z"),
      age: "1 h",
    });
  });

  it("une seule heure illisible ne fait pas conclure « jamais cartographié »", () => {
    expect(derniereCartographie(["pas une date"], MAINTENANT)).toEqual({ etat: "jamais" });
  });

  it("une heure venue du futur ne se dit jamais en négatif", () => {
    const cartographie = derniereCartographie(["2026-08-20T12:00:30Z"], MAINTENANT);
    expect(cartographie).toEqual({
      etat: "connue",
      quand: Date.parse("2026-08-20T12:00:30Z"),
      age: "0 s",
    });
  });
});

// Le bandeau du lecteur (FR-009, FR-010) : la tranche #62 a livré l'état et la
// relance, mais ils ne se voyaient que sur la page d'attente de l'application,
// faute de pont. Maintenant qu'il s'ouvre, l'interface peut le dire par-dessus
// n'importe quel écran - et elle ne le dit que quand il y a quelque chose à
// faire.

describe("bandeauDuLecteur - ne parle que quand ça cloche", () => {
  it("hors de l'application, l'interface ne prétend rien savoir du poste", () => {
    expect(bandeauDuLecteur(null)).toEqual({ visible: false });
  });

  it("un lecteur qui tourne n'a rien à annoncer par-dessus la carte", () => {
    expect(bandeauDuLecteur({ etat: "en_marche" })).toEqual({ visible: false });
  });

  it("un démarrage en cours n'appelle aucun geste : pas de bandeau", () => {
    expect(bandeauDuLecteur({ etat: "en_demarrage" })).toEqual({ visible: false });
  });

  it("un lecteur arrêté se signale et propose de le relancer", () => {
    const bandeau = bandeauDuLecteur({ etat: "arrete" });
    expect(bandeau.visible).toBe(true);
    if (!bandeau.visible) return;
    expect(bandeau.titre).toBe("Le lecteur est arrêté");
    expect(bandeau.relance).toBe(true);
  });

  it("un arrêt inattendu dit ce qu'on y perd, jamais seulement qu'il a cessé", () => {
    const bandeau = bandeauDuLecteur({
      etat: "en_echec",
      cas: "arret_inattendu",
      raison: "le lecteur s'est arrêté de lui-même.",
    });
    expect(bandeau.visible).toBe(true);
    if (!bandeau.visible) return;
    expect(bandeau.explication).toContain("n'envoie plus rien");
  });

  it("chaque cause connue rend un titre distinct : elles ne se corrigent pas pareil", () => {
    const titres = (["poste_tenu", "jeton_refuse", "arret_inattendu", "panne"] as const).map(
      (cas) => {
        const bandeau = bandeauDuLecteur({ etat: "en_echec", cas, raison: "raison brute" });
        return bandeau.visible ? bandeau.titre : "";
      },
    );
    expect(new Set(titres).size).toBe(titres.length);
  });

  it("la raison du poste est reprise en clair : elle nomme ce qu'il faut arrêter", () => {
    const bandeau = bandeauDuLecteur({
      etat: "en_echec",
      cas: "poste_tenu",
      raison: "un lecteur tourne déjà : « vibemap ».",
    });
    expect(bandeau.visible).toBe(true);
    if (!bandeau.visible) return;
    expect(bandeau.explication).toContain("vibemap");
    expect(bandeau.relance).toBe(true);
  });
});
