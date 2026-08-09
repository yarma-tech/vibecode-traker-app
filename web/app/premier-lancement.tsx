"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import { Appairage } from "./appairage";

/**
 * Premier lancement, aucune machine appairée (issue #12).
 *
 * Trois étapes à gauche, code d'appairage à droite. La page se remplit toute
 * seule : dès qu'une machine répond (une ligne apparaît dans `machines`), on
 * relit la page côté serveur, qui bascule alors sur l'accueil normal — sans
 * bouton « continuer », la première réponse suffit (critère 1).
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
        <h2 className="demarrage-titre">Relie ta première machine</h2>
        <p className="demarrage-pitch">
          Vibe Map allume la carte de tes codebases quand un agent y travaille&nbsp;:
          chaque dossier s&apos;éclaire à la lecture, à l&apos;écriture, au conflit.
          Il ne reste qu&apos;à relier la machine où tournent tes agents.
        </p>
      </div>

      <div className="demarrage-corps">
        <ol className="demarrage-etapes">
          <li className="demarrage-etape">
            <span className="demarrage-rang" aria-hidden="true">
              1
            </span>
            <span className="demarrage-fait">
              <b>Lance <code>vibemap</code></b> sur la machine où travaillent tes
              agents. Il observe les dossiers de sa configuration, sans jamais lire
              le contenu de tes fichiers.
            </span>
          </li>
          <li className="demarrage-etape">
            <span className="demarrage-rang" aria-hidden="true">
              2
            </span>
            <span className="demarrage-fait">
              <b>Colle la commande d&apos;appairage</b>{" "}
              ci-contre&nbsp;: elle relie cette machine à ton compte. Le code ne
              sert qu&apos;une fois.
            </span>
          </li>
          <li className="demarrage-etape">
            <span className="demarrage-rang" aria-hidden="true">
              3
            </span>
            <span className="demarrage-fait">
              <b>La carte s&apos;allume toute seule</b>, ici même, dès le premier
              signe de vie de la machine. Rien à cliquer, tu peux laisser cet
              onglet ouvert.
            </span>
          </li>
        </ol>

        <aside className="demarrage-appairage">
          <Appairage />
        </aside>
      </div>
    </div>
  );
}
