"use client";

/**
 * L'heure de la dernière cartographie, et les trois silences qui peuvent la
 * remplacer (FR-030, FR-074, FR-086, FR-087).
 *
 * L'heure ne vient PAS du pont : elle vit en base, avec le catalogue, et c'est
 * ce qui la rend lisible depuis un autre appareil et par-delà les fermetures de
 * l'application (FR-059). Elle reste donc affichée dans un navigateur
 * ordinaire, là où la liste des dossiers, elle, cède la place à une mention.
 *
 * C'est un agrégat, et l'écran l'établit comme tel : la plus récente des heures
 * de cartographie des dépôts, jamais une heure par dépôt (FR-086).
 *
 * Et quand il n'y a pas d'heure, trois silences qu'il ne faut jamais
 * confondre - jamais cartographié, cartographié sans rien trouver, base
 * injoignable. La décision vit dans `lib/poste.ts`, où elle s'éprouve sans
 * navigateur ; cet écran ne fait que la mettre en mots.
 */

import { useEffect, useState } from "react";
import { etatCartographie, type FaitsCartographie } from "@/lib/poste";

export function DerniereCartographie({
  faits,
  enCours,
}: {
  faits: FaitsCartographie;
  /**
   * Une cartographie est en train de se faire. Elle se signale À CÔTÉ de
   * l'heure connue, jamais à sa place (FR-074) : remplacer l'heure de la veille
   * par « en cours » ferait perdre la seule information datée de la ligne.
   */
  enCours: boolean;
}) {
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

  const cartographie = etatCartographie(faits, maintenant);

  const signal = enCours ? (
    <span className="cartographie-en-cours">cartographie en cours</span>
  ) : null;

  if (cartographie.etat === "injoignable") {
    return (
      <p className="cartographie">
        <span className="cartographie-quoi">Dernière cartographie</span>
        <span className="cartographie-inconnue">inconnue</span>
        <span className="cartographie-pourquoi">la base ne répond pas</span>
        {signal}
      </p>
    );
  }

  if (cartographie.etat === "jamais") {
    return (
      <p className="cartographie">
        <span className="cartographie-quoi">Dernière cartographie</span>
        <span className="cartographie-jamais">jamais</span>
        {signal}
      </p>
    );
  }

  if (cartographie.etat === "sans_depot") {
    return (
      <p className="cartographie">
        <span className="cartographie-quoi">Dernière cartographie</span>
        <span className="cartographie-jamais">aucun dépôt trouvé</span>
        {signal}
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
      {signal}
    </p>
  );
}
