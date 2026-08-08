"use client";

import { useCallback, useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { Bandeau } from "./bandeau";
import { Journal } from "./journal";
import { Plan, type Module } from "./plan";

/** Ce que la base répond pour un module actif. La base ne dit rien des autres. */
export type Etat = {
  module_path: string;
  etat: "lu" | "ecrit" | "conflit";
  lectures: number;
  ecritures: number;
  sessions_ecrivant: number;
  /** Les sessions qui écrivent dans le sous-arbre, pour les nommer. */
  sessions: string[];
  dernier_evenement: string;
};

export type Evenement = {
  id: number;
  session_id: string;
  module_path: string;
  file_path: string;
  kind: "read" | "write";
  occurred_at: string;
};

/**
 * Le relevé cumulé d'un repo : ses jetons, son coût, ses sessions. Somme de
 * toute son histoire, pas de la fenêtre. Le coût vaut `null` quand aucune
 * session n'a de modèle connu : des jetons sans coût valent mieux qu'un coût faux.
 */
export type Releve = {
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  cout_usd: number | null;
  sessions: number;
  sessions_tarifees: number;
};

/**
 * Un conflit, au module où les deux agents se rencontrent. La base n'en rend
 * qu'un par rencontre, même si le rouge remonte jusqu'à la racine sur le plan.
 */
export type Conflit = {
  repo_id: string;
  module_path: string;
  sessions_ecrivant: number;
  sessions: string[];
  dernier_evenement: string;
};

/**
 * La fenêtre finit toujours par s'écouler, même quand plus rien n'arrive :
 * sans relecture régulière, une parcelle resterait ambre après la fin du
 * travail. Le temps réel couvre l'allumage, cette horloge couvre l'extinction.
 */
const RELECTURE_MS = 15_000;

/** Combien d'événements le journal garde à l'écran. */
const LIGNES_DU_JOURNAL = 60;

export function Direct({
  repoId,
  modules,
  locTotal,
  etatsInitiaux,
  evenementsInitiaux,
  conflitsInitiaux,
  agentsInitiaux,
  releveInitial,
  fenetreSecondes,
}: {
  repoId: string;
  modules: Module[];
  locTotal: number;
  etatsInitiaux: Etat[];
  evenementsInitiaux: Evenement[];
  conflitsInitiaux: Conflit[];
  agentsInitiaux: number;
  releveInitial: Releve | null;
  fenetreSecondes: number;
}) {
  const [etats, setEtats] = useState(etatsInitiaux);
  const [evenements, setEvenements] = useState(evenementsInitiaux);
  const [conflits, setConflits] = useState(conflitsInitiaux);
  const [presents, setPresents] = useState(agentsInitiaux);
  const [releve, setReleve] = useState(releveInitial);

  const relire = useCallback(async () => {
    const supabase = createClient();

    const [etat, journal, alarmes, agents, bilan] = await Promise.all([
      supabase.rpc("etat_modules", { p_repo_id: repoId }),
      supabase
        .from("activity_events")
        .select("id,session_id,module_path,file_path,kind,occurred_at")
        .eq("repo_id", repoId)
        .order("occurred_at", { ascending: false })
        .limit(LIGNES_DU_JOURNAL),
      supabase.rpc("conflits", { p_repo_id: repoId }),
      supabase.rpc("agents_actifs", { p_repo_id: repoId }),
      supabase.rpc("releve_repo", { p_repo_id: repoId }),
    ]);

    if (etat.data) setEtats(etat.data as Etat[]);
    if (journal.data) setEvenements(journal.data as Evenement[]);
    if (alarmes.data) setConflits(alarmes.data as Conflit[]);
    if (typeof agents.data === "number") setPresents(agents.data);
    if (bilan.data) setReleve((bilan.data as Releve[])[0] ?? null);
  }, [repoId]);

  // Le temps réel n'est qu'un signal : sa charge utile arrive incomplète, et
  // un champ absent y vaut `undefined`, pas `null`. On relit donc par l'API.
  useEffect(() => {
    const supabase = createClient();

    const canal = supabase
      .channel(`activite-${repoId}`)
      .on(
        "postgres_changes",
        { event: "INSERT", schema: "public", table: "activity_events" },
        relire,
      )
      .subscribe();

    const horloge = setInterval(relire, RELECTURE_MS);

    return () => {
      supabase.removeChannel(canal);
      clearInterval(horloge);
    };
  }, [repoId, relire]);

  const lettres = agents(evenements, conflits);

  return (
    <>
      <Bandeau agents={presents} conflits={conflits} etats={etats} releve={releve} />

      <div className="releve">
        <div className="releve-plan">
          <Plan modules={modules} locTotal={locTotal} etats={etats} />
        </div>
        <Journal
          evenements={evenements}
          conflits={conflits}
          agents={lettres}
          fenetreSecondes={fenetreSecondes}
        />
      </div>
    </>
  );
}

/**
 * Une lettre par session, dans l'ordre où elle est apparue. Un identifiant de
 * session est un uuid : illisible, et surtout impossible à comparer d'un coup
 * d'œil entre le plan et le journal.
 *
 * Les sessions en conflit sont ajoutées après celles du journal : une alarme
 * doit pouvoir nommer ses deux agents même si leurs lignes sont sorties de
 * l'écran.
 */
export function agents(
  evenements: Evenement[],
  conflits: Conflit[] = [],
): Map<string, string> {
  const vus = [
    ...[...evenements]
      .sort((a, b) => a.occurred_at.localeCompare(b.occurred_at))
      .map((e) => e.session_id),
    ...conflits.flatMap((conflit) => conflit.sessions),
  ];

  const lettres = new Map<string, string>();
  for (const session of vus) {
    if (lettres.has(session)) continue;
    lettres.set(session, String.fromCharCode(65 + (lettres.size % 26)));
  }

  return lettres;
}
