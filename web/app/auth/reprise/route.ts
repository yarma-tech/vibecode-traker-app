import { NextResponse } from "next/server";

import { retirer } from "../relais";

/**
 * Ou la fenetre vient chercher le retour que le navigateur du systeme a
 * rapporte (FR-072, issue #63).
 *
 * Elle n'y trouve rien tant que rien n'est revenu, et c'est une reponse : la
 * fenetre attend, elle ne conclut pas a un echec. Ce qu'elle y trouve, elle le
 * retire : un code d'autorisation ne s'echange qu'une fois.
 *
 * Ce que ce chemin rend ne vaut rien pour un autre programme de la machine :
 * sans le verificateur PKCE de la fenetre, un code d'autorisation ne s'echange
 * pas. C'est la raison pour laquelle le relais peut vivre sur une origine que
 * n'importe qui peut appeler en local.
 */
export const dynamic = "force-dynamic";

export async function GET() {
  const retour = retirer();

  const corps =
    retour === null
      ? { etat: "rien" }
      : retour.quoi === "code"
        ? { etat: "code", code: retour.code }
        : { etat: "refus", raison: retour.raison };

  // Jamais de cache : une reponse « rien » gardee en reserve ferait attendre
  // la fenetre pendant que le code l'attend ici.
  return NextResponse.json(corps, { headers: { "Cache-Control": "no-store" } });
}
