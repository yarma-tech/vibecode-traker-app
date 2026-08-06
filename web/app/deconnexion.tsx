"use client";

import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";

export function Deconnexion() {
  const router = useRouter();

  return (
    <button
      className="lien"
      onClick={async () => {
        await createClient().auth.signOut();
        router.refresh();
      }}
    >
      Se déconnecter
    </button>
  );
}
