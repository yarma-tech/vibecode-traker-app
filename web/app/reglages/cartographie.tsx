"use client";

/**
 * L'heure de la dernière cartographie (FR-030, FR-086).
 *
 * Elle ne vient PAS du pont : elle vit en base, avec le catalogue, et c'est ce
 * qui la rend lisible depuis un autre appareil et par-delà les fermetures de
 * l'application (FR-059). Elle reste donc affichée dans un navigateur
 * ordinaire, là où la liste des dossiers, elle, cède la place à une mention.
 *
 * C'est un agrégat, et l'écran l'établit comme tel : la plus récente des heures
 * de cartographie des dépôts, jamais une heure par dépôt (FR-086).
 */

import { useEffect, useState } from "react";
import { derniereCartographie } from "@/lib/poste";

export function DerniereCartographie({ heures }: { heures: ReadonlyArray<string | null> }) {
  // L'horloge avance seule : sans cela, « il y a 2 min » resterait affiché
  // indéfiniment alors que la cartographie remonte à une heure.
  const [maintenant, setMaintenant] = useState(() => Date.now());
  useEffect(() => {
    const arrivee = setTimeout(() => setMaintenant(Date.now()), 0);
    const horloge = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => {
      clearTimeout(arrivee);
      clearInterval(horloge);
    };
  }, []);

  const cartographie = derniereCartographie(heures, maintenant);

  if (cartographie.etat === "jamais") {
    return (
      <p className="cartographie">
        <span className="cartographie-quoi">Dernière cartographie</span>
        <span className="cartographie-jamais">jamais</span>
      </p>
    );
  }

  return (
    <p className="cartographie">
      <span className="cartographie-quoi">Dernière cartographie</span>
      <span className="cartographie-quand">
        {new Date(cartographie.quand).toLocaleString("fr-FR", {
          dateStyle: "short",
          timeStyle: "short",
        })}
      </span>
      <span className="cartographie-age">il y a {cartographie.age}</span>
    </p>
  );
}
