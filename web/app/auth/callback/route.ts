import { createServerClient } from "@supabase/ssr";
import { NextResponse, type NextRequest } from "next/server";

/**
 * Retour de GitHub : on echange le code contre une session.
 *
 * Les cookies sont poses sur la reponse que l'on retourne, et pas sur le
 * magasin global. Une redirection fabriquee a part n'emporte pas les
 * ecritures du magasin : la session serait perdue en silence et l'utilisateur
 * reviendrait sur l'ecran de connexion sans comprendre pourquoi.
 */
export async function GET(request: NextRequest) {
  const { searchParams, origin } = new URL(request.url);
  const code = searchParams.get("code");
  const refusGithub = searchParams.get("error_description");

  const echec = (raison: string) =>
    NextResponse.redirect(`${origin}/?erreur=${encodeURIComponent(raison)}`);

  if (refusGithub) return echec(refusGithub);
  if (!code) return echec("GitHub n'a renvoye aucun code d'autorisation.");

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

  const { error } = await supabase.auth.exchangeCodeForSession(code);
  if (error) return echec(error.message);

  return reponse;
}
