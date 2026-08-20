import { createServerClient } from "@supabase/ssr";
import { NextResponse, type NextRequest } from "next/server";

import {
  PAGE_DE_RETOUR,
  initiateurDuFlux,
  origineDeLaRequete,
  retourDAutorisation,
} from "@/lib/autorisation";
import { deposer } from "../relais";

/**
 * Retour de GitHub : on echange le code contre une session.
 *
 * Les cookies sont poses sur la reponse que l'on retourne, et pas sur le
 * magasin global. Une redirection fabriquee a part n'emporte pas les
 * ecritures du magasin : la session serait perdue en silence et l'utilisateur
 * reviendrait sur l'ecran de connexion sans comprendre pourquoi.
 *
 * Deux clients peuvent arriver ici, et un seul peut echanger (issue #63) :
 *
 * - le navigateur qui est parti - le site heberge, ou la fenetre elle-meme.
 *   Il porte son verificateur PKCE, l'echange aboutit, et la carte s'affiche ;
 * - le navigateur du systeme, a qui l'application de bureau a confie
 *   l'autorisation. Il n'a aucun verificateur : son echange serait refuse, et
 *   le code serait brule pour rien. On le lui reprend et on le depose pour la
 *   fenetre, qui a de quoi le finir (FR-072).
 */
export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const retour = retourDAutorisation(searchParams);

  // L'origine que le CLIENT a demandee, et non celle que Next croit servir :
  // servi sur `127.0.0.1:51789`, il rend `localhost:51789`, qui n'est pas la
  // meme origine. La session posee ici y serait invisible, et la fenetre de
  // l'application refuserait d'y aller (FR-070, FR-072).
  const origin = origineDeLaRequete(
    request.url,
    request.headers.get("host"),
    request.headers.get("x-forwarded-proto"),
  );

  const echec = (raison: string) =>
    NextResponse.redirect(`${origin}/?erreur=${encodeURIComponent(raison)}`);

  // Qui revient, avant quoi que ce soit d'autre : un echange tente sans
  // verificateur consommerait le code, et la fenetre n'aurait plus rien a
  // echanger.
  if (!initiateurDuFlux(request.cookies.getAll().map(({ name }) => name))) {
    deposer(retour);
    // Le navigateur du systeme n'a plus rien a faire : il le dit, et la suite
    // se passe dans la fenetre.
    return NextResponse.redirect(`${origin}${PAGE_DE_RETOUR}`);
  }

  if (retour.quoi === "refus") return echec(retour.raison);

  const reponse = NextResponse.redirect(origin);

  const supabase = createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY!,
    {
      cookies: {
        getAll() {
          return request.cookies.getAll();
        },
        setAll(cookiesToSet) {
          for (const { name, value, options } of cookiesToSet) {
            reponse.cookies.set(name, value, options);
          }
        },
      },
    },
  );

  const { error } = await supabase.auth.exchangeCodeForSession(retour.code);
  if (error) {
    // Un echange refuse n'est pas avale. Il se lit dans l'ecran de connexion
    // du navigateur, et au relais pour la fenetre : celle-ci attend un retour
    // qui vient d'arriver et de ne rien donner, et elle doit pouvoir le dire
    // plutot que de guetter jusqu'a l'echeance.
    deposer({ quoi: "refus", raison: error.message, cause: "echange_refuse" });
    return echec(error.message);
  }

  return reponse;
}
