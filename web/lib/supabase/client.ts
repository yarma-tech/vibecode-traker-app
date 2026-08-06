import { createBrowserClient } from "@supabase/ssr";

/** Client Supabase cote navigateur. La session vit dans les cookies. */
export function createClient() {
  return createBrowserClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY!,
  );
}
