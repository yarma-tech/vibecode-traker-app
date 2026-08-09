"use client";

import { useEffect, useState } from "react";
import { heureFigement } from "@/lib/figement";
import type { Conflit, Etat, Releve } from "./direct";

/**
 * Le bandeau de relevé : une ligne, séparée par des filets, comme la barre
 * d'état d'un instrument. Ni gros chiffre isolé, ni micro-courbe.
 *
 * Quatre cellules : les agents et les conflits, live sur la fenêtre glissante ;
 * les jetons et la dépense, mémoire longue du repo qui survit à la purge.
 *
 * Quand la machine est figée, chaque valeur porte l'heure de son dernier relevé
 * (celle du dernier battement) : le bandeau ne prétend plus au direct, il date
 * ce qu'il montre.
 */
export function Bandeau({
  agents,
  conflits,
  etats,
  releve,
  fige,
  dernierBattement,
}: {
  agents: number;
  conflits: Conflit[];
  etats: Etat[];
  releve: Releve | null;
  fige: boolean;
  dernierBattement: string | null;
}) {
  const [maintenant, setMaintenant] = useState<number | null>(null);
  useEffect(() => {
    const arrivee = setTimeout(() => setMaintenant(Date.now()), 0);
    const horloge = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => {
      clearTimeout(arrivee);
      clearInterval(horloge);
    };
  }, []);

  // Les agents qui écrivent se comptent sur le plan lui-même : le bandeau et
  // les couleurs ne peuvent donc pas se contredire.
  const ecrivains = new Set(etats.flatMap((etat) => etat.sessions)).size;

  // L'heure du dernier relevé, apposée à chaque valeur quand l'écran est figé.
  const heure = fige ? heureFigement(dernierBattement) : null;
  const relevé = (heure ? <span className="heure-releve">{heure}</span> : null);

  return (
    <div className={fige ? "bandeau fige" : "bandeau"}>
      <div>
        <span className="cle">agents</span> <span className="valeur">{agents}</span>{" "}
        <span className="glose">
          {ecrivains === 0
            ? "aucun en écriture"
            : `dont ${ecrivains} en écriture`}
        </span>
        {relevé}
      </div>

      <div>
        <span className="cle">conflits</span>{" "}
        <span className={conflits.length > 0 ? "valeur chaud" : "valeur"}>
          {conflits.length}
        </span>{" "}
        <span className="glose">{ou(conflits, maintenant)}</span>
        {relevé}
      </div>

      <div>
        <span className="cle">jetons</span>{" "}
        <span className="valeur">{compact(totalJetons(releve))}</span>{" "}
        <span className="glose">{gloseJetons(releve)}</span>
        {relevé}
      </div>

      <div>
        <span className="cle">dépense</span>{" "}
        <span className="valeur">
          {releve && releve.cout_usd !== null ? dollars(releve.cout_usd) : "—"}
        </span>{" "}
        <span className="glose">{gloseDepense(releve)}</span>
        {relevé}
      </div>
    </div>
  );
}

/** Tous les jetons de la session confondus : entrée, sortie et cache. */
function totalJetons(releve: Releve | null): number {
  if (!releve) return 0;
  return (
    releve.input_tokens +
    releve.output_tokens +
    releve.cache_read_tokens +
    releve.cache_creation_tokens
  );
}

function gloseJetons(releve: Releve | null): string {
  if (!releve || totalJetons(releve) === 0) return "rien encore";
  const cache = releve.cache_read_tokens + releve.cache_creation_tokens;
  return cache > 0 ? `dont ${compact(cache)} de cache` : "sans cache";
}

function gloseDepense(releve: Releve | null): string {
  if (!releve || releve.sessions === 0) return "aucune session";
  // Aucune session tarifée : le coût est absent, pas nul. On le dit.
  if (releve.sessions_tarifees === 0) return "modèle inconnu";

  const pluriel = releve.sessions > 1 ? "s" : "";
  const base = `sur ${releve.sessions} session${pluriel}`;
  const sansPrix = releve.sessions - releve.sessions_tarifees;
  return sansPrix > 0 ? `${base}, ${sansPrix} sans prix` : base;
}

/** Un compte de jetons en notation courte : « 1,2 M », « 12 k ». */
function compact(n: number): string {
  return new Intl.NumberFormat("fr-FR", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(n);
}

/** Une dépense en dollars, avec plus de décimales quand elle est infime. */
function dollars(cout: number): string {
  const decimales = cout > 0 && cout < 1 ? 4 : 2;
  return `${cout.toLocaleString("fr-FR", {
    minimumFractionDigits: 2,
    maximumFractionDigits: decimales,
  })} $`;
}

function ou(conflits: Conflit[], maintenant: number | null): string {
  if (conflits.length === 0) return "personne ne se marche dessus";

  const [premier] = conflits;
  const ou = premier.module_path === "" ? "la racine" : premier.module_path;
  if (maintenant === null) return ou;

  const minutes = Math.round(
    (maintenant - Date.parse(premier.dernier_evenement)) / 60000,
  );
  const suite = conflits.length > 1 ? ` et ${conflits.length - 1} autre` : "";

  return `${ou}, ${minutes < 1 ? "à l’instant" : `il y a ${minutes} min`}${suite}`;
}
