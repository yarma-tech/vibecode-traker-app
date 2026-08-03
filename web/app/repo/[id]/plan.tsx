"use client";

import { useMemo, useState } from "react";
import { decouper } from "@/lib/treemap";

export type Module = {
  path: string;
  parent_path: string | null;
  depth: number;
  loc: number;
  file_count: number;
};

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

export function Plan({
  modules,
  locTotal,
}: {
  modules: Module[];
  locTotal: number;
}) {
  const [ouvert, setOuvert] = useState<string>("");

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

  const descendable = (m: Module) =>
    modules.some((autre) => autre.parent_path === m.path);

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
      {ouvert !== "" && (
        <nav className="fil">
          <button className="lien" onClick={() => setOuvert("")}>
            racine du repo
          </button>
          <span className="separateur">/</span>
          <span className="ici">{ouvert}</span>
        </nav>
      )}

      <div className="plan">
        {parcelles.map(({ donnee, x, y, largeur, hauteur }) => {
          const peutDescendre = descendable(donnee);
          const etiquette = `${nom(donnee.path)}, ${lignes(donnee.loc)}`;

          return (
            <button
              key={donnee.path}
              className="parcelle"
              style={{
                left: `${x}%`,
                top: `${y}%`,
                width: `${largeur}%`,
                height: `${hauteur}%`,
              }}
              onClick={() => peutDescendre && setOuvert(donnee.path)}
              disabled={!peutDescendre}
              title={`${donnee.path} · ${lignes(donnee.loc)} · ${donnee.file_count} fichiers`}
              aria-label={
                peutDescendre ? `${etiquette}, ouvrir` : etiquette
              }
            >
              {largeur >= LARGEUR_LISIBLE && hauteur >= HAUTEUR_LISIBLE && (
                <>
                  <span className="parcelle-nom">{nom(donnee.path)}</span>
                  <span className="parcelle-poids">{lignes(donnee.loc)}</span>
                </>
              )}
            </button>
          );
        })}
      </div>
    </>
  );
}
