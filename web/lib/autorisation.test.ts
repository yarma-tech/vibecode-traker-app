import { describe, it, expect } from "vitest";
import {
  ATTENTE_MAXIMALE_MS,
  DUREE_DU_RELAIS_MS,
  ORIGINE_LOCALE,
  attenteExpiree,
  departAutorisation,
  ecranDeConnexion,
  initiateurDuFlux,
  messageDEchec,
  origineDeLaRequete,
  relaisEncoreValable,
  repriseDeLaFenetre,
  retourDAutorisation,
  urlDeRetour,
} from "./autorisation";

// L'aller-retour d'autorisation traverse deux programmes : la fenêtre part, le
// navigateur du système revient. Ce qui suit éprouve les trois promesses qui
// tiennent ce chemin - le retour vise toujours l'origine locale fixe, un échec
// s'annonce et laisse recommencer, une session déjà ouverte n'en redemande pas
// une - sans qu'aucune n'ait besoin de GitHub pour être vraie.

const params = (requete: string) => new URLSearchParams(requete);

describe("urlDeRetour — l'origine locale fixe, dans la fenêtre (FR-071)", () => {
  it("dans l'application, le retour vaut l'origine locale fixe", () => {
    expect(urlDeRetour("http://127.0.0.1:51789", true)).toBe(
      "http://127.0.0.1:51789/auth/callback",
    );
  });

  it("et il la vaut même si la page est servie depuis une autre adresse", () => {
    // Un service d'interface déplacé, une page ouverte par `localhost` plutôt
    // que par `127.0.0.1` : le retour ne suit pas la page. Il ne peut pas :
    // seule l'origine déclarée chez le fournisseur revient (FR-070).
    for (const ailleurs of [
      "http://localhost:51789",
      "http://127.0.0.1:3000",
      "http://127.0.0.1:51790",
      "https://vibemap.example.com",
    ]) {
      expect(urlDeRetour(ailleurs, true)).toBe(`${ORIGINE_LOCALE}/auth/callback`);
    }
  });

  it("hors de l'application, le retour suit l'origine de la page", () => {
    expect(urlDeRetour("https://vibemap.example.com", false)).toBe(
      "https://vibemap.example.com/auth/callback",
    );
  });

  it("une origine à barre finale ne produit pas une double barre", () => {
    expect(urlDeRetour("https://vibemap.example.com/", false)).toBe(
      "https://vibemap.example.com/auth/callback",
    );
  });
});

describe("origineDeLaRequete — la réponse ne quitte pas l'origine du retour", () => {
  it("l'origine est celle que le client a demandée, pas celle que le serveur croit servir", () => {
    // Le cas constaté : servi sur `127.0.0.1:51789`, Next fabrique malgré tout
    // un `request.url` en `localhost`. Rediriger là-dessus quitterait l'origine
    // locale fixe - et la session qui vient d'y être posée.
    expect(
      origineDeLaRequete("http://localhost:51789/auth/callback?code=x", "127.0.0.1:51789"),
    ).toBe(ORIGINE_LOCALE);
  });

  it("le site hébergé garde son domaine et son protocole", () => {
    expect(
      origineDeLaRequete(
        "https://vibemap.example.com/auth/callback?code=x",
        "vibemap.example.com",
      ),
    ).toBe("https://vibemap.example.com");
  });

  it("derrière un terminaison TLS, le protocole transmis l'emporte", () => {
    expect(
      origineDeLaRequete(
        "http://vibemap.example.com/auth/callback",
        "vibemap.example.com",
        "https",
      ),
    ).toBe("https://vibemap.example.com");
  });

  it("un protocole transmis qui n'en est pas un ne décide de rien", () => {
    expect(
      origineDeLaRequete("https://vibemap.example.com/auth/callback", "vibemap.example.com", "ftp"),
    ).toBe("https://vibemap.example.com");
  });

  it("un hôte qui n'a pas la forme d'une autorité est écarté", () => {
    // L'en-tête vient du client : le laisser choisir une destination ferait de
    // l'échangeur une redirection ouverte.
    for (const bancal of [
      "vibemap.example.com/piege",
      "vibemap.example.com\nSet-Cookie: x=1",
      "https://piege.test",
      "piege.test:99999999",
      " ",
      "",
      null,
      undefined,
    ]) {
      expect(origineDeLaRequete("http://localhost:51789/auth/callback", bancal)).toBe(
        "http://localhost:51789",
      );
    }
  });

  it("une adresse IPv6 littérale reste lisible", () => {
    expect(origineDeLaRequete("http://localhost:51789/auth/callback", "[::1]:51789")).toBe(
      "http://[::1]:51789",
    );
  });
});

