"use client";

import { useCallback, useEffect, useId, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import {
  colonnes,
  estDecoupe,
  filtrerParType,
  libelleType,
  peutSortirDeTermine as blocPeutSortirDeTermine,
  reference,
  suggestionsEmplacement,
  titreValide,
  LIBELLE_TYPE,
  TYPES,
  type Bloc,
  type StatutBloc,
  type TypeBloc,
} from "@/lib/blocs";
import {
  cheminRequisPourNouvelleIssue,
  issuesDuBloc,
  peutSortirDeTermine as issuePeutSortirDeTermine,
  texteAvancement,
  type Issue,
} from "@/lib/issues";

/** Ce que chaque colonne affiche, dans l'ordre du tableau. */
const TITRE_COLONNE: Record<StatutBloc, string> = {
  todo: "À faire",
  doing: "En cours",
  done: "Terminé",
};

/** Le libelle d'une issue reprend celui des colonnes : c'est le meme statut,
 *  seule sa portee change - un bloc ou une issue (#30). */
const LIBELLE_STATUT: Record<StatutBloc, string> = TITRE_COLONNE;

const COLONNES_DANS_LORDRE: StatutBloc[] = ["todo", "doing", "done"];

/** Les champs relus a chaque signal, identiques a ceux du premier rendu. */
const SELECTION_BLOCS = "id,ref,type,titre,statut,version,chemin,position,created_at";
const SELECTION_ISSUES = "id,ref,bloc_id,titre,chemin,statut,version,position,created_at";

export function Tableau({
  repoId,
  blocsInitiaux,
  issuesInitiales,
  cheminsConnus,
}: {
  repoId: string;
  blocsInitiaux: Bloc[];
  issuesInitiales: Issue[];
  cheminsConnus: string[];
}) {
  const [blocs, setBlocs] = useState(blocsInitiaux);
  const [issues, setIssues] = useState(issuesInitiales);
  // Un seul bloc ouvert a la fois : l'ouverture reste un detail consulte en
  // place, jamais une deuxieme vue du tableau (FR-020).
  const [blocOuvert, setBlocOuvert] = useState<string | null>(null);
  // `null` = tous les types (FR-031). Un etat purement local : le filtre lit
  // le tableau, il ne l'ecrit pas, rien ne le persiste entre deux visites.
  const [filtreType, setFiltreType] = useState<TypeBloc | null>(null);

  const relire = useCallback(async () => {
    const supabase = createClient();
    const [blocsLus, issuesLues] = await Promise.all([
      supabase
        .from("blocs")
        .select(SELECTION_BLOCS)
        .eq("repo_id", repoId)
        .order("position", { ascending: true })
        .order("created_at", { ascending: true }),
      supabase
        .from("issues")
        .select(SELECTION_ISSUES)
        .eq("repo_id", repoId)
        .order("position", { ascending: true })
        .order("created_at", { ascending: true }),
    ]);
    if (blocsLus.data) setBlocs(blocsLus.data as Bloc[]);
    if (issuesLues.data) setIssues(issuesLues.data as Issue[]);
  }, [repoId]);

  // Le temps reel n'est qu'un signal : sa charge utile arrive incomplete, et
  // un `filter` sur la colonne `repo_id` ne livre rien du tout sur la pile
  // locale (constate a l'usage). On ecoute donc sans filtre, comme le plan
  // d'un repo (direct.tsx) : chaque signal relit par l'API, qui elle seule
  // scope a ce depot via la RLS. `issues` est ecoutee au meme titre que
  // `blocs` : une issue qui change (creation, statut) peut faire bouger
  // l'avancement d'un bloc et son statut derive (etat_bloc).
  useEffect(() => {
    const supabase = createClient();

    const canal = supabase
      .channel(`projet-${repoId}`)
      .on("postgres_changes", { event: "*", schema: "public", table: "blocs" }, relire)
      .on("postgres_changes", { event: "*", schema: "public", table: "issues" }, relire)
      .subscribe();

    return () => {
      supabase.removeChannel(canal);
    };
  }, [repoId, relire]);

  // Le filtre s'applique avant de ranger dans les colonnes : ce que le
  // tableau montre reste toujours coherent entre "combien dans cette
  // colonne" et "combien de cartes dessous" (FR-031, les trois colonnes a la
  // fois, jamais une seule).
  const rangees = colonnes(filtrerParType(blocs, filtreType));

  return (
    <div className="projet">
      <FormulaireBloc repoId={repoId} cheminsConnus={cheminsConnus} onCree={relire} />
      <FiltreType filtre={filtreType} onChoisir={setFiltreType} />

      <div className="colonnes">
        {COLONNES_DANS_LORDRE.map((statut) => (
          <section key={statut} className={`colonne colonne-${statut}`} aria-label={TITRE_COLONNE[statut]}>
            <div className="colonne-tete">
              <h2 className="titre colonne-titre">{TITRE_COLONNE[statut]}</h2>
              <span className="colonne-compte">{rangees[statut].length}</span>
            </div>

            {rangees[statut].length === 0 ? (
              <p className="colonne-vide">
                {filtreType === null ? "Rien ici pour l'instant." : "Rien de ce type ici."}
              </p>
            ) : (
              <ul className="cartes">
                {rangees[statut].map((bloc) => (
                  <Carte
                    key={bloc.id}
                    bloc={bloc}
                    issues={issuesDuBloc(issues, bloc.id)}
                    ouverte={blocOuvert === bloc.id}
                    cheminsConnus={cheminsConnus}
                    onBasculer={() => setBlocOuvert((courant) => (courant === bloc.id ? null : bloc.id))}
                    onIssueCreee={relire}
                    onSortie={relire}
                    onRetype={relire}
                  />
                ))}
              </ul>
            )}
          </section>
        ))}
      </div>
    </div>
  );
}

/**
 * Le filtre par type (F10, FR-031) : quatre bascules textuelles plus « Tous »,
 * pas une case a cocher par type ni un menu deroulant qui cacherait les
 * options. Un seul type actif a la fois - filtrer sur plusieurs types en
 * meme temps n'est pas demande par le PRD, et une bascule simple reste plus
 * lisible qu'un groupe de cases.
 *
 * De vrais `<button>`, pas des `<div onClick>` : le clavier et le lecteur
 * d'ecran les recoivent gratuitement, sans geste dedie a coder (#35 reste
 * hors scope, mais rien ici ne doit lui fermer la porte).
 */
function FiltreType({
  filtre,
  onChoisir,
}: {
  filtre: TypeBloc | null;
  onChoisir: (type: TypeBloc | null) => void;
}) {
  return (
    <div className="filtre-type" role="group" aria-label="Filtrer par type">
      <button
        type="button"
        className="filtre-bouton"
        aria-pressed={filtre === null}
        onClick={() => onChoisir(null)}
      >
        Tous
      </button>
      {TYPES.map((type) => (
        <button
          key={type}
          type="button"
          className="filtre-bouton"
          aria-pressed={filtre === type}
          onClick={() => onChoisir(filtre === type ? null : type)}
        >
          {LIBELLE_TYPE[type]}
        </button>
      ))}
    </div>
  );
}

function Carte({
  bloc,
  issues,
  ouverte,
  cheminsConnus,
  onBasculer,
  onIssueCreee,
  onSortie,
  onRetype,
}: {
  bloc: Bloc;
  issues: Issue[];
  ouverte: boolean;
  cheminsConnus: string[];
  onBasculer: () => void;
  onIssueCreee: () => void;
  onSortie: () => void;
  onRetype: () => void;
}) {
  const decoupe = estDecoupe(bloc);
  const idPanneau = `issues-${bloc.id}`;

  return (
    <li className="carte">
      <div className="carte-tete">
        <span className="carte-titre">{bloc.titre}</span>
        <code className="carte-ref">{reference(bloc.ref)}</code>
      </div>
      <div className="carte-meta">
        {/* Le type se lit et se retype au meme endroit (F10, FR-032) : un
            bloc decoupe se retype tout autant qu'un bloc simple - seul son
            STATUT est derive de ses issues (etat_bloc(), #30), jamais son
            type. Et une carte deja Terminee se retype aussi (FR-032,
            "a tout moment") : rien ici ne le distingue d'un bloc a faire, le
            serveur ne protege que `statut`, jamais `type` (#33). */}
        <SelectionType bloc={bloc} onRetype={onRetype} />
        {decoupe ? (
          <span className="bloc-avancement">{texteAvancement(issues)}</span>
        ) : (
          bloc.chemin && <code className="carte-chemin">{bloc.chemin}</code>
        )}
        <button
          type="button"
          className="bloc-issues-bouton"
          aria-expanded={ouverte}
          aria-controls={idPanneau}
          onClick={onBasculer}
        >
          {ouverte ? "Masquer les issues" : decoupe ? "Voir les issues" : "Découper"}
        </button>
        {/* La seule sortie de « Termine » (F8, FR-025) : jamais l'inverse -
            rien ici ne propose de glisser une carte VERS Termine, le geste
            n'existe pas dans cette interface. */}
        {blocPeutSortirDeTermine(bloc) && (
          <BoutonSortieTermine table="blocs" id={bloc.id} libelle="Ramener en cours" onSortie={onSortie} />
        )}
      </div>

      {ouverte && (
        <div className="bloc-issues-panneau" id={idPanneau}>
          {issues.length > 0 && (
            <ul className="bloc-issues-liste">
              {issues.map((issue) => (
                <li key={issue.id} className={`bloc-issue bloc-issue-${issue.statut}`}>
                  <div className="bloc-issue-tete">
                    <span className="bloc-issue-titre">{issue.titre}</span>
                    <code className="bloc-issue-ref">{reference(issue.ref)}</code>
                  </div>
                  <div className="bloc-issue-meta">
                    <span className="bloc-issue-statut">{LIBELLE_STATUT[issue.statut]}</span>
                    <code className="bloc-issue-chemin">{issue.chemin || "/"}</code>
                    {issuePeutSortirDeTermine(issue) && (
                      <BoutonSortieTermine
                        table="issues"
                        id={issue.id}
                        libelle="Ramener en cours"
                        onSortie={onSortie}
                      />
                    )}
                  </div>
                </li>
              ))}
            </ul>
          )}

          <FormulaireIssue
            bloc={bloc}
            issuesExistantes={issues}
            cheminsConnus={cheminsConnus}
            onCree={onIssueCreee}
          />
        </div>
      )}
    </li>
  );
}

/**
 * Le geste de sortie de « Terminé » (F8, FR-025) : un PATCH direct qui pose
 * `doing`, jamais l'inverse - la base refuse de toute facon `done` depuis le
 * web (policy `update`, #33). Partagee entre un bloc simple et une issue :
 * la meme ecriture, sur l'une ou l'autre table.
 */
function BoutonSortieTermine({
  table,
  id,
  libelle,
  onSortie,
}: {
  table: "blocs" | "issues";
  id: string;
  libelle: string;
  onSortie: () => void;
}) {
  const [enCours, setEnCours] = useState(false);

  async function sortir() {
    if (enCours) return;
    setEnCours(true);

    const { error } = await createClient().from(table).update({ statut: "doing" }).eq("id", id);

    setEnCours(false);
    if (!error) onSortie();
  }

  return (
    <button type="button" className="bouton-sortie-termine" onClick={sortir} disabled={enCours}>
      {enCours ? "Retour en cours…" : libelle}
    </button>
  );
}

/**
 * Le retypage d'un bloc (F10, FR-032) : un `<select>`, pas un second
 * formulaire ni une boite modale - changer d'avis sur le rangement d'une
 * carte doit couter aussi peu qu'un menu deroulant, jamais une interruption.
 * Un PATCH direct sur `type` uniquement : `statut` et `version` ne figurent
 * pas dans le corps de la requete, donc rien ne peut les faire bouger par
 * megarde depuis ce geste (l'historique - `fermetures` - n'est de toute
 * facon jamais ecrit que par `fermer_par_reference()`, jamais par le web).
 *
 * Aucun etat local optimiste : comme `BoutonSortieTermine`, l'affichage
 * n'avance qu'apres que l'ecriture a reussi et que `onRetype` (= `relire`) a
 * relu la base - la meme carte peut etre retypee depuis un autre onglet au
 * meme moment, et c'est alors la derniere ecriture confirmee qui gagne,
 * jamais un affichage local qui aurait menti en avance sur la base.
 */
function SelectionType({ bloc, onRetype }: { bloc: Bloc; onRetype: () => void }) {
  const [enCours, setEnCours] = useState(false);
  const [echec, setEchec] = useState<string | null>(null);

  // La contrainte `check` de la base peut porter un type que ce front ne
  // connait pas encore (elle evolue independamment d'un deploiement web) :
  // sans cet ajout, un bloc deja marque de ce type serait impossible a
  // selectionner tel quel dans son propre menu.
  const options = TYPES.includes(bloc.type) ? TYPES : [bloc.type, ...TYPES];

  async function retyper(nouveauType: string) {
    if (nouveauType === bloc.type || enCours) return;
    setEnCours(true);
    setEchec(null);

    const { error } = await createClient().from("blocs").update({ type: nouveauType }).eq("id", bloc.id);

    setEnCours(false);
    if (error) {
      setEchec(error.message);
    } else {
      onRetype();
    }
  }

  return (
    <span className="type-select-champ">
      <select
        className="type-select"
        aria-label={`Type de ${bloc.titre}`}
        value={bloc.type}
        disabled={enCours}
        onChange={(e) => retyper(e.target.value)}
      >
        {options.map((type) => (
          <option key={type} value={type}>
            {libelleType(type)}
          </option>
        ))}
      </select>
      {echec && (
        <span className="echec" role="alert">
          {echec}
        </span>
      )}
    </span>
  );
}

/**
 * L'ajout d'une issue a un bloc existant : la seule ecriture qui decoupe un
 * bloc (F2). Le champ emplacement ne s'affiche que quand il est requis
 * (FR-006) - la premiere issue d'un bloc simple herite le sien, la base s'en
 * charge (bloc_coherent()) et le formulaire n'a rien a lui demander.
 */
function FormulaireIssue({
  bloc,
  issuesExistantes,
  cheminsConnus,
  onCree,
}: {
  bloc: Bloc;
  issuesExistantes: Issue[];
  cheminsConnus: string[];
  onCree: () => void;
}) {
  const idListe = useId();
  const [titre, setTitre] = useState("");
  const [chemin, setChemin] = useState("");
  const [enCours, setEnCours] = useState(false);
  const [echec, setEchec] = useState<string | null>(null);

  const cheminRequis = cheminRequisPourNouvelleIssue(bloc, issuesExistantes);
  const suggestions = suggestionsEmplacement(cheminsConnus, chemin);
  const pret = titreValide(titre) && (!cheminRequis || chemin.trim() !== "");

  async function creer(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!pret || enCours) return;

    setEnCours(true);
    setEchec(null);

    const { error } = await createClient().rpc("creer_issue", {
      p_bloc_id: bloc.id,
      p_titre: titre.trim(),
      p_chemin: cheminRequis ? chemin.trim() : null,
    });

    if (error) {
      setEchec(error.message);
    } else {
      setTitre("");
      setChemin("");
      onCree();
    }

    setEnCours(false);
  }

  return (
    <form className="issue-form" onSubmit={creer}>
      <div className="bloc-champ bloc-champ-titre">
        <label className="titre" htmlFor={`issue-titre-${bloc.id}`}>
          Nouvelle issue
        </label>
        <input
          id={`issue-titre-${bloc.id}`}
          className="champ"
          type="text"
          value={titre}
          onChange={(e) => setTitre(e.target.value)}
          placeholder="Nouveau visuel"
          maxLength={200}
          required
        />
      </div>

      {cheminRequis && (
        <div className="bloc-champ bloc-champ-emplacement">
          <label className="titre" htmlFor={`issue-emplacement-${bloc.id}`}>
            Emplacement
          </label>
          <input
            id={`issue-emplacement-${bloc.id}`}
            className="champ"
            type="text"
            list={idListe}
            value={chemin}
            onChange={(e) => setChemin(e.target.value)}
            placeholder="web/app/hero/bandeau"
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
      )}

      <button className="bouton" type="submit" disabled={!pret || enCours}>
        {enCours ? "Ajout…" : "Ajouter"}
      </button>

      {echec && (
        <p className="echec" role="alert">
          {echec}
        </p>
      )}
    </form>
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
