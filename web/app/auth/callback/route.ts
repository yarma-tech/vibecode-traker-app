import { createServerClient } from "@supabase/ssr";
import { NextResponse, type NextRequest } from "next/server";

import { origineDeLaRequete, retourDAutorisation } from "@/lib/autorisation";

/**
 * Retour de GitHub pour un navigateur ordinaire : on echange le code contre
 * une session.
 *
 * Les cookies sont poses sur la reponse que l'on retourne, et pas sur le
 * magasin global. Une redirection fabriquee a part n'emporte pas les
 * ecritures du magasin : la session serait perdue en silence et l'utilisateur
 * reviendrait sur l'ecran de connexion sans comprendre pourquoi.
 *
 * Ce chemin echange TOUJOURS, parce qu'il n'est demande que par le client qui
 * porte son verificateur PKCE - le site heberge, ou la fenetre si elle partait
 * elle-meme. L'autorisation confiee au navigateur du systeme revient, elle, sur
 * `/auth/callback/fenetre`, ou rien ne s'echange (FR-072, issue #63).
 *
 * POURQUOI deux chemins plutot qu'un seul qui distingue ses visiteurs : voir
 * `lib/autorisation.ts`. Le resume : un cookie n'a pas de port dans sa portee,
 * la marque qu'on cherchait ici n'appartenait donc a personne en particulier.
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
  // Un echange refuse n'est pas avale : il se lit dans l'ecran de connexion du
  // navigateur qui est parti (FR-016). Rien n'est depose au relais depuis ici -
  // la fenetre ne revient jamais sur ce chemin, et un echec de navigateur
  // couperait une autorisation qu'elle attendrait au meme moment.
  if (error) return echec(error.message);

  return reponse;
}
