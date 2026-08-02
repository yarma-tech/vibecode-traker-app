"use client";

import { useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";

export type Machine = {
  id: string;
  label: string;
  platform: string | null;
  last_seen_at: string | null;
};

/** Seuil au-dela duquel une machine est declaree muette (spec, section 8). */
const SILENCE_MS = 90_000;

function depuis(instant: string | null, maintenant: number): string {
  if (!instant) return "jamais vue";

  const secondes = Math.max(0, Math.round((maintenant - Date.parse(instant)) / 1000));
  if (secondes < 60) return `il y a ${secondes} s`;

  const minutes = Math.round(secondes / 60);
  if (minutes < 60) return `il y a ${minutes} min`;

  const heures = Math.round(minutes / 60);
  return heures < 24 ? `il y a ${heures} h` : `il y a ${Math.round(heures / 24)} j`;
}

function muette(instant: string | null, maintenant: number): boolean {
  return !instant || maintenant - Date.parse(instant) > SILENCE_MS;
}

export function Machines({ initiales }: { initiales: Machine[] }) {
  const [machines, setMachines] = useState(initiales);
  const [maintenant, setMaintenant] = useState(() => Date.now());

  // L'horloge avance seule : sans cela « il y a 2 s » reste affiche
  // indefiniment alors que la machine est peut-etre morte depuis.
  useEffect(() => {
    const battement = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => clearInterval(battement);
  }, []);

  useEffect(() => {
    const supabase = createClient();
    const canal = supabase
      .channel("machines")
      .on(
        "postgres_changes",
        { event: "*", schema: "public", table: "machines" },
        (message) => {
          const ligne = message.new as Machine;
          setMachines((precedentes) => {
            const connue = precedentes.some((m) => m.id === ligne.id);
            return connue
              ? precedentes.map((m) => (m.id === ligne.id ? { ...m, ...ligne } : m))
              : [...precedentes, ligne];
          });
        },
      )
      .subscribe();

    return () => {
      supabase.removeChannel(canal);
    };
  }, []);

  if (machines.length === 0) {
    return (
      <div className="vide">
        <p className="vide-titre">Aucune machine reliée pour l&apos;instant.</p>
        <p className="vide-suite">
          Lance <code>vibemap</code> sur ton poste : il apparaîtra ici dès son
          premier signe de vie.
        </p>
      </div>
    );
  }

  return (
    <ul className="machines">
      {machines.map((machine) => {
        const silencieuse = muette(machine.last_seen_at, maintenant);
        return (
          <li key={machine.id} className={silencieuse ? "machine morte" : "machine"}>
            <span className="nom">{machine.label}</span>
            {machine.platform && <span className="plateforme">{machine.platform}</span>}
            <span className="etat">
              {silencieuse ? "muette" : "à jour"}
              <span className="quand">{depuis(machine.last_seen_at, maintenant)}</span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}
