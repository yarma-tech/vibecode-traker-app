import { describe, it, expect } from "vitest";
import {
  aDecouvrirDuPoste,
  bandeauDuLecteur,
  compteDepots,
  derniereCartographie,
  etatCartographie,
  ligneDossier,
  pontOuvert,
  sectionDossiers,
  silenceCartographie,
  suiteDeLAjout,
  type DossierSurveille,
  type Surveillance,
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
    expect(ligneDossier(dossier({ depots: 12 }), "connue")).toEqual({
      compte: "12 dépôts",
      explication: null,
      signal: null,
    });
  });

  it("un dossier sans aucun dépôt le dit en clair, et reste une ligne ordinaire", () => {
    const ligne = ligneDossier(dossier({ depots: 0 }), "connue");
    expect(ligne.compte).toBe("aucun dépôt");
    expect(ligne.signal).toBe(null);
  });

  it("un dossier introuvable est signalé sur sa ligne", () => {
    const ligne = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "connue");
    expect(ligne.signal).toBe("Dossier introuvable");
  });

  it("un dossier introuvable n'affiche aucun compte : zéro se lirait « vide »", () => {
    expect(
      ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "connue").compte,
    ).toBe(null);
  });

  it("un accès refusé se distingue d'un dossier disparu : les deux se corrigent autrement", () => {
    const refuse = ligneDossier(
      dossier({ lisibilite: "autorisation_refusee", depots: null }),
      "connue",
    );
    const disparu = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "connue");
    expect(refuse.signal).toBe("Accès refusé");
    expect(refuse.signal).not.toBe(disparu.signal);
  });

  it("un chemin qui n'est pas un dossier se signale aussi", () => {
    expect(
      ligneDossier(dossier({ lisibilite: "pas_un_dossier", depots: null }), "connue").signal,
    ).toBe("Pas un dossier");
  });

  it("un compte manquant sur un dossier dit lisible ne s'affiche pas comme zéro", () => {
    expect(ligneDossier(dossier({ depots: null }), "connue").compte).toBe(null);
  });
});

// FR-076 (issue #73) : la cartographie ne regarde que les enfants DIRECTS d'un
// dossier surveillé. Le geste le plus probable au sélecteur - choisir un dossier
// qui est lui-même un dépôt - ne remonte donc rien, et l'écran doit le dire au
// lieu d'afficher un compte nul sans explication.

describe("ligneDossier - un compte nul ne reste pas sans explication (FR-076)", () => {
  it("aucun dépôt : la ligne nomme ce qui a été cherché", () => {
    const ligne = ligneDossier(dossier({ depots: 0 }), "connue");
    expect(ligne.explication).toContain(".git");
    expect(ligne.explication).toContain("enfants directs");
  });

  it("l'explication dit quoi faire, pas seulement ce qui a été cherché", () => {
    const ligne = ligneDossier(dossier({ depots: 0 }), "connue");
    expect(ligne.explication).toMatch(/lui-même un dépôt/);
    expect(ligne.explication).toMatch(/ajoutez/i);
  });

  it("l'explication n'invente aucune cause : elle ne dit ni « vide » ni « renommé »", () => {
    // Un dossier vide et un dossier qui est lui-même un dépôt rendent tous deux
    // zéro, et le poste ne les distingue pas (`bureau/tests/pont.rs`). Une
    // phrase qui nommerait l'une des deux raisons serait fausse une fois sur
    // deux.
    const explication = ligneDossier(dossier({ depots: 0 }), "connue").explication ?? "";
    expect(explication).not.toMatch(/vide/i);
    expect(explication).not.toMatch(/renomm/i);
  });

  it("un compte non nul n'affiche aucune explication : ce serait du bruit", () => {
    expect(ligneDossier(dossier({ depots: 1 }), "connue").explication).toBe(null);
    expect(ligneDossier(dossier({ depots: 2 }), "connue").explication).toBe(null);
  });

  it("une ligne à signaler n'explique rien : elle n'a même pas de compte", () => {
    const ligne = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "connue");
    expect(ligne.explication).toBe(null);
  });
});

// FR-074 (issue #69) : tant qu'aucune cartographie n'a jamais abouti, un compte
// de zéro se lirait « ce dossier est vide » alors que rien n'a encore été
// regardé.

