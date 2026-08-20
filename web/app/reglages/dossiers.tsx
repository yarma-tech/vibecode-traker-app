"use client";

/**
 * Les dossiers surveillés, sur l'écran Réglages (FR-027 à FR-029, issue #68).
 *
 * Tout ce que cette section montre du POSTE vient du pont des commandes
 * locales, et jamais de la base (FR-059) : les chemins surveillés, ce qu'on y a
 * trouvé, et ce qui empêche d'y regarder. Rien de cela n'existe ailleurs que
 * sur cette machine, et rien n'en sort.
 *
 * Ouvert dans un navigateur ordinaire, où ce pont n'existe pas, cette part se
 * remplace par une mention (FR-060). L'heure de la dernière cartographie, elle,
 * vient de la base : elle continue de s'afficher là-bas, et c'est pourquoi elle
 * est rendue AVANT toute bascule.
 *
 * Le bouton d'ajout (FR-031, issue #70) est le geste que cette section
 * attendait : le sélecteur du système s'ouvre dans l'application, et le dossier
 * choisi est surveillé sans que personne ait eu à ouvrir un fichier (FR-036).
 * L'écran ne désigne aucun chemin - il demande un geste, et c'est
 * l'application qui va poser la question à l'utilisateur. Un dossier déjà
 * surveillé est refusé, et le refus nomme lequel s'y oppose (FR-037, FR-075).
 *
 * Retirer un dossier appartient à une tranche suivante.
 */

import { useEffect, useState } from "react";
import {
  aDecouvrirDuPoste,
  ligneDossier,
  sectionDossiers,
  silenceCartographie,
  suiteDeLAjout,
  type Annonce,
  type EtatLecteur,
  type ReponseAjout,
  type ReponseDuPont,
} from "@/lib/poste";
import { pont } from "../pont";
import { DerniereCartographie } from "./cartographie";

/**
 * À quelle cadence redemander où en est le lecteur. La même que le bandeau :
 * c'est une lecture en mémoire dans l'application, pas un appel réseau.
 */
const CADENCE_MS = 2000;

