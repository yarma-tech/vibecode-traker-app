"use client";

import { useState } from "react";
import { createClient } from "@/lib/supabase/client";

export function Connexion({ erreur }: { erreur?: string }) {
  const [enCours, setEnCours] = useState(false);
  const [echec, setEchec] = useState<string | null>(erreur ?? null);

  async function seConnecter() {
    setEnCours(true);
    setEchec(null);

    const supabase = createClient();
    const { error } = await supabase.auth.signInWithOAuth({
      provider: "github",
      options: { redirectTo: `${window.location.origin}/auth/callback` },
    });

    if (error) {
      setEchec(error.message);
      setEnCours(false);
    }
  }

  return (
    <div className="accueil">
      <h1>Vibe Map</h1>
      <p className="pitch">
        Une carte de tes repos qui s&apos;allume dès qu&apos;un agent se met au
        travail. Ton code reste chez toi : seules les couleurs voyagent.
      </p>

      <button className="bouton" onClick={seConnecter} disabled={enCours}>
        {enCours ? "Ouverture de GitHub…" : "Continuer avec GitHub"}
      </button>

      {echec && (
        <p className="echec" role="alert">
          La connexion a échoué : {echec}
        </p>
      )}
    </div>
  );
}
