"use client";

import { useEffect, useState } from "react";
import type { Evenement } from "./direct";

/**
 * Un glyphe en chasse fixe porte la nature de l'événement. Pas de filet
 * latéral coloré : la couleur ne dit qu'un état, jamais une décoration.
 */
const GLYPHE = { read: "·", write: "✎" } as const;

function heure(instant: string): string {
  return new Date(instant).toLocaleTimeString("fr-FR", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** Le chemin complet est trop long pour la colonne : les deux derniers segments suffisent. */
function court(chemin: string): string {
  const segments = chemin.split("/");
  return segments.slice(-2).join("/");
}

export function Journal({
  evenements,
  agents,
  fenetreSecondes,
}: {
  evenements: Evenement[];
  agents: Map<string, string>;
  fenetreSecondes: number;
}) {
  // L'heure affichée vient du navigateur : rendue sur le serveur, elle
  // porterait son fuseau et non celui du lecteur.
  const [monte, setMonte] = useState(false);
  useEffect(() => {
    const arrivee = setTimeout(() => setMonte(true), 0);
    return () => clearTimeout(arrivee);
  }, []);

  const minutes = Math.round(fenetreSecondes / 60);

  return (
    <aside className="journal">
      <div className="journal-titre">Journal</div>

      {evenements.length === 0 ? (
        <p className="journal-vide">
          Aucun agent n&apos;a touché ce repo depuis {minutes} min.
        </p>
      ) : (
        <ol className="journal-lignes">
          {evenements.map((evenement) => (
            <li key={evenement.id} className="ligne">
              <span className={`glyphe ${evenement.kind}`} aria-hidden="true">
                {GLYPHE[evenement.kind]}
              </span>
              <span className="quand">{monte ? heure(evenement.occurred_at) : ""}</span>
              <span className="dit">
                <b>{agents.get(evenement.session_id) ?? "?"}</b>{" "}
                {evenement.kind === "write" ? "a écrit" : "a lu"}{" "}
                <code>{court(evenement.file_path)}</code>
              </span>
            </li>
          ))}
        </ol>
      )}

      <div className="journal-pied">
        {evenements.length} événement{evenements.length > 1 ? "s" : ""} · fenêtre
        de {minutes} min
      </div>
    </aside>
  );
}
