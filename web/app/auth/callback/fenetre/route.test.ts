import { describe, it, expect, beforeEach, vi } from "vitest";
import { NextRequest } from "next/server";

import { retirer } from "../../relais";

// Le retour de l'autorisation que la FENETRE a confiee au navigateur du systeme
// (FR-072, issue #63). Ce qui se verifie ici tient en une phrase : rien ne
// s'echange sur ce chemin, quoi que porte le client qui l'emprunte.
//
// L'echangeur est tout de meme remplace, pour qu'un appel qui ne devrait pas
// avoir lieu se voie au lieu de partir vers Supabase.

const { echanger } = vi.hoisted(() => ({ echanger: vi.fn() }));

vi.mock("@supabase/ssr", () => ({
  createServerClient: () => ({ auth: { exchangeCodeForSession: echanger } }),
}));

const { GET } = await import("./route");

/** L'origine locale fixe de l'application (FR-070), telle qu'un client la demande. */
const ORIGINE_LOCALE = "http://127.0.0.1:51789";

function requete(chemin: string, cookies: Record<string, string> = {}) {
  const entetes = new Headers();
  const biscuits = Object.entries(cookies)
    .map(([nom, valeur]) => `${nom}=${valeur}`)
    .join("; ");
  if (biscuits !== "") entetes.set("cookie", biscuits);
  // `Host` est le seul temoin de l'origine demandee : Next rend `localhost`
  // meme servi sur `127.0.0.1`, et la fenetre n'admet que l'origine fixe.
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

describe("le navigateur du systeme - il revient sans rien pouvoir echanger (FR-072)", () => {
  it("le code lui est repris et depose pour la fenetre, sans etre echange ici", async () => {
    const reponse = await GET(requete("/auth/callback/fenetre?code=abc123"));

    // Le point qui tient tout : un echange tente ici serait refuse - le
    // verificateur est dans la fenetre - et brulerait le code. La fenetre
    // n'aurait plus rien a echanger.
    expect(echanger).not.toHaveBeenCalled();
    expect(ou(reponse).pathname).toBe("/auth/retour");
    expect(retirer()).toEqual({ quoi: "code", code: "abc123" });
  });

  it("un refus lui est repris de la meme facon, avec sa raison", async () => {
    const reponse = await GET(
      requete("/auth/callback/fenetre?error=access_denied&error_description=The+user+has+denied"),
    );

    expect(echanger).not.toHaveBeenCalled();
    expect(ou(reponse).pathname).toBe("/auth/retour");
    expect(retirer()).toEqual({
      quoi: "refus",
      raison: "The user has denied",
      cause: "refus_github",
    });
  });

  it("un retour vide se depose aussi : la fenetre doit cesser d'attendre", async () => {
    await GET(requete("/auth/callback/fenetre"));

    expect(echanger).not.toHaveBeenCalled();
    expect(retirer()).toMatchObject({ quoi: "refus", cause: "sans_code" });
  });

  it("la reponse ne quitte pas l'origine d'ou l'autorisation revient", async () => {
    const reponse = await GET(requete("/auth/callback/fenetre?code=abc123"));

    expect(ou(reponse).origin).toBe(ORIGINE_LOCALE);
  });

  it("un verificateur qui traine dans le navigateur ne lui fait rien echanger", async () => {
    // LA REGRESSION. Un cookie n'a pas de port dans sa portee : le site servi
    // en developpement sur `127.0.0.1:3000` pose ses verificateurs sur le meme
    // hote que l'origine locale fixe, et `@supabase/ssr` laisse en plus un
    // cookie d'index qui survit aux flux termines. Un navigateur qui a un jour
    // ouvert ce site revient donc marque - et l'echange tente au vu de cette
    // marque etait refuse : « PKCE code verifier not found in storage ». La
    // fenetre recevait cet echec au lieu de son code.
    const reponse = await GET(
      requete("/auth/callback/fenetre?code=abc123", {
        "sb-127-auth-token-flow-a1b2c3d4e5-code-verifier": "d-un-autre-flux",
        "sb-127-auth-token-flows-code-verifier": "index",
      }),
    );

    expect(echanger).not.toHaveBeenCalled();
    expect(ou(reponse).pathname).toBe("/auth/retour");
    expect(retirer()).toEqual({ quoi: "code", code: "abc123" });
  });
});

describe("le relais ne se lit qu'une fois", () => {
  it("un code repris n'est plus la au passage suivant", async () => {
    await GET(requete("/auth/callback/fenetre?code=abc123"));

    expect(retirer()).toEqual({ quoi: "code", code: "abc123" });
    expect(retirer()).toBe(null);
  });
});
