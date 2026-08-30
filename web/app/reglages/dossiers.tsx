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
 * Retirer un dossier (FR-032, FR-035, issue #72) est le geste inverse, et il
 * n'efface rien : la ligne s'en va, le lecteur cesse de cartographier ce
 * dossier, et ses dépôts restent au catalogue, figés à leur dernière heure de
 * cartographie. L'annonce le dit, parce que « retiré » tout seul se lit comme un
 * effacement.
 *
 * Redemander une autorisation (FR-061, issue #74) est le geste des dossiers que
 * le système ferme - `~/Documents`, `~/Desktop`, `~/Downloads`, les volumes
 * externes. Il ne paraît que sur la ligne qui en a besoin : un dossier renommé
 * ne se répare pas au sélecteur du système, et le bouton y serait un faux
 * espoir.
 */

import { useEffect, useState } from "react";
import {
  aDecouvrirDuPoste,
  echecDeLaRedemande,
  ligneDossier,
  sectionDossiers,
  silenceCartographie,
  suiteDeLAjout,
  suiteDeLaReautorisation,
  suiteDuRetrait,
  type Annonce,
  type EtatLecteur,
  type ReponseAjout,
  type ReponseDuPont,
  type ReponseReautorisation,
  type ReponseRetrait,
  type Suite,
} from "@/lib/poste";
import { pont, type CommandeLocale } from "../pont";
import { DerniereCartographie } from "./cartographie";

/**
 * Les deux commandes que ces tranches ajoutent au pont.
 *
 * Elles sont nommées ici le temps que `CommandeLocale` (`app/pont.ts`) les
 * accueille avec les autres : ce fichier est en cours de modification pour la
 * connexion GitHub, et deux tranches qui l'éditent en même temps se
 * marcheraient dessus. À la fusion, ces deux noms rejoignent l'union du pont et
 * la conversion de `demander` disparaît.
 */

/**
 * À quelle cadence redemander où en est le lecteur. La même que le bandeau :
 * c'est une lecture en mémoire dans l'application, pas un appel réseau.
 */
const CADENCE_MS = 2000;

/**
 * Le geste en cours, et sur quelle ligne. Le chemin sert à éteindre le bouton de
 * CETTE ligne-là, et non les autres : une liste dont tous les boutons
 * s'éteindraient ensemble laisserait croire à un écran figé.
 */
type Geste =
  | { quoi: "ajout" }
  | { quoi: "retrait"; chemin: string }
  | { quoi: "redemande"; chemin: string };

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

  // Un seul geste à la fois, et l'écran dit lequel. Deux raisons, et elles
  // tiennent ensemble : le sélecteur du système est modal - en proposer un
  // second pendant qu'il est ouvert ne mènerait nulle part -, et les trois
  // gestes écrivent dans la MÊME liste. Deux écritures lancées ensemble se
  // marcheraient dessus, et l'une des deux serait perdue sans que rien ne le
  // dise.
  const [geste, setGeste] = useState<Geste | null>(null);
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

  /**
   * Le fond commun des trois gestes : un appel au pont, une annonce, et la
   * liste que le poste rend. Ce qui les distingue - la commande, la phrase de
   * l'échec - est passé en paramètre ; le reste ne doit surtout pas diverger,
   * sous peine qu'un geste laisse l'écran allumé en attente d'une réponse déjà
   * arrivée.
   */
  async function demander(
    ce_geste: Geste,
    commande: CommandeLocale,
    arguments_: Record<string, unknown>,
    lire: (rendu: unknown) => Suite,
    echec: (raison: string) => Suite,
  ) {
    const invoquer = pont();
    if (!invoquer || geste) return;

    setGeste(ce_geste);
    setAnnonce(null);

    try {
      const suite = lire((await invoquer(commande, arguments_)) ?? null);
      setAnnonce(suite.annonce);
      if (suite.surveillance) setVu({ su: true, reponse: suite.surveillance });
    } catch (erreur: unknown) {
      // Un appel qui n'aboutit pas se dit exactement comme un refus du poste :
      // rien n'a changé, et la phrase le dit au même endroit.
      const raison = erreur instanceof Error ? erreur.message : String(erreur);
      setAnnonce(echec(raison).annonce);
    } finally {
      setGeste(null);
    }
  }

  // Le geste : la fenêtre demande, l'application ouvre le sélecteur du système
  // et écrit elle-même le dossier choisi. Aucun chemin ne part d'ici.
  const ajouter = () =>
    demander(
      { quoi: "ajout" },
      "ajouter_un_dossier",
      {},
      (rendu) => suiteDeLAjout(rendu as ReponseAjout),
      (raison) => suiteDeLAjout({ issue: "echoue", raison }),
    );

  // Retirer, lui, désigne une LIGNE de la liste que le poste vient de rendre -
  // jamais un endroit du disque. La borne du pont tient donc ici comme à
  // l'ajout : un chemin qui ne figure pas dans la liste n'ouvre rien et
  // n'efface rien, le poste répond qu'il ne le connaît pas.
  const retirer = (chemin: string) =>
    demander(
      { quoi: "retrait", chemin },
      "retirer_un_dossier",
      { chemin },
      (rendu) => suiteDuRetrait(rendu as ReponseRetrait),
      (raison) => suiteDuRetrait({ issue: "echoue", raison }),
    );

  // FR-061 : redemander l'autorisation rouvre le sélecteur du système sur ce
  // même dossier. C'est le seul geste qui la rende - un refus déjà donné ne se
  // redemande pas, le système ne repose plus la question.
  const redemander = (chemin: string) =>
    demander(
      { quoi: "redemande", chemin },
      "redemander_l_autorisation",
      { chemin },
      (rendu) => suiteDeLaReautorisation(rendu as ReponseReautorisation),
      (raison) => echecDeLaRedemande(chemin, raison),
    );

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
        {/* La mention couvre TOUS les faits du poste de cet écran, et pas les
            seuls dossiers : la section « Cette machine » - version, nom,
            dépouillement - s'efface avec eux hors de l'application (FR-060), et
            elle n'a pas de mention à elle. Deux mentions côte à côte diraient
            deux fois la même chose. */}
        <p className="hors-application">
          Ces réglages n&apos;existent que dans l&apos;application Vibe&nbsp;Map&nbsp;: elle seule
          voit les dossiers de cette machine, la version qu&apos;elle exécute et où en est son
          dépouillement.
        </p>
      </>
    );
  }

  // Le bouton et l'annonce précèdent tout le reste : ils valent dans les quatre
  // cas où il y a un pont, y compris quand la liste n'a pas pu être lue - c'est
  // même là qu'ajouter un dossier a le plus de chances d'être ce qu'on cherche
  // à faire.
  const bandeau = (
    <div className="dossiers-geste">
      <button type="button" className="ajouter-dossier" onClick={ajouter} disabled={geste !== null}>
        {geste?.quoi === "ajout" ? "Sélecteur ouvert…" : "Ajouter un dossier"}
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
        {bandeau}
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
        {bandeau}
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
        {bandeau}
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
      {bandeau}
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
                  fréquent - un dossier qui est lui-même un dépôt. FR-061 :
                  l'autorisation manquante s'explique ici aussi, parce qu'elle
                  est la seule dont la cause soit invisible. */}
              {ligne.explication ? (
                <span className="dossier-explication">{ligne.explication}</span>
              ) : null}
              <span className="dossier-gestes">
                {/* FR-061 : la redemande ne paraît que sur la ligne qui en a
                    besoin. Elle précède le retrait, parce que c'est le geste qui
                    répare - retirer un dossier qu'on n'a pas pu lire serait
                    renoncer avant d'avoir essayé. */}
                {ligne.redemande ? (
                  <button
                    type="button"
                    className="dossier-action dossier-redemander"
                    onClick={() => redemander(dossier.chemin)}
                    disabled={geste !== null}
                  >
                    {geste?.quoi === "redemande" && geste.chemin === dossier.chemin
                      ? "Sélecteur ouvert…"
                      : "Redemander l'autorisation"}
                  </button>
                ) : null}
                <button
                  type="button"
                  className="dossier-action"
                  onClick={() => retirer(dossier.chemin)}
                  disabled={geste !== null}
                  // Le chemin dans le nom accessible : « Retirer » répété sur
                  // chaque ligne ne dirait pas lequel, et une liste lue au
                  // clavier ou à la voix serait une suite de boutons identiques.
                  aria-label={`Retirer ${dossier.chemin} de la surveillance`}
                >
                  {geste?.quoi === "retrait" && geste.chemin === dossier.chemin
                    ? "Retrait…"
                    : "Retirer"}
                </button>
              </span>
            </li>
          );
        })}
      </ul>
    </>
  );
}
