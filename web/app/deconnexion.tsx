"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import { pont } from "./pont";
import { etapesDeDeconnexion } from "@/lib/session";

/**
 * Sortir depuis la fenêtre (issue #64, FR-015).
 *
 * Se déconnecter ne fait pas que ramener l'écran de connexion : cela arrête
 * aussi le lecteur du poste. POURQUOI les deux vont ensemble - plus aucune
 * session ouverte ne justifie que cette machine émette. Un lecteur laissé en
 * marche derrière la déconnexion tiendrait encore le poste et continuerait
 * d'envoyer l'activité de ce Mac au nom d'un compte dont on vient de sortir,
 * et rien à l'écran ne le dirait.
 *
 * L'ORDRE compte : le lecteur d'abord, la session ensuite. L'arrêt passe par le
 * pont de l'application, qui n'a besoin d'aucune session pour l'exécuter ; le
 * faire après aurait laissé, entre les deux, un instant où la machine émet sans
 * session. Et la session se ferme quoi qu'il arrive : une déconnexion retenue
 * par un pont muet serait le pire des deux mondes.
 *
 * Hors de l'application - le site hébergé dans un navigateur ordinaire - il n'y
 * a aucun pont, donc aucun lecteur à arrêter : la machine qui lit les dossiers
 * n'est pas celle qui affiche la page.
 */
export function Deconnexion() {
  const router = useRouter();
  const [enCours, setEnCours] = useState(false);

  async function seDeconnecter() {
    setEnCours(true);

    const appel = pont();
    const etapes = etapesDeDeconnexion(appel !== null);

    if (etapes.arreterLeLecteur) {
      try {
        await appel?.("arreter_le_lecteur");
      } catch {
        // Un pont qui ne répond pas ne doit pas retenir la sortie : la session
        // se ferme quand même. L'état du lecteur, lui, est redemandé sans
        // cesse par l'interface, qui l'affichera tel qu'il est.
      }
    }

    await createClient().auth.signOut();
    router.refresh();
    setEnCours(false);
  }

  return (
    <button className="lien" onClick={seDeconnecter} disabled={enCours}>
      {enCours ? "Déconnexion…" : "Se déconnecter"}
    </button>
  );
}
