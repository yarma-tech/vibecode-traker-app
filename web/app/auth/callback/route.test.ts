import { describe, it, expect, beforeEach, vi } from "vitest";
import { NextRequest } from "next/server";

import { retirer } from "../relais";

// L'echangeur d'autorisation, eprouve sans Supabase et sans GitHub : ce qui se
// verifie ici, c'est QUAND on echange, et ce qui est dit quand on n'echange
// pas. L'appel au fournisseur lui-meme est remplace - il n'a rien a nous
// apprendre, et il ne repond pas contre une pile locale sans application GitHub
// declaree (issue #63).
//
// Ce chemin-ci n'est demande que par le client qui porte son verificateur : ce
// qui revient du navigateur du systeme arrive sur `/auth/callback/fenetre`, et
// s'eprouve a cote (FR-072).

const { echanger } = vi.hoisted(() => ({ echanger: vi.fn() }));

vi.mock("@supabase/ssr", () => ({
  createServerClient: () => ({ auth: { exchangeCodeForSession: echanger } }),
}));

const { GET } = await import("./route");

/** L'origine locale fixe de l'application (FR-070), telle qu'un client la demande. */
const ORIGINE_LOCALE = "http://127.0.0.1:51789";

function requete(chemin: string) {
  const entetes = new Headers();
  // Next fabrique `request.url` a partir de son propre nom d'hote et rend
  // `localhost` meme servi sur `127.0.0.1`. L'en-tete `Host` est le seul temoin
  // de l'origine que le client a demandee - celle sur laquelle la reponse doit
  // rester.
  entetes.set("host", "127.0.0.1:51789");
  return new NextRequest(new Request(`http://localhost:51789${chemin}`, { headers: entetes }));
}

/** Ou la reponse emmene. */
function ou(reponse: Response) {
  return new URL(reponse.headers.get("location")!);
}

beforeEach(() => {
  echanger.mockReset();
  echanger.mockResolvedValue({ error: null });
  // Le relais est une place unique : un test ne doit pas y laisser de quoi
  // faire passer le suivant.
  retirer();
});

describe("l'echangeur - le client qui est parti revient ici", () => {
  it("un code valide ouvre la session et emmene a l'accueil", async () => {
    const reponse = await GET(requete("/auth/callback?code=abc123"));

    expect(echanger).toHaveBeenCalledWith("abc123");
    expect(reponse.status).toBe(307);
    expect(ou(reponse).pathname).toBe("/");
    expect(ou(reponse).search).toBe("");
  });

  it("la redirection ne quitte jamais l'origine d'ou l'autorisation revient", async () => {
    // Le retour vise l'origine locale fixe de l'application ; la session qui
    // vient d'y etre posee y est liee, et la fenetre n'admet qu'elle. Repartir
    // sur `localhost` - ce que `request.url` propose - laisserait la session
    // sur place et la fenetre devant une navigation refusee (FR-070, FR-072).
    const reponse = await GET(requete("/auth/callback?code=abc123"));

    expect(ou(reponse).origin).toBe(ORIGINE_LOCALE);
  });

  it("un retour porteur d'une erreur ramene a l'ecran de connexion, avec la raison", async () => {
    const reponse = await GET(
      requete("/auth/callback?error=access_denied&error_description=The+user+has+denied"),
    );

    expect(echanger).not.toHaveBeenCalled();
    expect(ou(reponse).pathname).toBe("/");
    expect(ou(reponse).searchParams.get("erreur")).toBe("The user has denied");
  });

  it("un retour sans code ne cree pas de session", async () => {
    const reponse = await GET(requete("/auth/callback"));

    expect(echanger).not.toHaveBeenCalled();
    expect(ou(reponse).pathname).toBe("/");
    expect(ou(reponse).searchParams.get("erreur")).not.toBe(null);
    expect(ou(reponse).searchParams.get("erreur")).not.toBe("");
  });

  it("un echange refuse n'est pas avale", async () => {
    echanger.mockResolvedValue({ error: { message: "invalid request" } });

    const reponse = await GET(requete("/auth/callback?code=abc123"));

    expect(ou(reponse).searchParams.get("erreur")).toBe("invalid request");
  });

  it("et il ne coupe pas une autorisation que la fenetre attendrait", async () => {
    // Ce chemin n'est jamais celui de la fenetre : ce qu'un navigateur y rate
    // ne la regarde pas. Deposer l'echec au relais lui ferait rendre les armes
    // au milieu de son propre aller-retour.
    echanger.mockResolvedValue({ error: { message: "invalid request" } });

    await GET(requete("/auth/callback?code=abc123"));

    expect(retirer()).toBe(null);
  });
});

