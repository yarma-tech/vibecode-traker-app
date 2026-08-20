"use client";

import { useEffect, useMemo, useState } from "react";
import { decouper } from "@/lib/treemap";
import { heureFigement } from "@/lib/figement";
import { ecrirePlanCache } from "@/lib/squelette";
import {
  datesEnMots,
  datesEnMotsSeparees,
  indexerTouches,
  touchesDeLaZone,
  type Touche,
} from "@/lib/touches";
import type { Etat, Worktree } from "./direct";

export type Module = {
  path: string;
  parent_path: string | null;
  depth: number;
  loc: number;
  file_count: number;
};

/** Le mot que porte chaque état, pour qui n'a pas la couleur. */
const DIT = {
  lu: "lu",
  ecrit: "écrit",
  conflit: "conflit",
} as const;

/**
 * Quand un dossier avale presque tout le repo, l'afficher seul ne dit rien :
 * on montre ses enfants à sa place. Le cas typique est le monorepo, où tout
 * vit sous `packages/`.
 */
const SEUIL_ABSORPTION = 0.7;

/**
 * En deçà, une étiquette ne tient pas : elle se tronque, déborde, et donne
 * l'impression d'un rendu cassé. La parcelle reste lisible au survol.
 */
const LARGEUR_LISIBLE = 9;
const HAUTEUR_LISIBLE = 7;

/**
 * Les deux dates disent bien plus long que le nom ou le poids, et leur seuil
 * de lisibilité à elles ne se mesure pas en pourcentage du plan : « modifié il
 * y a 2 j » occupe un nombre de PIXELS fixe, qu'un plan de 340 px de large sur
 * un téléphone n'offrirait pas là où un plan de 800 px l'offre. Ce seuil vit
 * donc en CSS, dans une requête de conteneur sur `.parcelle` (`globals.css`),
 * seul endroit qui connaisse la largeur réelle de la parcelle.
 *
 * En deçà, les dates ne se tronquent pas : elles ne paraissent pas, et restent
 * atteignables au survol (FR-063). Une date coupée en plein mot apprendrait
 * moins qu'une date absente, et ferait douter du reste de l'écran.
 */

function nom(chemin: string): string {
  // Les parcelles en « /. » portent les fichiers poses directement dans un
  // dossier, a cote de ses sous-dossiers.
  if (chemin === ".") return "fichiers à la racine";
  if (chemin.endsWith("/.")) return "fichiers";

  const dernier = chemin.split("/").pop();
  return dernier && dernier.length > 0 ? dernier : chemin;
}

function lignes(n: number): string {
  return n >= 1000
    ? `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k lignes`
    : `${n} lignes`;
}

function depuis(instant: string, maintenant: number): string {
  const secondes = Math.max(0, Math.round((maintenant - Date.parse(instant)) / 1000));
  if (secondes < 60) return `il y a ${secondes} s`;
  return `il y a ${Math.round(secondes / 60)} min`;
}

