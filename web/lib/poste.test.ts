import { describe, it, expect } from "vitest";
import {
  aDecouvrirDuPoste,
  avancementDepouillement,
  bandeauDuLecteur,
  compteDepots,
  derniereCartographie,
  echecDeLaRedemande,
  etatCartographie,
  journauxDepouilles,
  ligneDossier,
  pontOuvert,
  sectionDossiers,
  sectionPoste,
  silenceCartographie,
  suiteDeLAjout,
  suiteDeLaReautorisation,
  suiteDuRetrait,
  type DossierSurveille,
  type FaitsDuPoste,
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
      redemande: false,
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
    // FR-061 nomme la cause plutôt que le symptôme : ce n'est pas un accès qui a
    // échoué, c'est une autorisation que le système n'a pas encore donnée.
    expect(refuse.signal).toBe("Autorisation manquante");
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

// Retirer un dossier sans effacer ce qu'on a déjà observé (FR-032, FR-035,
// FR-036, issue #72). Ce que le poste fait du fichier de configuration s'éprouve
// dans `bureau/tests/retrait.rs` ; ce qui se joue ici, c'est ce que
// l'utilisateur LIT du retrait - et notamment qu'il n'a rien perdu.

describe("suiteDuRetrait - ce que l'écran fait d'un retrait", () => {
  const liste: Surveillance = { etat: "lue", dossiers: [dossier()] };

  it("un dossier retiré est nommé, et l'écran affiche la liste rendue", () => {
    const suite = suiteDuRetrait({ issue: "retire", chemin: "~/Sites", surveillance: liste });
    expect(suite.surveillance).toBe(liste);
    expect(suite.annonce?.ton).toBe("succes");
    expect(suite.annonce?.texte).toContain("~/Sites");
  });

  it("l'annonce dit ce que deviennent les dépôts : sinon retirer se lit comme effacer", () => {
    // FR-035, et le titre même de la tranche. « Retiré » tout seul laisserait
    // croire que des semaines de carte viennent de partir avec le dossier.
    const texte =
      suiteDuRetrait({ issue: "retire", chemin: "~/Sites", surveillance: liste }).annonce?.texte ??
      "";
    expect(texte).toMatch(/catalogue/);
    expect(texte).toMatch(/figés/);
    expect(texte).not.toMatch(/supprim|efface/i);
  });

  it("l'annonce dit comment revenir en arrière : le retrait n'a rien de définitif", () => {
    const texte =
      suiteDuRetrait({ issue: "retire", chemin: "~/Sites", surveillance: liste }).annonce?.texte ??
      "";
    expect(texte).toMatch(/rajoutez/i);
  });

  it("retirer un dossier absent ne se donne pas pour un succès, et le dit", () => {
    const suite = suiteDuRetrait({ issue: "inconnu", chemin: "~/Sites", surveillance: liste });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("~/Sites");
    expect(suite.annonce?.texte).not.toMatch(/catalogue/);
  });

  it("un retrait sans objet remet quand même la liste d'aplomb : elle avait vieilli", () => {
    // C'est la seule explication possible d'un dossier qu'on croyait surveillé
    // et qui ne l'est plus. Garder la liste affichée laisserait le bouton
    // proposer une seconde fois un geste sans objet.
    expect(
      suiteDuRetrait({ issue: "inconnu", chemin: "~/Sites", surveillance: liste }).surveillance,
    ).toBe(liste);
  });

  it("un échec dit la raison, et ne remplace pas la liste : rien n'a bougé", () => {
    const suite = suiteDuRetrait({ issue: "echoue", raison: "disque plein" });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("disque plein");
    expect(suite.surveillance).toBeNull();
  });

  it("sans pont, il n'y a rien à annoncer : la liste n'est pas là non plus (FR-060)", () => {
    expect(suiteDuRetrait(null)).toEqual({ annonce: null, surveillance: null });
  });

  it("les trois issues ne se lisent pas pareil", () => {
    const textes = [
      suiteDuRetrait({ issue: "retire", chemin: "~/Sites", surveillance: liste }).annonce?.texte,
      suiteDuRetrait({ issue: "inconnu", chemin: "~/Sites", surveillance: liste }).annonce?.texte,
      suiteDuRetrait({ issue: "echoue", raison: "disque plein" }).annonce?.texte,
    ];
    expect(new Set(textes).size).toBe(textes.length);
  });
});

// Signaler une autorisation manquante et la redemander (FR-061, issue #74).
// macOS ferme `~/Documents`, `~/Desktop`, `~/Downloads` et les volumes externes
// tant que l'autorisation n'est pas donnée, et peut reprendre un accès accordé.
// Un dossier plein de dépôts se lirait alors comme un dossier vide.

describe("ligneDossier - une autorisation manquante se dit sur la ligne (FR-061)", () => {
  const manquante = () =>
    ligneDossier(dossier({ lisibilite: "autorisation_refusee", depots: null }), "connue");

  it("la ligne nomme ce qui manque, et non « 0 dépôt »", () => {
    const ligne = manquante();
    expect(ligne.signal).toBe("Autorisation manquante");
    expect(ligne.compte).toBe(null);
  });

  it("la ligne dit quoi faire, pas seulement ce qui cloche", () => {
    expect(manquante().explication).toMatch(/[Rr]edemandez/);
  });

  it("l'explication dit ce que le bouton va ouvrir : un sélecteur qui surgit sans prévenir se referme", () => {
    expect(manquante().explication).toMatch(/sélecteur/);
    expect(manquante().explication).toMatch(/même dossier/);
  });

  it("le bouton de redemande n'apparaît que sur l'autorisation manquante", () => {
    expect(manquante().redemande).toBe(true);
    expect(
      ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "connue").redemande,
    ).toBe(false);
    expect(
      ligneDossier(dossier({ lisibilite: "pas_un_dossier", depots: null }), "connue").redemande,
    ).toBe(false);
    expect(ligneDossier(dossier({ depots: 12 }), "connue").redemande).toBe(false);
    expect(ligneDossier(dossier({ depots: 0 }), "connue").redemande).toBe(false);
  });

  it("un dossier renommé n'appelle ni la phrase ni le bouton de l'autorisation", () => {
    // Ce n'est pas la même cause et ce n'est pas le même geste : le sélecteur du
    // système ne fait pas réapparaître un dossier qui n'est plus là.
    const disparu = ligneDossier(dossier({ lisibilite: "introuvable", depots: null }), "connue");
    expect(disparu.signal).toBe("Dossier introuvable");
    expect(disparu.explication).toBe(null);
  });

  it("l'autorisation manquante se dit même avant la première cartographie", () => {
    // Le signal vaut avant l'attente : une cartographie de plus n'ouvrira pas un
    // dossier que le système ferme.
    const ligne = ligneDossier(
      dossier({ lisibilite: "autorisation_refusee", depots: null }),
      "jamais",
    );
    expect(ligne.signal).toBe("Autorisation manquante");
    expect(ligne.redemande).toBe(true);
  });
});

