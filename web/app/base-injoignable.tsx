"use client";

import { useTransition } from "react";
import { useRouter } from "next/navigation";

/**
 * La base est injoignable (FR-085, issue #59).
 *
 * La fenêtre s'ouvre toujours, puisqu'elle est servie depuis la machine ; ce
 * qui vient du dehors, lui, peut manquer. L'écran le dit alors, avec sa raison,
 * plutôt que de laisser croire à un compte vide ou de tourner sans fin.
 *
 * Le bouton relit la page côté serveur : le réseau revenu, la carte se peuple
 * sans qu'on ait à quitter l'application. Tant que la relecture dure, il porte
 * son propre état - un bouton muet laisserait croire au clic perdu.
 */
export function BaseInjoignable({ raison }: { raison: string }) {
  const router = useRouter();
  const [enCours, demarrer] = useTransition();

  return (
    <div className="vide injoignable" role="status">
      <p className="vide-titre">La base est injoignable.</p>
      <p className="vide-suite">
        {raison}{" "}
        Tout ce que la carte montre vient d&apos;elle&nbsp;: tant qu&apos;elle ne
        répond pas, il n&apos;y a rien à afficher.
      </p>
      <button
        type="button"
        className="bouton injoignable-reessayer"
        onClick={() => demarrer(() => router.refresh())}
        disabled={enCours}
      >
        {enCours ? "Nouvel essai…" : "Réessayer"}
      </button>
    </div>
  );
}
