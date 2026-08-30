import { NextResponse, type NextRequest } from "next/server";

import { PAGE_DE_RETOUR, origineDeLaRequete, retourDAutorisation } from "@/lib/autorisation";
import { deposer } from "../../relais";

/**
 * Retour de GitHub pour une autorisation que la FENETRE a confiee au
 * navigateur du systeme (FR-072, issue #63).
 *
 * Rien ne s'echange ici, jamais. Le verificateur PKCE est reste dans la
 * fenetre, qui est partie ; le navigateur qui revient sur ce chemin n'en a
 * aucun, et un echange tente ici brulerait le code sans rien ouvrir. Le retour
 * est donc depose au relais, ou la fenetre vient le chercher, et le navigateur
 * est renvoye sur la page qui lui dit qu'il n'a plus rien a faire.
 *
 * Un refus se depose de la meme facon : la fenetre attend un retour, et elle
 * doit pouvoir dire pourquoi il n'ouvre pas de session plutot que de guetter
 * jusqu'a l'echeance (FR-016).
 *
 * POURQUOI un chemin a part, et non une distinction faite a l'arrivee : c'est
 * le DEPART qui sait qu'il sort dans le navigateur du systeme, et lui seul.
 * Le retour ne devine rien - voir `lib/autorisation.ts`.
 */
export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);

  deposer(retourDAutorisation(searchParams));

  // Comme pour l'echangeur : l'origine que le CLIENT a demandee, et non celle
  // que Next croit servir. `localhost` n'est pas `127.0.0.1` (FR-070).
  const origin = origineDeLaRequete(
    request.url,
    request.headers.get("host"),
    request.headers.get("x-forwarded-proto"),
  );

  return NextResponse.redirect(`${origin}${PAGE_DE_RETOUR}`);
}