describe("suiteDeLaReautorisation - ce que l'écran fait d'une redemande", () => {
  const liste: Surveillance = { etat: "lue", dossiers: [dossier()] };

  it("un sélecteur refermé sans choix ne dit rien et ne change rien", () => {
    expect(suiteDeLaReautorisation({ issue: "annulee" })).toEqual({
      annonce: null,
      surveillance: null,
    });
  });

  it("sans pont, il n'y a rien à annoncer : le bouton n'existe pas là-bas (FR-060)", () => {
    expect(suiteDeLaReautorisation(null)).toEqual({ annonce: null, surveillance: null });
  });

  it("accordée : la ligne reprend son affichage normal, et les dépôts arrivent", () => {
    const suite = suiteDeLaReautorisation({
      issue: "accordee",
      chemin: "~/Documents",
      surveillance: liste,
    });
    expect(suite.annonce?.ton).toBe("succes");
    expect(suite.annonce?.texte).toContain("~/Documents");
    expect(suite.annonce?.texte).toMatch(/minute/);
    expect(suite.surveillance).toBe(liste);
  });

  it("toujours refusée : le détour par les réglages du système, nommé pas à pas", () => {
    // Sans ce chemin-là, l'utilisateur recommencerait indéfiniment le même geste
    // sans effet : le sélecteur ne peut plus rien quand c'est le système qui
    // refuse.
    const suite = suiteDeLaReautorisation({
      issue: "refusee",
      chemin: "~/Documents",
      surveillance: liste,
    });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toMatch(/Réglages Système/);
    expect(suite.annonce?.texte).toMatch(/Confidentialité et sécurité/);
  });

  it("un refus ne se donne pas pour un succès", () => {
    const suite = suiteDeLaReautorisation({
      issue: "refusee",
      chemin: "~/Documents",
      surveillance: liste,
    });
    expect(suite.annonce?.ton).not.toBe("succes");
    expect(suite.annonce?.texte).not.toMatch(/d'ici une minute/);
  });

  it("un autre dossier désigné : les deux sont nommés, et le geste est redit", () => {
    const suite = suiteDeLaReautorisation({
      issue: "autre_dossier",
      attendu: "~/Documents",
      choisi: "~/Bureau",
    });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("~/Documents");
    expect(suite.annonce?.texte).toContain("~/Bureau");
    expect(suite.annonce?.texte).toMatch(/[Rr]edemandez/);
  });

  it("un autre dossier désigné ne remplace pas la liste : rien n'a bougé sur le poste", () => {
    expect(
      suiteDeLaReautorisation({
        issue: "autre_dossier",
        attendu: "~/Documents",
        choisi: "~/Bureau",
      }).surveillance,
    ).toBeNull();
  });

  it("un dossier qui n'est plus surveillé le dit, et remet la liste d'aplomb", () => {
    const suite = suiteDeLaReautorisation({
      issue: "inconnu",
      chemin: "~/Documents",
      surveillance: liste,
    });
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("~/Documents");
    expect(suite.surveillance).toBe(liste);
  });

  it("un appel qui n'aboutit pas ne se déguise pas en refus du système", () => {
    // Les cinq issues sont ce que le POSTE répond ; ici, il n'a rien répondu.
    // Annoncer « le système refuse » enverrait régler des autorisations qui n'y
    // sont pour rien.
    const suite = echecDeLaRedemande("~/Documents", "commande inconnue");
    expect(suite.annonce?.ton).toBe("echec");
    expect(suite.annonce?.texte).toContain("~/Documents");
    expect(suite.annonce?.texte).toContain("commande inconnue");
    expect(suite.annonce?.texte).not.toMatch(/Réglages Système/);
    expect(suite.surveillance).toBeNull();
  });

  it("les quatre issues qui parlent ne se lisent pas pareil", () => {
    const textes = [
      suiteDeLaReautorisation({ issue: "accordee", chemin: "~/Documents", surveillance: liste })
        .annonce?.texte,
      suiteDeLaReautorisation({ issue: "refusee", chemin: "~/Documents", surveillance: liste })
        .annonce?.texte,
      suiteDeLaReautorisation({ issue: "autre_dossier", attendu: "~/Documents", choisi: "~/Bureau" })
        .annonce?.texte,
      suiteDeLaReautorisation({ issue: "inconnu", chemin: "~/Documents", surveillance: liste })
        .annonce?.texte,
    ];
    expect(new Set(textes).size).toBe(textes.length);
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

// Les faits que le poste sait de lui-même : sa version, son nom, et où en est
// son dépouillement (FR-050, FR-065, FR-068). Comme les dossiers, ils viennent
// du pont et de lui seul ; comme eux, ils cèdent la place à la mention de
// FR-060 dans un navigateur ordinaire.

/** Un poste qui répond, avec le dépouillement qu'on lui donne. */
function poste(reste: Partial<FaitsDuPoste> = {}): FaitsDuPoste {
  return {
    version: "0.1.0",
    machine: "MacBook de Yarma",
    depouillement: { etat: "jamais" },
    ...reste,
  };
}

/** Un instant de référence, pour que rien ne dépende de l'horloge réelle. */
const MIDI = Date.parse("2026-08-19T12:00:00Z");

describe("sectionPoste - ce que la section montre (FR-060)", () => {
  it("sans pont : rien à montrer, et la mention des dossiers couvre le reste", () => {
    expect(sectionPoste(null)).toEqual({ quoi: "hors_application" });
  });

  it("avec pont : la version, le nom de la machine et le dépouillement", () => {
    const faits = poste();
    expect(sectionPoste(faits)).toEqual({ quoi: "faits", faits });
  });

  it("un pont présent mais muet ne se déguise pas en navigateur ordinaire", () => {
    expect(sectionPoste({ etat: "sans_reponse", raison: "le pont n'a rien rendu" })).toEqual({
      quoi: "sans_reponse",
      raison: "le pont n'a rien rendu",
    });
  });

  it("un poste qui ne sait pas se nommer garde sa version : c'est ce qu'on venait lire", () => {
    const section = sectionPoste(poste({ machine: null }));
    expect(section.quoi).toBe("faits");
    if (section.quoi !== "faits") return;
    expect(section.faits.machine).toBeNull();
    expect(section.faits.version).toBe("0.1.0");
  });
});

describe("avancementDepouillement - un dépouillement en cours (FR-050)", () => {
  it("montre les deux nombres : les journaux dépouillés sur le total", () => {
    const vu = avancementDepouillement(
      { etat: "en_cours", journaux: 120, total: 400 },
      MIDI,
    );
    expect(vu.etat).toBe("en_cours");
    expect(vu.texte).toBe("120 journaux dépouillés sur 400");
  });

  it("le premier journal ne se dit pas au pluriel", () => {
    const vu = avancementDepouillement({ etat: "en_cours", journaux: 1, total: 400 }, MIDI);
    expect(vu.texte).toBe("1 journal dépouillé sur 400");
  });

  it("une reprise repart de son avancement, jamais de zéro (FR-049, FR-078)", () => {
    // Quatre cents journaux dépouillés hier, vingt écrits depuis. « 0 sur 20 »
    // donnerait à croire que tout est à refaire ; c'est précisément ce que cette
    // tranche rend observable.
    const vu = avancementDepouillement({ etat: "en_cours", journaux: 400, total: 420 }, MIDI);
    expect(vu.texte).toBe("400 journaux dépouillés sur 420");
    expect(vu.texte).not.toContain("0 sur 20");
  });
});

describe("avancementDepouillement - un dépouillement terminé (FR-050)", () => {
  it("dit quand il s'est terminé, et depuis combien de temps", () => {
    const vu = avancementDepouillement(
      { etat: "termine", quand: "2026-08-19T11:58:00Z", journaux: 400 },
      MIDI,
    );
    expect(vu.etat).toBe("termine");
    if (vu.etat !== "termine") return;
    expect(vu.age).toBe("2 min");
    expect(vu.quand).toBe(Date.parse("2026-08-19T11:58:00Z"));
    expect(vu.texte).toContain("terminé il y a 2 min");
  });

  it("l'horloge est reçue en paramètre : la même fin vieillit avec l'écran", () => {
    const fini = { etat: "termine", quand: "2026-08-19T11:58:00Z", journaux: 400 } as const;
    const tout_de_suite = avancementDepouillement(fini, MIDI);
    const deux_heures_plus_tard = avancementDepouillement(fini, MIDI + 2 * 3600_000);
    expect(tout_de_suite.texte).not.toBe(deux_heures_plus_tard.texte);
  });

  it("une heure illisible n'efface pas le fait : le dépouillement s'est terminé", () => {
    const vu = avancementDepouillement(
      { etat: "termine", quand: "pas une date", journaux: 400 },
      MIDI,
    );
    expect(vu.etat).toBe("termine");
    if (vu.etat !== "termine") return;
    expect(vu.quand).toBeNull();
    expect(vu.age).toBeNull();
    expect(vu.texte).toContain("terminé");
    expect(vu.texte).not.toContain("NaN");
  });
});

describe("avancementDepouillement - rien à dépouiller, et jamais commencé", () => {
  it("un Mac sans aucun journal le dit tout de suite (FR-065)", () => {
    const vu = avancementDepouillement({ etat: "rien_a_depouiller" }, MIDI);
    expect(vu.etat).toBe("rien");
    expect(vu.texte).toBe("aucun journal à dépouiller sur cette machine");
  });

  it("ce même Mac ne montre ni « 0 sur 0 » ni un avancement figé", () => {
    const vu = avancementDepouillement({ etat: "rien_a_depouiller" }, MIDI);
    // Aucun chiffre du tout : « 0 sur 0 » se lirait comme un dépouillement qui
    // n'avance pas, et « 0 journal » comme un compte en attente d'un autre.
    expect(vu.texte).not.toMatch(/\d/);
  });

  it("un avancement sans dénominateur ne s'affiche pas comme « 0 sur 0 »", () => {
    // Le poste ne rend pas cette forme-là, mais l'écran ne doit pas afficher un
    // avancement qui ne peut pas bouger si cela arrivait.
    const vu = avancementDepouillement({ etat: "en_cours", journaux: 0, total: 0 }, MIDI);
    expect(vu.etat).toBe("rien");
    expect(vu.texte).not.toContain("0 sur 0");
  });

  it("un dépouillement jamais commencé ne se donne pas pour un dépouillement sans journal", () => {
    const vu = avancementDepouillement({ etat: "jamais" }, MIDI);
    expect(vu.etat).toBe("jamais");
    expect(vu.texte).toBe("en attente du premier dépouillement");
  });

  it("les quatre cas rendent quatre phrases distinctes : aucun ne se lit pour un autre", () => {
    const phrases = [
      avancementDepouillement({ etat: "jamais" }, MIDI).texte,
      avancementDepouillement({ etat: "rien_a_depouiller" }, MIDI).texte,
      avancementDepouillement({ etat: "en_cours", journaux: 120, total: 400 }, MIDI).texte,
      avancementDepouillement(
        { etat: "termine", quand: "2026-08-19T11:58:00Z", journaux: 400 },
        MIDI,
      ).texte,
    ];
    expect(new Set(phrases).size).toBe(phrases.length);
  });
});

describe("journauxDepouilles - une phrase, pas une case de tableau", () => {
  it("aucun journal", () => {
    expect(journauxDepouilles(0)).toBe("0 journaux dépouillés");
  });

  it("un seul journal reste au singulier", () => {
    expect(journauxDepouilles(1)).toBe("1 journal dépouillé");
  });

  it("plusieurs journaux", () => {
    expect(journauxDepouilles(400)).toBe("400 journaux dépouillés");
  });
});
