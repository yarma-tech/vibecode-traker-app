"use client";

/**
 * Page de confort pour essayer l'espace projet sur la pile Supabase locale.
 *
 * L'application ne connait qu'OAuth GitHub, qui ne repond pas contre une pile
 * locale : cette page ouvre une session par mot de passe sur le compte de
 * demonstration, et rien d'autre. Elle n'est pas suivie par git et n'a aucune
 * raison d'exister ailleurs que sur ce poste.
 */

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";

export default function ConnexionLocale() {
  const router = useRouter();
  const [etat, setEtat] = useState("Ouverture de la session...");

  useEffect(() => {
    (async () => {
      const supabase = createClient();
      const { error } = await supabase.auth.signInWithPassword({
        email: "demo@vibemap.local",
        password: "demo-vibemap-2026",
      });

      if (error) {
        setEtat(`Echec : ${error.message}`);
        return;
      }

      setEtat("Session ouverte, redirection...");
      router.push("/");
      router.refresh();
    })();
  }, [router]);

  return (
    <main style={{ padding: "3rem", fontFamily: "system-ui, sans-serif" }}>
      <p>{etat}</p>
    </main>
  );
}
