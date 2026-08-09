"use client";

import { useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
// La bascule muette/figée et son « depuis N » sont partagés avec le rail et
// l'écran d'un repo : une seule règle du silence, impossible à contredire.
import { depuisTexte as depuis, estFige as muette } from "@/lib/figement";

export type Machine = {
  id: string;
  label: string;
  platform: string | null;
  last_seen_at: string | null;
  revoked_at: string | null;
};

export function Machines({ initiales }: { initiales: Machine[] }) {
  const [machines, setMachines] = useState(initiales);
  const [maintenant, setMaintenant] = useState(() => Date.now());
  const [echec, setEchec] = useState<string | null>(null);

  // Revoquer coupe les ecritures de la machine a la milliseconde : la regle
  // vit dans la RLS, pas ici. L'ecran ne fait que poser la date.
  async function revoquer(machine: Machine) {
    setEchec(null);
    const { error } = await createClient()
      .from("machines")
      .update({ revoked_at: new Date().toISOString() })
      .eq("id", machine.id);

    if (error) setEchec(`Impossible de révoquer ${machine.label} : ${error.message}`);
    // Le rafraichissement viendra du canal temps reel, declenche par cette ecriture.
  }

  // L'horloge avance seule : sans cela « il y a 2 s » reste affiche
  // indefiniment alors que la machine est peut-etre morte depuis.
  useEffect(() => {
    const battement = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => clearInterval(battement);
  }, []);

  // Le temps reel sert de signal, pas de source de verite : sa charge utile
  // arrive incomplete, et un champ absent vaut `undefined`, pas `null`. On
  // relit donc la liste par l'API, qui passe par la RLS et rend tout.
  useEffect(() => {
    const supabase = createClient();

    async function relire() {
      const { data } = await supabase
        .from("machines")
        .select("id,label,platform,last_seen_at,revoked_at")
        .order("label");

      if (data) setMachines(data as Machine[]);
    }

    const canal = supabase
      .channel("machines")
      .on(
        "postgres_changes",
        { event: "*", schema: "public", table: "machines" },
        relire,
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
    <>
      {echec && (
        <p className="echec" role="alert">
          {echec}
        </p>
      )}
      <ul className="machines">
        {machines.map((machine) => {
          const revoquee = Boolean(machine.revoked_at);
          const silencieuse = !revoquee && muette(machine.last_seen_at, maintenant);
          const classe = revoquee ? "machine revoquee" : silencieuse ? "machine morte" : "machine";

          return (
            <li key={machine.id} className={classe}>
              <span className="nom">{machine.label}</span>
              {machine.platform && <span className="plateforme">{machine.platform}</span>}
              <span className="etat">
                {revoquee ? "révoquée" : silencieuse ? "muette" : "à jour"}
                <span className="quand">{depuis(machine.last_seen_at, maintenant)}</span>
              </span>
              {!revoquee && (
                <button className="lien" onClick={() => revoquer(machine)}>
                  Révoquer
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </>
  );
}
