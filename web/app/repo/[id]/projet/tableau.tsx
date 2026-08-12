"use client";

import { useCallback, useEffect, useId, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import {
  colonnes,
  reference,
  suggestionsEmplacement,
  titreValide,
  LIBELLE_TYPE,
  type Bloc,
  type StatutBloc,
  type TypeBloc,
} from "@/lib/blocs";

/** Ce que chaque colonne affiche, dans l'ordre du tableau. */
const TITRE_COLONNE: Record<StatutBloc, string> = {
  todo: "À faire",
  doing: "En cours",
  done: "Terminé",
};

const COLONNES_DANS_LORDRE: StatutBloc[] = ["todo", "doing", "done"];

const TYPES: TypeBloc[] = ["feature", "correction", "technique", "exploration"];

/** Les champs relus a chaque signal, identiques a ceux du premier rendu. */
const SELECTION = "id,ref,type,titre,statut,version,chemin,position,created_at";

export function Tableau({
  repoId,
  blocsInitiaux,
  cheminsConnus,
}: {
  repoId: string;
  blocsInitiaux: Bloc[];
  cheminsConnus: string[];
}) {
  const [blocs, setBlocs] = useState(blocsInitiaux);

  const relire = useCallback(async () => {
    const { data } = await createClient()
      .from("blocs")
      .select(SELECTION)
      .eq("repo_id", repoId)
      .order("position", { ascending: true })
      .order("created_at", { ascending: true });
    if (data) setBlocs(data as Bloc[]);
  }, [repoId]);

  // Le temps reel n'est qu'un signal : sa charge utile arrive incomplete, et
  // un `filter` sur la colonne `repo_id` ne livre rien du tout sur la pile
  // locale (constate a l'usage). On ecoute donc sans filtre, comme le plan
  // d'un repo (direct.tsx) : chaque signal relit par l'API, qui elle seule
  // scope a ce depot via la RLS.
  useEffect(() => {
    const supabase = createClient();

    const canal = supabase
      .channel(`blocs-${repoId}`)
      .on("postgres_changes", { event: "*", schema: "public", table: "blocs" }, relire)
      .subscribe();

    return () => {
      supabase.removeChannel(canal);
    };
  }, [repoId, relire]);

  const rangees = colonnes(blocs);

  return (
    <div className="projet">
      <FormulaireBloc repoId={repoId} cheminsConnus={cheminsConnus} onCree={relire} />

      <div className="colonnes">
        {COLONNES_DANS_LORDRE.map((statut) => (
          <section key={statut} className={`colonne colonne-${statut}`} aria-label={TITRE_COLONNE[statut]}>
            <div className="colonne-tete">
              <h2 className="titre colonne-titre">{TITRE_COLONNE[statut]}</h2>
              <span className="colonne-compte">{rangees[statut].length}</span>
            </div>

            {rangees[statut].length === 0 ? (
              <p className="colonne-vide">Rien ici pour l&apos;instant.</p>
            ) : (
              <ul className="cartes">
                {rangees[statut].map((bloc) => (
                  <Carte key={bloc.id} bloc={bloc} />
                ))}
              </ul>
            )}
          </section>
        ))}
      </div>
    </div>
  );
}

function Carte({ bloc }: { bloc: Bloc }) {
  return (
    <li className="carte">
      <div className="carte-tete">
        <span className="carte-titre">{bloc.titre}</span>
        <code className="carte-ref">{reference(bloc.ref)}</code>
      </div>
      <div className="carte-meta">
        <span className="badge">{LIBELLE_TYPE[bloc.type]}</span>
        {bloc.chemin && <code className="carte-chemin">{bloc.chemin}</code>}
      </div>
    </li>
  );
}

/**
 * La seule ecriture que porte cette issue : declarer un bloc en une saisie
 * courte (F1). Il arrive toujours en « A faire » — rien d'autre ne le deplace
 * encore, l'automatisme viendra avec #31 et #32.
 */
function FormulaireBloc({
  repoId,
  cheminsConnus,
  onCree,
}: {
  repoId: string;
  cheminsConnus: string[];
  onCree: () => void;
}) {
  const idListe = useId();
  const [titre, setTitre] = useState("");
  const [type, setType] = useState<TypeBloc>("feature");
  const [chemin, setChemin] = useState("");
  const [enCours, setEnCours] = useState(false);
  const [echec, setEchec] = useState<string | null>(null);

  const suggestions = suggestionsEmplacement(cheminsConnus, chemin);
  const pret = titreValide(titre) && chemin.trim() !== "";

  async function creer(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!pret || enCours) return;

    setEnCours(true);
    setEchec(null);

    const { error } = await createClient().rpc("creer_bloc", {
      p_repo_id: repoId,
      p_titre: titre.trim(),
      p_type: type,
      p_chemin: chemin.trim(),
    });

    if (error) {
      setEchec(error.message);
    } else {
      setTitre("");
      setChemin("");
      setType("feature");
      onCree();
    }

    setEnCours(false);
  }

  return (
    <form className="bloc-form" onSubmit={creer}>
      <div className="bloc-champ bloc-champ-titre">
        <label className="titre" htmlFor="bloc-titre">
          Titre
        </label>
        <input
          id="bloc-titre"
          className="champ"
          type="text"
          value={titre}
          onChange={(e) => setTitre(e.target.value)}
          placeholder="Refaire la landing"
          maxLength={200}
          required
        />
      </div>

      <div className="bloc-champ">
        <label className="titre" htmlFor="bloc-type">
          Type
        </label>
        <select
          id="bloc-type"
          className="champ"
          value={type}
          onChange={(e) => setType(e.target.value as TypeBloc)}
        >
          {TYPES.map((t) => (
            <option key={t} value={t}>
              {LIBELLE_TYPE[t]}
            </option>
          ))}
        </select>
      </div>

      <div className="bloc-champ bloc-champ-emplacement">
        <label className="titre" htmlFor="bloc-emplacement">
          Emplacement
        </label>
        <input
          id="bloc-emplacement"
          className="champ"
          type="text"
          list={idListe}
          value={chemin}
          onChange={(e) => setChemin(e.target.value)}
          placeholder="web/app/checkout"
          autoComplete="off"
          spellCheck={false}
          required
        />
        <datalist id={idListe}>
          {suggestions.map((s) => (
            <option key={s} value={s} />
          ))}
        </datalist>
      </div>

      <button className="bouton" type="submit" disabled={!pret || enCours}>
        {enCours ? "Création…" : "Créer"}
      </button>

      {echec && (
        <p className="echec" role="alert">
          {echec}
        </p>
      )}
    </form>
  );
}