export function Plan({
  repoId,
  modules,
  locTotal,
  etats,
  touches,
  worktrees,
  fige,
  dernierBattement,
}: {
  repoId: string;
  modules: Module[];
  locTotal: number;
  etats: Etat[];
  touches: Touche[];
  worktrees: Worktree[];
  fige: boolean;
  dernierBattement: string | null;
}) {
  const [ouvert, setOuvert] = useState<string>("");

  // L'heure ne s'installe qu'après le montage : rendue sur le serveur, elle
  // ne correspondrait pas à celle du navigateur et l'hydratation s'en plaindrait.
  const [maintenant, setMaintenant] = useState<number | null>(null);
  useEffect(() => {
    const arrivee = setTimeout(() => setMaintenant(Date.now()), 0);
    const horloge = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => {
      clearTimeout(arrivee);
      clearInterval(horloge);
    };
  }, []);

  const parModule = useMemo(
    () => new Map(etats.map((etat) => [etat.module_path, etat])),
    [etats],
  );

  // Les deux dates de dernière touche, héritage des sous-dossiers compris - la
  // base l'a déjà calculé pour chaque zone (`touches_modules`, FR-043). Canal
  // strictement séparé de `parModule` : ces dates se lisent, elles n'entrent
  // pas dans la couleur de la parcelle (FR-044).
  const parTouche = useMemo(() => indexerTouches(touches), [touches]);

  const enfants = useMemo(
    () => modules.filter((m) => (m.parent_path ?? "") === ouvert),
    [modules, ouvert],
  );

  // La règle des 70 % ne s'applique qu'au premier niveau : plus bas, c'est
  // l'utilisateur qui a choisi de descendre.
  const affiches = useMemo(() => {
    if (ouvert !== "" || locTotal === 0) return enfants;

    const glouton = enfants.find((m) => m.loc / locTotal > SEUIL_ABSORPTION);
    if (!glouton) return enfants;

    const remplacants = modules.filter((m) => m.parent_path === glouton.path);
    return remplacants.length > 0
      ? [...enfants.filter((m) => m.path !== glouton.path), ...remplacants]
      : enfants;
  }, [enfants, modules, ouvert, locTotal]);

  const parcelles = useMemo(
    () =>
      decouper(
        affiches.map((m) => ({ donnee: m, valeur: m.loc })),
        { x: 0, y: 0, largeur: 100, hauteur: 100 },
      ),
    [affiches],
  );

  // On grave la géométrie du plan racine dans le cache local : à la prochaine
  // visite, le squelette de chargement repeint EXACTEMENT ces plaques, pour que
  // rien ne saute quand les données arrivent (issue #12, critère 2). Seul le
  // premier niveau vaut d'être mémorisé : c'est lui que le chargement montre.
  useEffect(() => {
    if (ouvert !== "") return;
    const memoire = typeof window !== "undefined" ? window.localStorage : null;
    ecrirePlanCache(
      repoId,
      parcelles.map(({ x, y, largeur, hauteur }) => ({ x, y, largeur, hauteur })),
      memoire,
    );
  }, [ouvert, parcelles, repoId]);

  const descendable = (m: Module) =>
    modules.some((autre) => autre.parent_path === m.path);

  // Un worktree est une copie de travail du repo ENTIER : chaque parcelle
  // affichee est donc concernee. Le canal est a part de l'activite : une
  // parcelle peut porter sa couleur d'etat ET la hachure d'un worktree.
  const sousWorktree = worktrees.length > 0;

  // Une branche peut apparaitre une seule fois, meme si git l'interdit sur deux
  // worktrees : on se protege quand meme d'un doublon a l'affichage.
  const branches = Array.from(new Set(worktrees.map((w) => w.branch)));

  if (parcelles.length === 0) {
    return (
      <div className="vide">
        <p className="vide-titre">Rien à cartographier ici.</p>
        <p className="vide-suite">
          Ce dossier ne contient aucune ligne suivie par git.
        </p>
      </div>
    );
  }

  return (
    <>
      {fige && (
        <p className="plan-gele" role="status">
          <span className="plan-gele-cle">état gelé</span>
          <span className="plan-gele-heure">à {heureFigement(dernierBattement)}</span>
          <span className="plan-gele-note">dernier état connu, la machine ne répond plus</span>
        </p>
      )}

      {sousWorktree && (
        <div
          className="worktrees-bandeau"
          aria-label={`worktrees ouverts : ${branches.join(", ")}`}
        >
          <span className="worktrees-cle">worktrees</span>
          {branches.map((branche) => (
            <span key={branche} className="worktree-badge">
              <i className="worktree-vignette" aria-hidden="true" />
              {branche}
            </span>
          ))}
        </div>
      )}

      {ouvert !== "" && (
        <nav className="fil">
          <button className="lien" onClick={() => setOuvert("")}>
            racine du repo
          </button>
          <span className="separateur">/</span>
          <span className="ici">{ouvert}</span>
        </nav>
      )}

      <div
        className={`plan${sousWorktree ? " sous-worktree" : ""}${fige ? " fige" : ""}`}
      >
        {parcelles.map(({ donnee, x, y, largeur, hauteur }) => {
          const peutDescendre = descendable(donnee);
          const etat = parModule.get(donnee.path);
          // Les deux dates de la zone, mises en mots (FR-041, FR-042). Tant que
          // l'horloge du navigateur n'a pas démarré, on ne peut juger l'âge de
          // rien : la ligne reste vide plutôt que d'annoncer un « rien de
          // récent » qui serait faux dès la seconde suivante.
          const touche = touchesDeLaZone(parTouche, donnee.path);
          const membresDates =
            maintenant !== null ? datesEnMotsSeparees(touche, maintenant) : [];
          const dates = maintenant !== null ? datesEnMots(touche, maintenant) : "";
          const dit = etat ? DIT[etat.etat] : "inactif";
          const surimpression = sousWorktree ? `, worktree ${branches.join(", ")}` : "";
          // Les dates entrent dans l'étiquette lue à voix haute comme dans
          // l'infobulle : une parcelle trop petite pour les porter en clair ne
          // doit pas les faire disparaître pour autant (FR-063).
          const etiquette =
            `${nom(donnee.path)}, ${lignes(donnee.loc)}, ${dit}${surimpression}` +
            (dates ? `, ${dates}` : "");
          const infobulle =
            `${donnee.path} · ${lignes(donnee.loc)} · ${donnee.file_count} fichiers` +
            (dates ? ` · ${dates}` : "");

          return (
            <button
              key={donnee.path}
              className={
                `parcelle${etat ? ` ${etat.etat}` : ""}${sousWorktree ? " worktree" : ""}`
              }
              style={{
                left: `${x}%`,
                top: `${y}%`,
                width: `${largeur}%`,
                height: `${hauteur}%`,
              }}
              onClick={() => peutDescendre && setOuvert(donnee.path)}
              disabled={!peutDescendre}
              title={infobulle}
              aria-label={
                peutDescendre ? `${etiquette}, ouvrir` : etiquette
              }
            >
              {largeur >= LARGEUR_LISIBLE && hauteur >= HAUTEUR_LISIBLE && (
                <>
                  <span className="parcelle-nom">{nom(donnee.path)}</span>
                  {/* La ligne de l'activité vivante garde sa place même au
                      repos (FR-077) : aucune date ne vient l'occuper, et la
                      parcelle ne se réorganise pas sous l'œil au moment où un
                      agent s'y met. */}
                  <span className="parcelle-fait">
                    {etat && (
                      <>
                        {dit}
                        {maintenant ? `, ${depuis(etat.dernier_evenement, maintenant)}` : ""}
                      </>
                    )}
                  </span>
                  <span className="parcelle-dates">
                    {/* Chaque date reste d'un bloc : quand la parcelle est trop
                        étroite pour les deux, le repli se fait ENTRE elles,
                        jamais au milieu de « il y a 20 min ». */}
                    {membresDates.map((membre, rang) => (
                      <span key={membre} className="parcelle-date">
                        {membre}
                        {rang < membresDates.length - 1 ? "," : ""}
                      </span>
                    ))}
                  </span>
                  <span className="parcelle-poids">{lignes(donnee.loc)}</span>
                </>
              )}
            </button>
          );
        })}

      </div>

      <div className="cartouche" aria-hidden="true">
        <span className="cartouche-titre">Légende</span>
        <span className="temoignage">
          <i className="temoin" /> inactif
        </span>
        <span className="temoignage">
          <i className="temoin lu" /> lu
        </span>
        <span className="temoignage">
          <i className="temoin ecrit" /> écrit
        </span>
        <span className="temoignage">
          <i className="temoin conflit" /> conflit
        </span>
        <span className="temoignage">
          <i className="temoin worktree" /> worktree
        </span>
      </div>
    </>
  );
}
