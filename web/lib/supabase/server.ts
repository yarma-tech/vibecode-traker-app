import { createServerClient } from "@supabase/ssr";
import { cookies } from "next/headers";

/** Client Supabase cote serveur. Lit et rafraichit la session dans les cookies. */
export async function createClient() {
  const cookieStore = await cookies();

  return createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY!,
    {
      cookies: {
        getAll() {
          return cookieStore.getAll();
        },
        setAll(cookiesToSet) {
          try {
            for (const { name, value, options } of cookiesToSet) {
              cookieStore.set(name, value, options);
            }
          } catch {
            // Appele depuis un composant serveur : le middleware rafraichit
            // deja la session, on peut ignorer sans risque.
          }
        },
      },
    },
  );
}
