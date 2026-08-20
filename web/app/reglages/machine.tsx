"use client";

/**
 * Ce que cette machine sait d'elle-même, sur l'écran Réglages (issues #81, #85 -
 * FR-050, FR-065, FR-068).
 *
 * Deux faits du poste, et un seul appel pour les deux : le numéro de version que
 * l'application exécute, à côté du nom de la machine (FR-068), et où en est le
 * dépouillement des trente derniers jours de journaux (FR-050). Ni l'un ni
 * l'autre ne vit en base et ne pourrait y vivre - la base ne sait rien du binaire
 * qui tourne ici, ni d'une relecture qui n'a lieu que sur ce Mac (FR-059). Ils
 * passent donc par le pont des commandes locales, comme les dossiers surveillés.
 *
 * Ouverte dans un navigateur ordinaire, où ce pont n'existe pas, la section
 * s'efface entièrement (FR-060) : la mention de la section des dossiers, juste
 * au-dessus, couvre ces faits-là comme les siens. Deux mentions côte à côte
 * diraient deux fois la même chose.
 *
 * L'avancement se redemande sans cesse, parce qu'il BOUGE : c'est ainsi qu'un
 * dépouillement se voit progresser sans recharger la page - et c'est aussi ce
 * qui rend visible qu'il REPREND au lieu de recommencer, puisqu'il repart de son
 * avancement dès la première réponse d'une nouvelle ouverture.
 */

import { useEffect, useState } from "react";
import { avancementDepouillement, sectionPoste, type ReponseContexte } from "@/lib/poste";
import { pont, type CommandeLocale } from "../pont";

/**
 * La commande que cette tranche ajoute au pont.
 *
 * Elle est nommée ici le temps que `CommandeLocale` (`app/pont.ts`) l'accueille
 * avec les autres : ce fichier est en cours de modification par une autre
 * tranche, et deux tranches qui l'éditent en même temps se marcheraient dessus.
 * À la fusion, ce nom rejoint l'union du pont et la conversion disparaît.
 */
const CONTEXTE_DU_POSTE = "contexte_du_poste" as CommandeLocale;

/**
 * À quelle cadence redemander. La même que l'état du lecteur : c'est une lecture
 * de fichier sur le poste, pas un appel réseau.
 */
const CADENCE_MS = 2000;

export function CetteMachine() {
  // Trois temps, et non deux : tant que le navigateur n'a pas répondu, on ne
  // sait pas encore s'il y a un pont. Afficher le titre avant serait le faire
  // apparaître puis disparaître dans un navigateur ordinaire.
  const [vu, setVu] = useState<{ su: boolean; reponse: ReponseContexte }>({
    su: false,
    reponse: null,
  });

  // L'horloge de l'écran, prise à chaque réponse : sans elle, « terminé il y a
  // 2 min » resterait affiché tel quel une heure plus tard.
  const [maintenant, setMaintenant] = useState(() => Date.now());

  useEffect(() => {
    const invoquer = pont();
    let vivant = true;

    // Rien ne se décide dans le corps de l'effet : la réponse arrive toujours
    // par un rappel, y compris quand il n'y a aucun pont à interroger - c'est
    // une réponse aussi, et c'est celle du navigateur ordinaire.
    const demander = () => {
      const demande: Promise<unknown> = invoquer
        ? invoquer(CONTEXTE_DU_POSTE)
        : Promise.resolve(null);

      demande
        .then((rendu) => {
          if (!vivant) return;
          setVu({ su: true, reponse: (rendu ?? null) as ReponseContexte });
          setMaintenant(Date.now());
        })
        .catch((erreur: unknown) => {
          if (!vivant) return;
          // Un pont présent et muet ne se déguise pas en navigateur ordinaire :
          // ici, quelque chose cloche, et se taire enverrait chercher ailleurs.
          setVu({
            su: true,
            reponse: {
              etat: "sans_reponse",
              raison: erreur instanceof Error ? erreur.message : String(erreur),
            },
          });
        });
    };

    demander();

    // Sans pont, personne à qui redemander : une horloge qui interrogerait le
    // vide toutes les deux secondes ne changerait jamais rien à l'écran.
    const horloge = invoquer ? setInterval(demander, CADENCE_MS) : null;
    return () => {
      vivant = false;
      if (horloge !== null) clearInterval(horloge);
    };
  }, []);

  if (!vu.su) return null;

  const section = sectionPoste(vu.reponse);

  // FR-060 : hors de l'application, ces faits n'existent pas, et la mention de
  // la section des dossiers les couvre. Jamais une ligne vide, jamais une
  // erreur.
  if (section.quoi === "hors_application") return null;

  if (section.quoi === "sans_reponse") {
    return (
      <>
        <h2 className="titre">Cette machine</h2>
        <p className="echec" role="alert">
          Ce que cette machine sait d&apos;elle-même n&apos;a pas pu être lu&nbsp;: {section.raison}
        </p>
      </>
    );
  }

  const { version, machine, depouillement } = section.faits;
  const avancement = avancementDepouillement(depouillement, maintenant);

  return (
    <>
      <h2 className="titre">Cette machine</h2>

      <p className="etape">
        Ce que Vibe&nbsp;Map exécute sur ce Mac, et où en est la relecture des trente derniers
        jours de journaux d&apos;agents - celle qui rend leurs dernières dates aux zones de la
        carte, sans rien allumer.
      </p>

      <p className="poste-fait">
        <span className="poste-quoi">Machine</span>
        {/* Le nom manque quand le poste ne sait pas le dire : mieux vaut se
            taire qu'afficher un nom inventé à la place de celui du Mac. La
            version, elle, ne doit rien à la configuration et reste là. */}
        {machine ? <span className="poste-valeur">{machine}</span> : null}
        <span className="poste-precision">version {version}</span>
      </p>

      <p className="poste-fait">
        <span className="poste-quoi">Dépouillement du passé</span>
        <span
          className={
            avancement.etat === "en_cours" ? "poste-valeur poste-en-cours" : "poste-valeur"
          }
        >
          {avancement.texte}
        </span>
        {/* L'heure exacte accompagne le terme, elle ne le remplace pas : « il y a
            2 min » se lit d'un coup d'oeil, la date répond à « quand, au
            juste ». */}
        {avancement.etat === "termine" && avancement.quand !== null ? (
          <span className="poste-precision">
            {new Date(avancement.quand).toLocaleString("fr-FR", {
              dateStyle: "short",
              timeStyle: "short",
            })}
          </span>
        ) : null}
      </p>
    </>
  );
}
