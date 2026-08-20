"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import { ETAPES } from "@/lib/premier-lancement";
import { Telechargement } from "./telechargement";

/**
 * Premier lancement, aucune machine déclarée (issue #12, réécrit par #83).
 *
 * Trois étapes à gauche, le téléchargement à droite. Aucun code, aucune
 * commande de terminal (FR-023, FR-024) : le seul chemin qui relie une machine
 * passe désormais par l'application de bureau (FR-025), et le lien vers ses
 * publications est ici, à un clic (FR-058).
 *
 * La page se remplit toute seule : dès qu'une machine répond (une ligne
 * apparaît dans `machines`), on relit la page côté serveur, qui bascule alors
 * sur l'accueil normal - sans bouton « continuer », la première réponse suffit
 * (critère 1).
 *
 * En mode démonstration (`?demo=onboarding`), on n'écoute rien : l'écran reste
 * affiché pour la revue, même quand des machines existent déjà.
 */
export function PremierLancement({ demo = false }: { demo?: boolean }) {
  const router = useRouter();

  useEffect(() => {
    if (demo) return;
    const supabase = createClient();
    const canal = supabase
      .channel("premier-lancement")
      .on(
        "postgres_changes",
        { event: "INSERT", schema: "public", table: "machines" },
        () => router.refresh(),
      )
      .subscribe();

    return () => {
      supabase.removeChannel(canal);
    };
  }, [demo, router]);

  return (
    <div className="demarrage">
      <div className="demarrage-tete">
        <h2 className="demarrage-titre">Installe l&apos;application de bureau</h2>
        <p className="demarrage-pitch">
          Vibe Map allume la carte de tes codebases quand un agent y travaille&nbsp;:
          chaque dossier s&apos;éclaire à la lecture, à l&apos;écriture, au conflit.
          L&apos;application observe la machine où tournent tes agents, et cette
          page montre ce qu&apos;elle voit.
        </p>
      </div>

      <div className="demarrage-corps">
        <ol className="demarrage-etapes">
          {ETAPES.map((etape, rang) => (
            <li className="demarrage-etape" key={etape.geste}>
              <span className="demarrage-rang" aria-hidden="true">
                {rang + 1}
              </span>
              <span className="demarrage-fait">
                <b>{etape.geste}</b> {etape.suite}
              </span>
            </li>
          ))}
        </ol>

        <aside className="demarrage-telechargement">
          <Telechargement />
        </aside>
      </div>
    </div>
  );
}
