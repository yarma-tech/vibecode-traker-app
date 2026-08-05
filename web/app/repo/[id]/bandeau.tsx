"use client";

import { useEffect, useState } from "react";
import type { Conflit, Etat } from "./direct";

/**
 * Le bandeau de relevé : une ligne, séparée par des filets, comme la barre
 * d'état d'un instrument. Ni gros chiffre isolé, ni micro-courbe.
 *
 * Les jetons et la dépense viendront s'y poser avec l'issue #7.
 */
export function Bandeau({
  agents,
  conflits,
  etats,
}: {
  agents: number;
  conflits: Conflit[];
  etats: Etat[];
}) {
  const [maintenant, setMaintenant] = useState<number | null>(null);
  useEffect(() => {
    const arrivee = setTimeout(() => setMaintenant(Date.now()), 0);
    const horloge = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => {
      clearTimeout(arrivee);
      clearInterval(horloge);
    };
  }, []);

  // Les agents qui écrivent se comptent sur le plan lui-même : le bandeau et
  // les couleurs ne peuvent donc pas se contredire.
  const ecrivains = new Set(etats.flatMap((etat) => etat.sessions)).size;

  return (
    <div className="bandeau">
      <div>
        <span className="cle">agents</span> <span className="valeur">{agents}</span>{" "}
        <span className="glose">
          {ecrivains === 0
            ? "aucun en écriture"
            : `dont ${ecrivains} en écriture`}
        </span>
      </div>

      <div>
        <span className="cle">conflits</span>{" "}
        <span className={conflits.length > 0 ? "valeur chaud" : "valeur"}>
          {conflits.length}
        </span>{" "}
        <span className="glose">{ou(conflits, maintenant)}</span>
      </div>
    </div>
  );
}

function ou(conflits: Conflit[], maintenant: number | null): string {
  if (conflits.length === 0) return "personne ne se marche dessus";

  const [premier] = conflits;
  const ou = premier.module_path === "" ? "la racine" : premier.module_path;
  if (maintenant === null) return ou;

  const minutes = Math.round(
    (maintenant - Date.parse(premier.dernier_evenement)) / 60000,
  );
  const suite = conflits.length > 1 ? ` et ${conflits.length - 1} autre` : "";

  return `${ou}, ${minutes < 1 ? "à l’instant" : `il y a ${minutes} min`}${suite}`;
}
