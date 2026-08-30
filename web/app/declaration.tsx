"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import { pont } from "./pont";
import {
  annonceDeLaMachine,
  laMachineEstPrete,
  lireEtatMachine,
  type EtatMachine,
} from "@/lib/machine";

/**
 * La machine se présente (issues #65, #66, #67 - FR-017, FR-019, FR-055 à
 * FR-057, FR-080).
 *
 * POURQUOI ici, sur l'accueil, et non dans l'écran de connexion : FR-055 dit
 * « à chaque lancement suivant », pas « à chaque connexion ». Un lancement
 * ordinaire ne repasse jamais par la connexion - c'est tout l'objet de FR-014 -
 * et l'identifiant conservé ne serait alors jamais représenté. Cet écran-là,
 * lui, s'affiche à chaque ouverture.
 *
 * Ce que fait ce composant tient en un appel : présenter la session de
 * l'utilisateur au poste, qui présente à son tour l'identifiant qu'il conserve.
 * Le poste ne déclare une machine QUE s'il n'en conserve aucune, ou si celui
 * qu'il conserve ne désigne plus rien sur ce compte (FR-056) - c'est ce qui
 * fait que rouvrir l'application trois fois ne crée pas trois machines.
 *
 * Le jeton de la machine ne remonte jamais jusqu'ici : `EtatMachine` n'a pas de
 * champ où le loger. La session part, le jeton ne revient pas.
 *
 * Hors de l'application, il n'y a pas de pont : ce composant ne fait rien et
 * n'affiche rien. Le site hébergé n'a aucune machine à déclarer.
 */
export function DeclarationDeLaMachine() {
  const router = useRouter();
  const [etat, setEtat] = useState<EtatMachine | null>(null);
  const [essai, setEssai] = useState(0);

  /** Ce que le poste répond, ou `null` quand il n'y a personne à qui demander. */
  const presenter = useCallback(async (): Promise<EtatMachine | null> => {
    const appel = pont();
    if (appel === null) return null;

    const {
      data: { session },
    } = await createClient().auth.getSession();

    // Sans session, il n'y a rien à présenter : la déclaration passe par
    // l'identité de l'utilisateur, et par elle seule.
    if (!session?.access_token) return null;

    try {
      return lireEtatMachine(await appel("declarer_la_machine", { jeton: session.access_token }));
    } catch (erreur) {
      return { etat: "echec", raison: String(erreur) };
    }
  }, []);

  // `essai` est la seule raison de repasser ici : le bouton « Réessayer » de
  // FR-080, qui redemande l'accès au trousseau une fois l'autorisation
  // accordée.
  useEffect(() => {
    let vivant = true;

    void (async () => {
      const vu = await presenter();
      if (!vivant) return;
      setEtat(vu);

      if (!laMachineEstPrete(vu)) return;

      // La configuration du poste vient peut-être d'être écrite : le lecteur
      // n'avait alors rien à lire au démarrage de l'application. Sans effet
      // s'il tourne déjà.
      await pont()?.("relancer_le_lecteur");
      if (!vivant) return;

      // Et la liste des machines gagne la sienne : l'accueil se relit.
      router.refresh();
    })();

    return () => {
      vivant = false;
    };
  }, [presenter, router, essai]);

  const annonce = annonceDeLaMachine(etat);
  if (!annonce.visible) return null;

  // Les mêmes classes que le bandeau du lecteur, et à dessein : c'est le même
  // objet à l'écran - un fait du poste qui ne se voit nulle part ailleurs, qui
  // ne parle que quand ça cloche, et qui porte le geste qui le corrige. Deux
  // apparences pour la même chose se liraient comme deux choses.
  // Le rôle suit le ton : un lecteur d'écran interrompt sur « alert », et
  // interrompre pour une redéclaration réussie apprendrait à ignorer le bandeau.
  return (
    <div className="bandeau-lecteur" role={annonce.ton === "alerte" ? "alert" : "status"}>
      <div className="bandeau-lecteur-corps">
        <div className="bandeau-lecteur-dit">
          <b className="bandeau-lecteur-titre">{annonce.titre}</b>{" "}
          <span className="bandeau-lecteur-suite">{annonce.explication}</span>
        </div>
        {annonce.reessayer && (
          <button
            type="button"
            className="bandeau-lecteur-relancer"
            onClick={() => setEssai((tour) => tour + 1)}
          >
            Réessayer
          </button>
        )}
      </div>
    </div>
  );
}
