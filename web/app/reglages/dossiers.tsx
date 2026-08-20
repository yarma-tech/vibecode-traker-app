"use client";

/**
 * Les dossiers surveillés, sur l'écran Réglages (FR-027 à FR-029, issue #68).
 *
 * Tout ce que cette section montre vient du POSTE, par le pont des commandes
 * locales, et jamais de la base (FR-059) : les chemins surveillés, ce qu'on y a
 * trouvé, et ce qui empêche d'y regarder. Rien de cela n'existe ailleurs que
 * sur cette machine, et rien n'en sort.
 *
 * Ouvert dans un navigateur ordinaire, où ce pont n'existe pas, la section se
 * remplace par une mention (FR-060). Le reste de l'écran - les comptes, qui
 * viennent de la base - n'en sait rien et continue de s'afficher.
 *
 * Voir, seulement : ajouter et retirer un dossier appartiennent aux tranches
 * suivantes.
 */

import { useEffect, useState } from "react";
import { ligneDossier, sectionDossiers, type ReponseDuPont } from "@/lib/poste";
import { pont } from "../pont";

export function DossiersSurveilles() {
  // Trois temps, et non deux : tant que le navigateur n'a pas répondu, on ne
  // sait pas encore s'il y a un pont. Conclure avant serait afficher la mention
  // de FR-060 pendant un éclair à l'intérieur même de l'application. Les deux
  // vont ensemble, dans un seul état : un rendu où l'on saurait déjà sans avoir
  // la réponse n'existe pas.
  const [vu, setVu] = useState<{ su: boolean; reponse: ReponseDuPont }>({
    su: false,
    reponse: null,
  });

  useEffect(() => {
    let vivant = true;
    const invoquer = pont();

    // Rien ne se décide dans le corps de l'effet : la réponse arrive toujours
    // par un rappel, y compris quand il n'y a aucun pont à interroger - c'est
    // une réponse aussi, et c'est celle du navigateur ordinaire.
    const demande: Promise<unknown> = invoquer
      ? invoquer("dossiers_surveilles")
      : Promise.resolve(null);

    demande
      .then((rendu) => {
        if (vivant) setVu({ su: true, reponse: (rendu ?? null) as ReponseDuPont });
      })
      .catch((erreur: unknown) => {
        if (!vivant) return;
        setVu({
          su: true,
          reponse: {
            etat: "sans_reponse",
            raison: erreur instanceof Error ? erreur.message : String(erreur),
          },
        });
      });

    return () => {
      vivant = false;
    };
  }, []);

  if (!vu.su) {
    return <p className="dossiers-attente">Lecture des dossiers surveillés…</p>;
  }

  const section = sectionDossiers(vu.reponse);

  if (section.quoi === "hors_application") {
    return (
      <p className="hors-application">
        Ces réglages n&apos;existent que dans l&apos;application Vibe&nbsp;Map&nbsp;: elle seule
        voit les dossiers de cette machine.
      </p>
    );
  }

  if (section.quoi === "sans_reponse") {
    return (
      <p className="echec" role="alert">
        Les dossiers surveillés n&apos;ont pas pu être lus&nbsp;: {section.raison}
      </p>
    );
  }

  if (section.quoi === "sans_configuration") {
    return (
      <p className="echec" role="alert">
        Impossible de savoir ce qui est surveillé&nbsp;: {section.raison}
      </p>
    );
  }

  if (section.quoi === "aucun") {
    return (
      <div className="vide">
        <p className="vide-titre">Aucun dossier surveillé.</p>
        <p className="vide-suite">
          Rien n&apos;est cartographié sur cette machine tant qu&apos;aucun dossier n&apos;est
          surveillé.
        </p>
      </div>
    );
  }

  return (
    <ul className="dossiers">
      {section.dossiers.map((dossier) => {
        const ligne = ligneDossier(dossier);
        return (
          <li
            key={dossier.emplacement}
            className={[
              "dossier",
              ligne.signal ? "dossier-illisible" : "",
              // Le chemin barré est réservé à ce qui n'est plus là : un dossier
              // dont l'accès est refusé existe toujours, et le barrer dirait le
              // contraire de ce qu'il faut aller corriger.
              dossier.lisibilite === "introuvable" ? "dossier-absent" : "",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            <span className="dossier-chemin" title={dossier.emplacement}>
              {dossier.chemin}
            </span>
            {ligne.signal ? (
              <span className="dossier-signal">{ligne.signal}</span>
            ) : (
              <span className="dossier-compte">{ligne.compte}</span>
            )}
          </li>
        );
      })}
    </ul>
  );
}