describe("departAutorisation — ce que le clic déclenche", () => {
  const origine = "http://127.0.0.1:51789";

  it("une session déjà ouverte n'en redemande pas une (FR-014)", () => {
    expect(
      departAutorisation({ sessionOuverte: true, dansLApplication: true, origineDeLaPage: origine }),
    ).toEqual({ quoi: "deja_connecte" });
  });

  it("et pas davantage hors de l'application", () => {
    expect(
      departAutorisation({
        sessionOuverte: true,
        dansLApplication: false,
        origineDeLaPage: "https://vibemap.example.com",
      }),
    ).toEqual({ quoi: "deja_connecte" });
  });

  it("sans session, dans l'application : GitHub s'ouvre dehors (FR-071)", () => {
    // La fenêtre ne navigue jamais vers github.com : la vue embarquée y est
    // refusée, et la fenêtre refuse de son côté toute origine étrangère. Un
    // départ « dans cette page » ne ferait donc strictement rien.
    expect(
      departAutorisation({
        sessionOuverte: false,
        dansLApplication: true,
        origineDeLaPage: origine,
      }),
    ).toEqual({ quoi: "au_navigateur_du_systeme", retour: `${ORIGINE_LOCALE}/auth/callback` });
  });

  it("sans session, hors de l'application : la page part elle-même", () => {
    expect(
      departAutorisation({
        sessionOuverte: false,
        dansLApplication: false,
        origineDeLaPage: "https://vibemap.example.com",
      }),
    ).toEqual({
      quoi: "dans_cette_page",
      retour: "https://vibemap.example.com/auth/callback",
    });
  });
});

describe("retourDAutorisation — un échec est annoncé, jamais avalé (FR-016)", () => {
  it("un code seul est un code", () => {
    expect(retourDAutorisation(params("code=abc123"))).toEqual({ quoi: "code", code: "abc123" });
  });

  it("un refus de GitHub porte sa raison", () => {
    expect(
      retourDAutorisation(params("error=access_denied&error_description=The+user+has+denied")),
    ).toEqual({
      quoi: "refus",
      raison: "The user has denied",
      cause: "refus_github",
    });
  });

  it("un refus sans description se rabat sur le code d'erreur", () => {
    expect(retourDAutorisation(params("error=access_denied"))).toEqual({
      quoi: "refus",
      raison: "access_denied",
      cause: "refus_github",
    });
  });

  it("un refus l'emporte sur un code, quand les deux sont là", () => {
    // Le fournisseur peut renvoyer les deux. Afficher « aucun code » ou ouvrir
    // une session sur un flux refusé serait aussi faux l'un que l'autre.
    const lu = retourDAutorisation(params("code=abc123&error_description=Bad+verification+code"));
    expect(lu).toEqual({
      quoi: "refus",
      raison: "Bad verification code",
      cause: "refus_github",
    });
  });

  it("un retour sans code n'ouvre pas de session, et le dit", () => {
    const lu = retourDAutorisation(params(""));
    expect(lu.quoi).toBe("refus");
    expect(lu).toMatchObject({ cause: "sans_code" });
    if (lu.quoi === "refus") expect(lu.raison).not.toBe("");
  });

  it("un code vide n'est pas un code", () => {
    expect(retourDAutorisation(params("code=")).quoi).toBe("refus");
    expect(retourDAutorisation(params("code=%20%20")).quoi).toBe("refus");
  });
});

describe("initiateurDuFlux — qui peut finir l'échange", () => {
  it("le client qui est parti porte son vérificateur PKCE", () => {
    expect(initiateurDuFlux(["sb-abcdefgh-auth-token-code-verifier"])).toBe(true);
  });

  it("le vérificateur d'un flux nommé compte aussi", () => {
    expect(initiateurDuFlux(["sb-abcdefgh-auth-token-flow-a1b2c3d4e5-code-verifier"])).toBe(true);
  });

  it("le navigateur du système, qui revient sans rien, n'est pas l'initiateur", () => {
    expect(initiateurDuFlux([])).toBe(false);
  });

  it("des cookies de session ne font pas un initiateur", () => {
    // Un navigateur déjà venu sur l'origine locale porte des cookies. Aucun
    // ne permet d'échanger un code : seul le vérificateur le permet.
    expect(initiateurDuFlux(["sb-abcdefgh-auth-token", "theme", "NEXT_LOCALE"])).toBe(false);
  });
});