describe("ligneDossier - avant la première cartographie (FR-074)", () => {
  it("les dossiers annoncent une attente, jamais zéro dépôt", () => {
    const ligne = ligneDossier(dossier({ depots: 0 }), "jamais");
    expect(ligne.compte).toBe("en attente de la première cartographie");
    expect(ligne.compte).not.toMatch(/aucun dépôt/);
  });

  it("l'attente vaut aussi pour un dossier qui porte des dépôts sur le disque", () => {
    // Ils sont là, mais rien ne les a encore relevés : afficher « 12 dépôts »
    // ferait croire à une carte qui les porte déjà.
    expect(ligneDossier(dossier({ depots: 12 }), "jamais").compte).toBe(
      "en attente de la première cartographie",
    );
  });

  it("une attente n'appelle aucune explication : il n'y a encore rien à expliquer", () => {
    expect(ligneDossier(dossier({ depots: 0 }), "jamais").explication).toBe(null);
  });

  it("un dossier introuvable reste signalé : il le sera cartographie ou pas", () => {
    const ligne = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "jamais");
    expect(ligne.signal).toBe("Dossier introuvable");
    expect(ligne.compte).toBe(null);
  });

  it("base injoignable : le compte du poste reste vrai et s'affiche", () => {
    // L'heure vient de la base, le compte vient du poste. Une base muette ne
    // rend pas faux ce que le disque local dit à l'instant même.
    expect(ligneDossier(dossier({ depots: 12 }), "injoignable").compte).toBe("12 dépôts");
  });

  it("une cartographie qui n'a rien trouvé montre bien un compte, pas une attente", () => {
    expect(ligneDossier(dossier({ depots: 0 }), "sans_depot").compte).toBe("aucun dépôt");
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

// Ajouter un dossier (FR-031, FR-034, FR-036). Le sélecteur du système ne se
// teste pas ici - il n'y a pas de fenêtre du système dans un test -, mais tout
// ce que l'écran fait de la réponse, si : c'est là que se décide ce que
// l'utilisateur lit, et s'il doit recommencer ou non.

describe("suiteDeLAjout - ce que l'écran fait d'un ajout", () => {
  const liste: Surveillance = {
    etat: "lue",
    dossiers: [dossier(), dossier({ chemin: "~/Sites", emplacement: "/Users/moi/Sites", depots: 2 })],
  };

  it("un sélecteur refermé sans choix ne dit rien et ne change rien", () => {
    expect(suiteDeLAjout({ issue: "annule" })).toEqual({ annonce: null, surveillance: null });
  });

  it("un dossier ajouté est nommé, et l'écran affiche la liste rendue", () => {
    const suite = suiteDeLAjout({ issue: "ajoute", chemin: "~/Sites", surveillance: liste });
    expect(suite.surveillance).toBe(liste);
    expect(suite.annonce?.ton).toBe("succes");
    expect(suite.annonce?.texte).toContain("~/Sites");
  });

  it("l'annonce dit quand les dépôts arrivent : sinon on recommence, ou on va voir le fichier", () => {
    const suite = suiteDeLAjout({ issue: "ajoute", chemin: "~/Sites", surveillance: liste });
    expect(suite.annonce?.texte).toMatch(/minute/);
  });

  it("un refus dit la raison, et ne se fait pas passer pour un succès", () => {
    const suite = suiteDeLAjout({
      issue: "echoue",
      raison: "aucune configuration a /Users/moi/.config/vibemap/config.toml",
    });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("/Users/moi/.config/vibemap/config.toml");
  });

  it("un refus ne remplace pas la liste affichée : rien n'a bougé sur le poste", () => {
    expect(suiteDeLAjout({ issue: "echoue", raison: "disque plein" }).surveillance).toBeNull();
  });

  it("sans pont, il n'y a rien à annoncer : le bouton n'existe pas là-bas (FR-060)", () => {
    expect(suiteDeLAjout(null)).toEqual({ annonce: null, surveillance: null });
  });
});

// Refuser un dossier déjà surveillé, en NOMMANT le coupable (FR-037, FR-075,
// issue #71). La règle elle-même - `~` déplié, liens symboliques résolus, barre
// finale normalisée, emboîtement dans les deux sens - vit dans le poste et
// s'éprouve là (`bureau/tests/doublon.rs`). Ce qui se joue ici, c'est ce que
// l'utilisateur LIT du refus : sans le nom du dossier qui s'y oppose, il devrait
// ouvrir le fichier de configuration pour le chercher, ce que FR-036 interdit.

describe("suiteDeLAjout - un dossier déjà surveillé est refusé, et le refus nomme", () => {
  it("le même dossier : le refus nomme celui qui est déjà surveillé", () => {
    const suite = suiteDeLAjout({
      issue: "refuse",
      chemin: "~/Developer",
      deja: "~/Developer",
      cas: "meme",
    });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("~/Developer");
  });

  it("un dossier contenu : le refus nomme le dossier surveillé, pas seulement le choisi", () => {
    const suite = suiteDeLAjout({
      issue: "refuse",
      chemin: "~/Developer/vibecode-traker-app",
      deja: "~/Developer",
      cas: "contenu",
    });
    expect(suite.annonce?.texte).toContain("~/Developer");
    expect(suite.annonce?.texte).toContain("~/Developer/vibecode-traker-app");
  });

  it("un dossier qui en contient un : le refus dit lequel des deux est surveillé", () => {
    const suite = suiteDeLAjout({
      issue: "refuse",
      chemin: "~",
      deja: "~/Developer",
      cas: "contient",
    });
    expect(suite.annonce?.texte).toContain("~/Developer");
    expect(suite.annonce?.texte).toMatch(/deux fois|contient/);
  });

  it("les trois cas ne se corrigent pas pareil : trois phrases distinctes", () => {
    const textes = (["meme", "contenu", "contient"] as const).map(
      (cas) =>
        suiteDeLAjout({ issue: "refuse", chemin: "~", deja: "~/Developer", cas }).annonce?.texte,
    );
    expect(new Set(textes).size).toBe(textes.length);
  });

  it("chaque refus dit quoi faire, jamais seulement « déjà surveillé »", () => {
    for (const cas of ["meme", "contenu", "contient"] as const) {
      const texte =
        suiteDeLAjout({
          issue: "refuse",
          chemin: "~/ailleurs",
          deja: "~/Developer",
          cas,
        }).annonce?.texte ?? "";
      expect(texte).toMatch(/Rien n'a été ajouté|Choisissez|Retirez/);
    }
  });

  it("un refus ne remplace pas la liste affichée : rien n'a bougé sur le poste", () => {
    const suite = suiteDeLAjout({
      issue: "refuse",
      chemin: "~/Developer",
      deja: "~/Developer",
      cas: "meme",
    });
    expect(suite.surveillance).toBeNull();
  });

  it("un refus ne se donne pas pour un succès", () => {
    const suite = suiteDeLAjout({
      issue: "refuse",
      chemin: "~/Developer",
      deja: "~/Developer",
      cas: "meme",
    });
    expect(suite.annonce?.ton).not.toBe("succes");
    expect(suite.annonce?.texte).not.toMatch(/d'ici une minute/);
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

// Les trois silences (FR-074, FR-085, FR-087, issue #69). Ils se ressemblent et
// ne disent pas la même chose : « jamais cartographié », « cartographié sans
// rien trouver » et « la base est injoignable » appellent trois gestes
// différents, et les confondre envoie chercher une panne là où il n'y en a pas.

describe("aDecouvrirDuPoste - le fait local, ou personne pour le dire (FR-087)", () => {
  it("le pont a lu la configuration : le fait est là", () => {
    expect(aDecouvrirDuPoste({ etat: "lue", dossiers: [], a_decouvrir: "rien" })).toBe("rien");
    expect(
      aDecouvrirDuPoste({ etat: "lue", dossiers: [dossier()], a_decouvrir: "des_depots" }),
    ).toBe("des_depots");
  });

  it("hors de l'application, personne ne le dit : l'écran s'en tient à la base (FR-060)", () => {
    expect(aDecouvrirDuPoste(null)).toBeNull();
  });

  it("une configuration illisible ne se conclut pas « rien à cartographier »", () => {
    expect(aDecouvrirDuPoste({ etat: "sans_configuration", raison: "aucune config" })).toBeNull();
    expect(aDecouvrirDuPoste({ etat: "sans_reponse", raison: "commande inconnue" })).toBeNull();
  });

  it("un pont d'avant cette tranche ne dit rien plutôt que n'importe quoi", () => {
    expect(aDecouvrirDuPoste({ etat: "lue", dossiers: [dossier()] })).toBeNull();
  });
});

describe("silenceCartographie - les trois silences ne se confondent pas", () => {
  const RIEN: Parameters<typeof silenceCartographie>[0] = {
    heures: [],
    baseInjoignable: false,
    aDecouvrir: null,
  };

  it("aucune heure, et des dépôts à cartographier : jamais cartographié (FR-074)", () => {
    expect(silenceCartographie({ ...RIEN, aDecouvrir: "des_depots" })).toBe("jamais");
  });

  it("aucune heure, et rien à cartographier : la cartographie a abouti sur du vide (FR-087)", () => {
    expect(silenceCartographie({ ...RIEN, aDecouvrir: "rien" })).toBe("sans_depot");
  });

  it("« rien trouvé » ne se dit jamais « jamais » : c'est tout l'objet de FR-087", () => {
    expect(silenceCartographie({ ...RIEN, aDecouvrir: "rien" })).not.toBe("jamais");
  });

  it("la base injoignable ne se conclut pas « jamais cartographié » (FR-085)", () => {
    const silence = silenceCartographie({ ...RIEN, baseInjoignable: true, aDecouvrir: "rien" });
    expect(silence).toBe("injoignable");
    expect(silence).not.toBe("jamais");
    expect(silence).not.toBe("sans_depot");
  });

  it("une base muette l'emporte même sur des heures déjà lues : elles ne sont pas à jour", () => {
    expect(
      silenceCartographie({
        heures: ["2026-08-20T11:00:00Z"],
        baseInjoignable: true,
        aDecouvrir: "des_depots",
      }),
    ).toBe("injoignable");
  });

  it("une heure connue l'emporte sur le fait local : elle est un fait, lui une déduction", () => {
    expect(
      silenceCartographie({
        heures: ["2026-08-20T11:00:00Z"],
        baseInjoignable: false,
        aDecouvrir: "rien",
      }),
    ).toBe("connue");
  });

  it("un dossier illisible ne fait pas conclure « rien trouvé »", () => {
    expect(silenceCartographie({ ...RIEN, aDecouvrir: "inconnu" })).toBe("jamais");
  });

  it("hors de l'application, sans fait local, on s'en tient à ce que la base porte", () => {
    expect(silenceCartographie({ ...RIEN, aDecouvrir: null })).toBe("jamais");
    expect(
      silenceCartographie({ heures: ["2026-08-20T11:00:00Z"], baseInjoignable: false, aDecouvrir: null }),
    ).toBe("connue");
  });

  it("une heure illisible ne vaut pas une cartographie", () => {
    expect(silenceCartographie({ ...RIEN, heures: ["pas une date"], aDecouvrir: "rien" })).toBe(
      "sans_depot",
    );
  });
});

describe("etatCartographie - le silence, et l'heure quand il y en a une", () => {
  it("l'heure connue est la plus récente, et une seule (FR-086)", () => {
    expect(
      etatCartographie(
        {
          heures: ["2026-08-20T11:00:00Z", "2026-08-20T11:55:00Z"],
          baseInjoignable: false,
          aDecouvrir: "des_depots",
        },
        MAINTENANT,
      ),
    ).toEqual({
      etat: "connue",
      quand: Date.parse("2026-08-20T11:55:00Z"),
      age: "5 min",
    });
  });

  it("une heure de la veille reste affichée : elle ne devient pas « jamais »", () => {
    // Le poste a été cartographié hier, l'application a été quittée puis
    // rouverte. La base porte toujours l'heure d'hier, et c'est elle qu'on lit.
    const veille = "2026-08-19T09:30:00Z";
    const etat = etatCartographie(
      { heures: [veille], baseInjoignable: false, aDecouvrir: "des_depots" },
      MAINTENANT,
    );
    expect(etat.etat).toBe("connue");
    if (etat.etat !== "connue") return;
    expect(etat.quand).toBe(Date.parse(veille));
  });

  it("les quatre états sont distincts : l'écran ne peut pas les confondre", () => {
    const etats = [
      etatCartographie({ heures: [], baseInjoignable: true, aDecouvrir: null }, MAINTENANT).etat,
      etatCartographie({ heures: [], baseInjoignable: false, aDecouvrir: "rien" }, MAINTENANT)
        .etat,
      etatCartographie(
        { heures: [], baseInjoignable: false, aDecouvrir: "des_depots" },
        MAINTENANT,
      ).etat,
      etatCartographie(
        { heures: ["2026-08-20T11:00:00Z"], baseInjoignable: false, aDecouvrir: null },
        MAINTENANT,
      ).etat,
    ];
    expect(etats).toEqual(["injoignable", "sans_depot", "jamais", "connue"]);
  });

  it("un silence ne porte jamais d'heure : il n'y en a pas à porter", () => {
    for (const etat of [
      etatCartographie({ heures: [], baseInjoignable: true, aDecouvrir: null }, MAINTENANT),
      etatCartographie({ heures: [], baseInjoignable: false, aDecouvrir: "rien" }, MAINTENANT),
      etatCartographie({ heures: [], baseInjoignable: false, aDecouvrir: null }, MAINTENANT),
    ]) {
      expect(etat).not.toHaveProperty("quand");
    }
  });

  it("aucune horloge réelle n'est lue : deux appels au même instant se répondent", () => {
    const faits = {
      heures: ["2026-08-20T11:00:00Z"],
      baseInjoignable: false,
      aDecouvrir: "des_depots" as const,
    };
    expect(etatCartographie(faits, MAINTENANT)).toEqual(etatCartographie(faits, MAINTENANT));
    // Un `maintenant` plus tard vieillit la même heure : c'est bien le
    // paramètre qui décide de l'âge, et non une horloge lue au passage.
    expect(etatCartographie(faits, MAINTENANT + 3_600_000)).not.toEqual(
      etatCartographie(faits, MAINTENANT),
    );
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
