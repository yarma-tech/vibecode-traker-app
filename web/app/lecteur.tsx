"use client";

/**
 * L'état du lecteur, par-dessus l'interface (FR-009, FR-010).
 *
 * Le lecteur est ce qui alimente la carte depuis cette machine. Qu'il se soit
 * arrêté ne se voit nulle part ailleurs : la carte continue de s'afficher,
 * alimentée par ce que la machine avait déjà envoyé, et la base ne sait rien
 * d'un verrou pris sur ce poste.
 *
 * La tranche #62 avait livré l'état et la relance, mais ils ne se voyaient que
 * sur la page d'attente de l'application, faute de pont. Le pont ouvert, ils
 * valent par-dessus n'importe quel écran.
 *
 * Hors de l'application, il n'y a pas de pont : le bandeau ne s'affiche jamais,
 * et l'interface ne prétend rien savoir du poste (FR-060).
 */

import { useCallback, useEffect, useState } from "react";
import { bandeauDuLecteur, type EtatLecteur } from "@/lib/poste";
import { pont } from "./pont";

/**
 * À quelle cadence redemander. FR-010 accorde dix secondes pour signaler un
 * lecteur qui s'arrête ; deux suffisent, et c'est une lecture en mémoire dans
 * l'application, pas un appel réseau.
 */
const CADENCE_MS = 2000;

export function Lecteur() {
  const [lecteur, setLecteur] = useState<EtatLecteur | null>(null);
  const [relance, setRelance] = useState(false);

  useEffect(() => {
    const invoquer = pont();
    if (!invoquer) return;

    let vivant = true;
    const demander = async () => {
      try {
        const rendu = await invoquer("etat_du_lecteur");
        if (vivant) setLecteur(rendu as EtatLecteur);
      } catch {
        // Un pont qui ne répond pas ne justifie pas d'alarmer par-dessus la
        // carte : on garde le dernier état connu et on redemandera.
      }
    };

    demander();
    const horloge = setInterval(demander, CADENCE_MS);
    return () => {
      vivant = false;
      clearInterval(horloge);
    };
  }, []);

  const relancer = useCallback(async () => {
    const invoquer = pont();
    if (!invoquer) return;
    setRelance(true);
    try {
      // La commande rend la main tout de suite - la lecture du jeton au
      // trousseau peut attendre longtemps. C'est la relève suivante qui dira
      // où en est le nouveau départ.
      await invoquer("relancer_le_lecteur");
    } finally {
      setRelance(false);
    }
  }, []);

  const bandeau = bandeauDuLecteur(lecteur);
  if (!bandeau.visible) return null;

  return (
    <div className="bandeau-lecteur" role="status">
      <div className="bandeau-lecteur-corps">
        <div className="bandeau-lecteur-dit">
          <b className="bandeau-lecteur-titre">{bandeau.titre}</b>{" "}
          <span className="bandeau-lecteur-suite">{bandeau.explication}</span>
        </div>
        {bandeau.relance && (
          <button
            type="button"
            className="bandeau-lecteur-relancer"
            onClick={relancer}
            disabled={relance}
          >
            {relance ? "Relance…" : "Relancer le lecteur"}
          </button>
        )}
      </div>
    </div>
  );
}
