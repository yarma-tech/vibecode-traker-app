"use client";

import { useState } from "react";
import { createClient } from "@/lib/supabase/client";

export type Correspondance = {
  owner: string;
  label: "perso" | "pro";
};

type Choix = "perso" | "pro" | null;

export function Comptes({
  owners,
  initiales,
}: {
  owners: string[];
  initiales: Correspondance[];
}) {
  const [classement, setClassement] = useState<Record<string, Choix>>(() => {
    const depart: Record<string, Choix> = {};
    for (const owner of owners) depart[owner] = null;
    for (const c of initiales) depart[c.owner] = c.label;
    return depart;
  });
  const [echec, setEchec] = useState<string | null>(null);

  // Classer, c'est poser une correspondance owner -> compte. Recliquer sur le
  // meme choix la retire : un owner peut redevenir non classe, donc sans badge.
  async function classer(owner: string, choix: "perso" | "pro") {
    setEchec(null);
    const avant = classement[owner];
    const apres = avant === choix ? null : choix;

    // Optimiste : l'ecran repond tout de suite, la base suit.
    setClassement((c) => ({ ...c, [owner]: apres }));

    const supabase = createClient();
    if (apres === null) {
      const { error } = await supabase.from("account_mappings").delete().eq("owner", owner);
      if (error) {
        setClassement((c) => ({ ...c, [owner]: avant }));
        setEchec(`Impossible de déclasser ${owner} : ${error.message}`);
      }
      return;
    }

    const { error } = await supabase
      .from("account_mappings")
      .upsert({ owner, label: apres }, { onConflict: "user_id,owner" });
    if (error) {
      setClassement((c) => ({ ...c, [owner]: avant }));
      setEchec(`Impossible de classer ${owner} : ${error.message}`);
    }
  }

  return (
    <>
      {echec && (
        <p className="echec" role="alert">
          {echec}
        </p>
      )}
      <ul className="comptes">
        {owners.map((owner) => (
          <li key={owner} className="compte-ligne">
            <span className="compte-owner">{owner}</span>
            <div className="compte-choix" role="group" aria-label={`Compte de ${owner}`}>
              {(["perso", "pro"] as const).map((choix) => (
                <button
                  key={choix}
                  type="button"
                  className={
                    classement[owner] === choix ? `onglet actif ${choix}` : "onglet"
                  }
                  aria-pressed={classement[owner] === choix}
                  onClick={() => classer(owner, choix)}
                >
                  @{choix}
                </button>
              ))}
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}
