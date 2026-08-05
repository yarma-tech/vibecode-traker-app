"use client";

import { useEffect, useState } from "react";
import type { Conflit, Evenement } from "./direct";

/**
 * Un glyphe en chasse fixe porte la nature de l'événement. Pas de filet
 * latéral coloré : la couleur ne dit qu'un état, jamais une décoration. Le
 * conflit, lui, teinte la ligne entière : c'est la seule alarme du produit.
 */
const GLYPHE = { read: "·", write: "✎", conflit: "⚠" } as const;

/** Une ligne du journal : un appel d'outil, ou une alarme. */
type Ligne =
  | { sorte: "read" | "write"; cle: string; quand: string; evenement: Evenement }
  | { sorte: "conflit"; cle: string; quand: string; conflit: Conflit };

/**
 * Les alarmes se rangent parmi les événements, à l'heure du dernier geste qui
 * les a provoquées. Une alarme épinglée en tête mentirait sur sa fraîcheur.
 */
function melanger(evenements: Evenement[], conflits: Conflit[]): Ligne[] {
  const lignes: Ligne[] = [
    ...evenements.map((evenement) => ({
      sorte: evenement.kind,
      cle: `e-${evenement.id}`,
      quand: evenement.occurred_at,
      evenement,
    })),
    ...conflits.map((conflit) => ({
      sorte: "conflit" as const,
      cle: `c-${conflit.repo_id}-${conflit.module_path}`,
      quand: conflit.dernier_evenement,
      conflit,
    })),
  ];

  return lignes.sort((a, b) => b.quand.localeCompare(a.quand));
}

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

/**
 * « A et C », « A, C et D ». Les lettres sont remises dans l'ordre : la base
 * rend les sessions triées par identifiant, ce qui donnerait « C et A ».
 */
function enumerer(noms: string[]): string {
  const ordonnes = [...noms].sort();
  if (ordonnes.length <= 1) return ordonnes[0] ?? "?";
  return `${ordonnes.slice(0, -1).join(", ")} et ${ordonnes[ordonnes.length - 1]}`;
}

export function Journal({
  evenements,
  conflits,
  agents,
  fenetreSecondes,
}: {
  evenements: Evenement[];
  conflits: Conflit[];
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
  const lignes = melanger(evenements, conflits);

  return (
    <aside className="journal">
      <div className="journal-titre">Journal</div>

      {lignes.length === 0 ? (
        <p className="journal-vide">
          Aucun agent n&apos;a touché ce repo depuis {minutes} min.
        </p>
      ) : (
        <ol className="journal-lignes">
          {lignes.map((ligne) => (
            <li
              key={ligne.cle}
              className={ligne.sorte === "conflit" ? "ligne alarme" : "ligne"}
            >
              <span className={`glyphe ${ligne.sorte}`} aria-hidden="true">
                {GLYPHE[ligne.sorte]}
              </span>
              <span className="quand">{monte ? heure(ligne.quand) : ""}</span>

              {ligne.sorte === "conflit" ? (
                <span className="dit">
                  <b>
                    {enumerer(
                      ligne.conflit.sessions.map((s) => agents.get(s) ?? "?"),
                    )}
                  </b>{" "}
                  écrivent tous deux dans{" "}
                  <code>{ligne.conflit.module_path || "la racine"}</code>
                </span>
              ) : (
                <span className="dit">
                  <b>{agents.get(ligne.evenement.session_id) ?? "?"}</b>{" "}
                  {ligne.sorte === "write" ? "a écrit" : "a lu"}{" "}
                  <code>{court(ligne.evenement.file_path)}</code>
                </span>
              )}
            </li>
          ))}
        </ol>
      )}

      <div className="journal-pied">
        {evenements.length} événement{evenements.length > 1 ? "s" : ""} · fenêtre
        de {minutes} min
        {conflits.length > 0 && (
          <>
            {" · "}
            <span className="chaud">
              {conflits.length} conflit{conflits.length > 1 ? "s" : ""}
            </span>
          </>
        )}
      </div>
    </aside>
  );
}
