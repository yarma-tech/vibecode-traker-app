"use client";

import { useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import { lirePlanCache, plaquesSquelette, type Plaque } from "@/lib/squelette";

/** L'identifiant du repo, tiré du chemin `/repo/<id>`. Le squelette n'a pas de
 *  props (une UI de chargement n'en reçoit pas) : il lit le repo dans l'URL. */
function repoDuChemin(chemin: string): string {
  const parts = chemin.split("/").filter(Boolean);
  const i = parts.indexOf("repo");
  return i >= 0 && parts[i + 1] ? parts[i + 1] : "";
}

/**
 * Le squelette de chargement d'un repo (issue #12).
 *
 * Il reproduit la charpente de l'écran — titre, mesures, bandeau, plan — pour
 * que rien ne saute quand le vrai contenu prend sa place. Le plan, au centre,
 * garde les PROPORTIONS RÉELLES des parcelles : leur géométrie vient du cache
 * local gravé à la dernière visite (critère 2). Aucun tourniquet au milieu du
 * contenu — juste les plaques du plan, en attente de couleur.
 */
export function SqueletteRepo() {
  const chemin = usePathname();

  // Le cache vit dans le navigateur : on ne le lit qu'après le montage, sinon le
  // rendu serveur et le rendu client divergeraient. Avant, on montre le
  // squelette par défaut — déjà proportionné, déjà crédible.
  const [plaques, setPlaques] = useState<Plaque[]>(() => plaquesSquelette(null));
  useEffect(() => {
    const repoId = repoDuChemin(chemin ?? "");
    const memoire = typeof window !== "undefined" ? window.localStorage : null;
    const arrivee = setTimeout(
      () => setPlaques(plaquesSquelette(lirePlanCache(repoId, memoire))),
      0,
    );
    return () => clearTimeout(arrivee);
  }, [chemin]);

  return (
    <main className="tableau large squelette" aria-busy="true">
      <header className="entete">
        <span className="lien">← tous les repos</span>
        <span className="squelette-trait squelette-compte" />
      </header>

      <div className="squelette-trait squelette-titre" />
      <div className="squelette-trait squelette-mesures" />

      <p className="squelette-annonce" role="status">
        On recompose le plan de ce repo à partir du dernier état connu…
      </p>

      <div className="bandeau squelette-bandeau" aria-hidden="true">
        {["agents", "conflits", "jetons", "dépense"].map((cle) => (
          <div key={cle}>
            <span className="cle">{cle}</span>{" "}
            <span className="squelette-trait squelette-valeur" />
          </div>
        ))}
      </div>

      <div className="releve">
        <div className="releve-plan">
          <div className="plan squelette-plan" aria-hidden="true">
            {plaques.map((p, i) => (
              <span
                key={i}
                className="parcelle squelette-plaque"
                style={{
                  left: `${p.x}%`,
                  top: `${p.y}%`,
                  width: `${p.largeur}%`,
                  height: `${p.hauteur}%`,
                }}
              />
            ))}
          </div>
        </div>

        <div className="journal squelette-journal" aria-hidden="true">
          {Array.from({ length: 7 }).map((_, i) => (
            <span key={i} className="squelette-trait squelette-ligne" />
          ))}
        </div>
      </div>
    </main>
  );
}