describe("relaisEncoreValable — un code d'hier n'ouvre rien aujourd'hui", () => {
  it("un dépôt de la seconde d'avant est bon à prendre", () => {
    expect(relaisEncoreValable(1_000_000, 1_001_000)).toBe(true);
  });

  it("passé la durée du relais, il ne l'est plus", () => {
    expect(relaisEncoreValable(1_000_000, 1_000_000 + DUREE_DU_RELAIS_MS)).toBe(false);
    expect(relaisEncoreValable(1_000_000, 1_000_000 + DUREE_DU_RELAIS_MS + 1)).toBe(false);
  });

  it("un dépôt qui vient du futur ne vaut rien non plus", () => {
    // Horloge reculée pendant l'aller-retour : mieux vaut redemander une
    // autorisation que faire confiance à un âge négatif.
    expect(relaisEncoreValable(1_000_000, 999_000)).toBe(false);
  });
});

describe("repriseDeLaFenetre — ce que la fenêtre fait de la réponse du relais", () => {
  it("rien à prendre : on attend", () => {
    expect(repriseDeLaFenetre({ etat: "rien" })).toEqual({ quoi: "rien" });
  });

  it("un code déposé est repris", () => {
    expect(repriseDeLaFenetre({ etat: "code", code: "abc123" })).toEqual({
      quoi: "code",
      code: "abc123",
    });
  });

  it("un refus déposé porte sa raison jusqu'à la fenêtre", () => {
    expect(repriseDeLaFenetre({ etat: "refus", raison: "The user has denied" })).toEqual({
      quoi: "refus",
      raison: "The user has denied",
    });
  });

  it("une réponse illisible devient un refus, jamais une attente sans fin", () => {
    // Attendre sur une réponse qu'on ne sait pas lire, c'est avaler l'échec :
    // le retour est arrivé, et l'écran n'aurait ni raison ni bouton.
    for (const bancale of [{}, { etat: "code" }, { etat: "refus" }, { etat: "?" }, "texte", 42]) {
      const lu = repriseDeLaFenetre(bancale);
      expect(lu.quoi).toBe("refus");
    }
  });

  it("une absence de réponse reste une attente", () => {
    expect(repriseDeLaFenetre(null)).toEqual({ quoi: "rien" });
    expect(repriseDeLaFenetre(undefined)).toEqual({ quoi: "rien" });
  });
});

describe("attenteExpiree — le retour finit toujours par se dire", () => {
  it("on attend pendant le délai", () => {
    expect(attenteExpiree(1_000_000, 1_000_000 + ATTENTE_MAXIMALE_MS - 1)).toBe(false);
  });

  it("passé le délai, on le dit", () => {
    expect(attenteExpiree(1_000_000, 1_000_000 + ATTENTE_MAXIMALE_MS)).toBe(true);
  });
});

describe("ecranDeConnexion — un bouton, et rien d'autre (FR-012, FR-016)", () => {
  it("au repos : un seul bouton, actif, et rien à lire", () => {
    expect(ecranDeConnexion("repos", null)).toEqual({
      bouton: { texte: "Continuer avec GitHub", actif: true },
      attente: null,
      echec: null,
    });
  });

  it("pendant l'ouverture, le bouton ne se reclique pas", () => {
    const ecran = ecranDeConnexion("ouverture", null);
    expect(ecran.bouton.actif).toBe(false);
    expect(ecran.echec).toBe(null);
  });

  it("pendant l'attente, l'écran dit où se passe la suite", () => {
    const ecran = ecranDeConnexion("attente", null);
    expect(ecran.bouton.actif).toBe(false);
    expect(ecran.attente).not.toBe(null);
    expect(ecran.attente).toContain("navigateur");
  });

  it("un échec porte sa raison", () => {
    expect(ecranDeConnexion("repos", "The user has denied").echec).toBe(
      messageDEchec("The user has denied"),
    );
  });

  it("un échec laisse toujours recommencer, quelle que soit la phase", () => {
    // La seconde moitié de FR-016. Un écran qui afficherait la raison en
    // gardant son bouton éteint serait un cul-de-sac : l'utilisateur n'aurait
    // plus qu'à relancer l'application.
    for (const phase of ["repos", "ouverture", "attente"] as const) {
      const ecran = ecranDeConnexion(phase, "The user has denied");
      expect(ecran.bouton.actif).toBe(true);
      expect(ecran.bouton.texte).toBe("Continuer avec GitHub");
      expect(ecran.attente).toBe(null);
    }
  });

  it("aucune phase ne produit deux boutons ni un écran muet", () => {
    for (const phase of ["repos", "ouverture", "attente"] as const) {
      for (const echec of [null, "raison"]) {
        const ecran = ecranDeConnexion(phase, echec);
        expect(Object.keys(ecran).sort()).toEqual(["attente", "bouton", "echec"]);
        expect(ecran.bouton.texte).not.toBe("");
      }
    }
  });
});