export function DossiersSurveilles({
  heures,
  baseInjoignable,
}: {
  /** Les heures de cartographie des dépôts, lues en base (FR-059, FR-086). */
  heures: ReadonlyArray<string | null>;
  /** La base n'a pas répondu : on ne sait rien de l'heure (FR-085). */
  baseInjoignable: boolean;
}) {
  // Trois temps, et non deux : tant que le navigateur n'a pas répondu, on ne
  // sait pas encore s'il y a un pont. Conclure avant serait afficher la mention
  // de FR-060 pendant un éclair à l'intérieur même de l'application. Les deux
  // vont ensemble, dans un seul état : un rendu où l'on saurait déjà sans avoir
  // la réponse n'existe pas.
  const [vu, setVu] = useState<{ su: boolean; reponse: ReponseDuPont }>({
    su: false,
    reponse: null,
  });

  // Le sélecteur du système est modal : tant qu'il est ouvert, le bouton ne
  // doit pas en proposer un second, et il doit dire pourquoi il ne répond plus.
  const [selecteurOuvert, setSelecteurOuvert] = useState(false);
  const [annonce, setAnnonce] = useState<Annonce | null>(null);

  // Le lecteur cartographie dès son départ, avant sa première boucle : tant
  // qu'il démarre, la cartographie de ce lancement n'a pas encore abouti. C'est
  // le seul fait local qui dise « une cartographie est en train de se faire »,
  // et il vaut aussi après un ajout, qui fait repartir le lecteur (FR-034).
  const [lecteur, setLecteur] = useState<EtatLecteur | null>(null);

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

  useEffect(() => {
    const invoquer = pont();
    if (!invoquer) return;

    let vivant = true;
    const demander = async () => {
      try {
        const rendu = await invoquer("etat_du_lecteur");
        if (vivant) setLecteur(rendu as EtatLecteur);
      } catch {
        // Un pont muet ne justifie pas d'annoncer une cartographie en cours :
        // on garde le dernier état connu et on redemandera.
      }
    };

    demander();
    const horloge = setInterval(demander, CADENCE_MS);
    return () => {
      vivant = false;
      clearInterval(horloge);
    };
  }, []);

  // Le geste : la fenêtre demande, l'application ouvre le sélecteur du système
  // et écrit elle-même le dossier choisi. Aucun chemin ne part d'ici.
  async function ajouter() {
    const invoquer = pont();
    if (!invoquer || selecteurOuvert) return;

    setSelecteurOuvert(true);
    setAnnonce(null);

    try {
      const rendu = await invoquer("ajouter_un_dossier");
      const suite = suiteDeLAjout((rendu ?? null) as ReponseAjout);
      setAnnonce(suite.annonce);
      if (suite.surveillance) setVu({ su: true, reponse: suite.surveillance });
    } catch (erreur: unknown) {
      // Un appel qui n'aboutit pas se dit exactement comme un refus du poste :
      // le dossier n'est pas surveillé, et la phrase le dit au même endroit.
      const raison = erreur instanceof Error ? erreur.message : String(erreur);
      setAnnonce(suiteDeLAjout({ issue: "echoue", raison }).annonce);
    } finally {
      setSelecteurOuvert(false);
    }
  }

  const faits = {
    heures,
    baseInjoignable,
    aDecouvrir: aDecouvrirDuPoste(vu.reponse),
  };
  const silence = silenceCartographie(faits);

  // L'heure vient de la base, jamais du pont : elle s'affiche donc toujours, y
  // compris hors de l'application et avant même de savoir s'il y a un pont.
  const cartographie = (
    <DerniereCartographie faits={faits} enCours={lecteur?.etat === "en_demarrage"} />
  );

  if (!vu.su) {
    return (
      <>
        {cartographie}
        <p className="dossiers-attente">Lecture des dossiers surveillés…</p>
      </>
    );
  }

  const section = sectionDossiers(vu.reponse);

  if (section.quoi === "hors_application") {
    return (
      <>
        {cartographie}
        <p className="hors-application">
          Ces réglages n&apos;existent que dans l&apos;application Vibe&nbsp;Map&nbsp;: elle seule
          voit les dossiers de cette machine.
        </p>
      </>
    );
  }

  // Le bouton et l'annonce précèdent tout le reste : ils valent dans les quatre
  // cas où il y a un pont, y compris quand la liste n'a pas pu être lue - c'est
  // même là qu'ajouter un dossier a le plus de chances d'être ce qu'on cherche
  // à faire.
  const geste = (
    <div className="dossiers-geste">
      <button
        type="button"
        className="ajouter-dossier"
        onClick={ajouter}
        disabled={selecteurOuvert}
      >
        {selecteurOuvert ? "Sélecteur ouvert…" : "Ajouter un dossier"}
      </button>
      {annonce ? (
        <p
          className={annonce.ton === "echec" ? "annonce-ajout annonce-echec" : "annonce-ajout"}
          role={annonce.ton === "echec" ? "alert" : "status"}
        >
          {annonce.texte}
        </p>
      ) : null}
    </div>
  );

  if (section.quoi === "sans_reponse") {
    return (
      <>
        {cartographie}
        {geste}
        <p className="echec" role="alert">
          Les dossiers surveillés n&apos;ont pas pu être lus&nbsp;: {section.raison}
        </p>
      </>
    );
  }

  if (section.quoi === "sans_configuration") {
    return (
      <>
        {cartographie}
        {geste}
        <p className="echec" role="alert">
          Impossible de savoir ce qui est surveillé&nbsp;: {section.raison}
        </p>
      </>
    );
  }

  if (section.quoi === "aucun") {
    return (
      <>
        {cartographie}
        {geste}
        <div className="vide">
          <p className="vide-titre">Aucun dossier surveillé.</p>
          <p className="vide-suite">
            Rien n&apos;est cartographié sur cette machine tant qu&apos;aucun dossier n&apos;est
            surveillé&nbsp;: ajoutez-en un ci-dessus.
          </p>
        </div>
      </>
    );
  }

  return (
    <>
      {cartographie}
      {geste}
      <ul className="dossiers">
        {section.dossiers.map((dossier) => {
          const ligne = ligneDossier(dossier, silence);
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
              <span className="dossier-ligne">
                <span className="dossier-chemin" title={dossier.emplacement}>
                  {dossier.chemin}
                </span>
                {ligne.signal ? (
                  <span className="dossier-signal">{ligne.signal}</span>
                ) : (
                  <span className="dossier-compte">{ligne.compte}</span>
                )}
              </span>
              {/* FR-076 : un compte nul ne se laisse pas sans explication. Ce
                  qu'on a cherché, et le geste qui corrige le cas le plus
                  fréquent - un dossier qui est lui-même un dépôt. */}
              {ligne.explication ? (
                <span className="dossier-explication">{ligne.explication}</span>
              ) : null}
            </li>
          );
        })}
      </ul>
    </>
  );
}
