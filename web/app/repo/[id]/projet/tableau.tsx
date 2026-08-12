"use client";

import { useCallback, useEffect, useId, useRef, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import {
  afficherAvancement,
  colonnes,
  compteSansExploration,
  estDecoupe,
  estExploration,
  estToucheFiltre,
  filtrerParType,
  indexSuivantFiltre,
  libelleCopier,
  libelleType,
  messageCopie,
  origineCourte,
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
import {
  clinDoeilVisible,
  dureeDepuisSignal,
  estFrais,
  etatTableau,
  type EtatTableau,
} from "@/lib/fraicheur";

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

/** Les champs relus a chaque signal, identiques a ceux du premier rendu. Les
 *  colonnes `prd_*` (#37) : la priorite et l'origine se lisent, jamais un
 *  champ qui permettrait de les ecrire depuis ce formulaire. */
const SELECTION_BLOCS =
  "id,ref,type,titre,statut,version,chemin,position,created_at,prd_cle,prd_priorite,prd_a_clarifier,prd_absent,prd_converti";
const SELECTION_ISSUES = "id,ref,bloc_id,titre,chemin,statut,version,position,created_at";

/** La fenetre finit par s'ecouler meme quand plus rien n'arrive : sans
 *  relecture reguliere, un signal qui vient de manquer le seuil de fraicheur
 *  (F14) resterait affiche « frais » jusqu'au prochain evenement temps reel -
 *  qui, justement, n'arrive plus si le daemon s'est tu. Meme raison qu'a
 *  l'accueil et sur le plan d'un repo (`accueil.tsx`, `direct.tsx`). */
const RELECTURE_MS = 15_000;

export function Tableau({
  repoId,
  machineId,
  scannedAtInitial,
  dernierBattementInitial,
  blocsInitiaux,
  issuesInitiales,
  cheminsConnus,
}: {
  repoId: string;
  machineId: string;
  scannedAtInitial: string;
  dernierBattementInitial: string | null;
  blocsInitiaux: Bloc[];
  issuesInitiales: Issue[];
  cheminsConnus: string[];
}) {
  const [blocs, setBlocs] = useState(blocsInitiaux);
  const [issues, setIssues] = useState(issuesInitiales);
  // Les deux colonnes dont F14 derive la fraicheur (issue #39) : la derniere
  // cartographie complete et le dernier battement de la machine qui porte ce
  // repo. Relues a chaque passage comme le reste, jamais recalculees ici -
  // c'est `lib/fraicheur.ts` qui en tire un age, jamais ce composant.
  const [scannedAt, setScannedAt] = useState(scannedAtInitial);
  const [dernierBattement, setDernierBattement] = useState(dernierBattementInitial);
  // Un seul bloc ouvert a la fois : l'ouverture reste un detail consulte en
  // place, jamais une deuxieme vue du tableau (FR-020).
  const [blocOuvert, setBlocOuvert] = useState<string | null>(null);
  // `null` = tous les types (FR-031). Un etat purement local : le filtre lit
  // le tableau, il ne l'ecrit pas, rien ne le persiste entre deux visites.
  const [filtreType, setFiltreType] = useState<TypeBloc | null>(null);
  // La reference de la derniere carte a avoir porte le focus (#35). Suivie en
  // continu par `focusin` plutot que capturee au moment de la relecture : le
  // geste local de sortie de Termine desactive son bouton des le clic (pour
  // eviter un double envoi), ce qui fait retomber le focus sur `<body>` AVANT
  // meme que la relecture ne demarre - la capturer plus tard trouverait deja
  // `<body>` et aurait perdu la piste. `focusin` ne se declenche jamais vers
  // `<body>` (ce n'est pas une vraie cible de focus), la valeur suivie reste
  // donc celle de la derniere carte reellement focalisee par l'utilisateur.
  const refDerniereCarteFocalisee = useRef<string | null>(null);

  useEffect(() => {
    function surFocus(e: FocusEvent) {
      const cible = e.target as HTMLElement | null;
      const carte = cible?.closest<HTMLElement>(".carte, .bloc-issue") ?? null;
      refDerniereCarteFocalisee.current =
        carte?.querySelector<HTMLElement>("[data-ref]")?.dataset.ref ?? null;
    }
    document.addEventListener("focusin", surFocus);
    return () => document.removeEventListener("focusin", surFocus);
  }, []);

  // L'horloge ne demarre qu'apres le montage, comme sur l'accueil et le plan
  // d'un repo (`accueil.tsx`, `direct.tsx`) : rendue sur le serveur, elle ne
  // correspondrait pas a celle du navigateur. Tant qu'elle vaut `null`, le
  // tableau reste frais - le premier rendu reste celui du serveur, jamais un
  // clignotement en muet avant meme que l'horloge n'ait demarre.
  const [maintenant, setMaintenant] = useState<number | null>(null);
  useEffect(() => {
    const arrivee = setTimeout(() => setMaintenant(Date.now()), 0);
    const horloge = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => {
      clearTimeout(arrivee);
      clearInterval(horloge);
    };
  }, []);

  const relire = useCallback(async () => {
    const supabase = createClient();
    const [blocsLus, issuesLues, repoLu, machineLue] = await Promise.all([
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
      // F14 (issue #39) : le battement retabli par le retour du daemon
      // degele le tableau sans rechargement, tout comme le plan d'un repo
      // (`direct.tsx`) degele son bandeau au retour de `machines`.
      supabase.from("repos").select("scanned_at").eq("id", repoId).maybeSingle(),
      supabase.from("machines").select("last_seen_at").eq("id", machineId).maybeSingle(),
    ]);
    if (blocsLus.data) setBlocs(blocsLus.data as Bloc[]);
    if (issuesLues.data) setIssues(issuesLues.data as Issue[]);
    if (repoLu.data) setScannedAt(repoLu.data.scanned_at as string);
    if (machineLue.data) setDernierBattement((machineLue.data.last_seen_at as string | null) ?? null);
  }, [repoId, machineId]);

  // Rejoue apres chaque relecture : si le focus a fini sur `<body>` (perdu,
  // que ce soit par la desactivation d'un bouton en cours d'envoi ou par le
  // demontage/remontage d'une carte qui change de colonne), on le repose sur
  // le bouton de copie de la derniere carte suivie par `focusin` - une
  // approximation assumee (ce n'est pas forcement le controle exact qui
  // avait le focus), mais qui laisse toujours l'utilisateur sur la carte
  // qu'il consultait plutot que de le renvoyer en haut de la page.
  useEffect(() => {
    if (document.activeElement !== document.body) return;
    const cible = refDerniereCarteFocalisee.current;
    if (!cible) return;
    document.querySelector<HTMLElement>(`[data-ref="${cible}"]`)?.focus();
  }, [blocs, issues]);

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
      // F14 (issue #39) : `repos` porte `scanned_at`, `machines` porte
      // `last_seen_at` - les deux signaux dont la fraicheur se derive. Sans
      // cette ecoute, un daemon qui repart ne rafraichirait le tableau qu'a
      // la prochaine ecriture de bloc ou d'issue, potentiellement jamais.
      .on("postgres_changes", { event: "*", schema: "public", table: "repos" }, relire)
      .on("postgres_changes", { event: "*", schema: "public", table: "machines" }, relire)
      .subscribe();

    // Le temps reel n'est qu'un signal, sa charge utile arrive incomplete :
    // sans relecture reguliere, un daemon qui s'arrete sans plus rien ecrire
    // ne declenche justement plus aucun evenement pour faire redescendre le
    // tableau en muet - meme raison qu'a l'accueil et sur le plan d'un repo.
    const horloge = setInterval(relire, RELECTURE_MS);

    return () => {
      supabase.removeChannel(canal);
      clearInterval(horloge);
    };
  }, [repoId, relire]);

  // Le filtre s'applique avant de ranger dans les colonnes : ce que le
  // tableau montre reste toujours coherent entre "combien dans cette
  // colonne" et "combien de cartes dessous" (FR-031, les trois colonnes a la
  // fois, jamais une seule).
  const rangees = colonnes(filtrerParType(blocs, filtreType));

  // F14 (issue #39) : la fraicheur et l'etat vide se calculent sur la
  // totalite du tableau, jamais sur ce que le filtre par type laisse
  // visible - filtrer sur "correction" quand il n'y en a aucune ne doit pas
  // faire passer un tableau par ailleurs bien vivant pour "muet".
  // `frais` vaut vrai (optimiste) tant que l'horloge n'a pas demarre : le
  // premier rendu reste celui du serveur, comme `fige`/`muette` ailleurs
  // (accueil.tsx, direct.tsx) - mais leur drapeau signale le MAUVAIS etat
  // (defaut a faux = optimiste), quand le notre signale le BON (`frais`) :
  // le meme court-circuit `maintenant !== null && ...` inverserait la
  // polarite et ferait clignoter "muet" une frame avant que l'horloge ne
  // parte, exactement l'approximation a eviter ici.
  const frais = maintenant === null || estFrais(scannedAt, dernierBattement, maintenant);
  const colonnesCompletes = colonnes(blocs);
  const etatVide: EtatTableau = etatTableau(
    {
      todo: colonnesCompletes.todo.length,
      doing: colonnesCompletes.doing.length,
      done: colonnesCompletes.done.length,
    },
    frais,
  );

  return (
    <div className="projet">
      {/* FR-049 : dire depuis quand aucun signal n'a ete recu, mais
          seulement quand il y a quelque chose a en dire - un tableau frais
          n'a besoin d'aucune etiquette supplementaire (meme registre que
          l'accueil : le silence ne se signale que quand il est mauvais). */}
      {maintenant !== null && !frais && (
        <p className="muet-signal" role="status">
          Muet depuis {dureeDepuisSignal(scannedAt, dernierBattement, maintenant)}
        </p>
      )}

      <FormulaireBloc repoId={repoId} cheminsConnus={cheminsConnus} onCree={relire} />
      <FiltreType filtre={filtreType} onChoisir={setFiltreType} />

      {etatVide !== "normal" && (
        <EtatVideTableau etat={etatVide} frais={frais} />
      )}

      <div className="colonnes">
        {COLONNES_DANS_LORDRE.map((statut) => (
          <section key={statut} className={`colonne colonne-${statut}`} aria-label={TITRE_COLONNE[statut]}>
            <div className="colonne-tete">
              <h2 className="titre colonne-titre">{TITRE_COLONNE[statut]}</h2>
              {/* FR-047 : une exploration reste visible ci-dessous (la liste
                  entiere de la colonne), mais n'entre dans AUCUN total - ce
                  compteur l'exclut expressement. */}
              <span className="colonne-compte">{compteSansExploration(rangees[statut])}</span>
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
                    onRenomme={relire}
                    onSuppression={relire}
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
 * Les trois etats vides du tableau (F14, FR-050, FR-051, issue #39) - jamais
 * confondus : `lib/fraicheur.ts` (`etatTableau`) decide LEQUEL montrer, ce
 * composant ne fait qu'habiller ce choix d'un texte. Le dessin exact reste a
 * trancher en revue humaine (issue #43) ; cette version reste sobre et
 * s'appuie sur le registre deja en place (`.vide`, issue #29 - le meme cadre
 * en tirets que "aucun repo cartographie" ou "aucune machine").
 *
 * Les trois colonnes restent affichees en dessous, toujours toutes les
 * trois : ce bandeau ne les remplace jamais, il precede seulement leur
 * lecture d'un mot d'explication qu'elles ne portent pas seules.
 */
function EtatVideTableau({ etat, frais }: { etat: Exclude<EtatTableau, "normal">; frais: boolean }) {
  if (etat === "neuf") {
    return (
      <div className="vide vide-neuf" role="status">
        <p className="vide-titre">Ce tableau est encore vierge.</p>
        <p className="vide-suite">
          Une carte créée ici rejoint « À faire », avance en « En cours », se
          ferme en « Terminé » dès qu&apos;un commit la référence.
          {/* FR-051 : le clin d'oeil ne se montre que si la fraicheur le
              permet (issue #39) - le promettre sur un tableau qui ne peut
              pas prouver qu'il est vivant reviendrait a mentir une fois. */}
          {clinDoeilVisible(etat, frais) && (
            <>
              {" "}« Terminé » restera vide malgré l&apos;historique du dépôt :
              seul ce qui se passe à partir de maintenant y sera compté.
            </>
          )}
        </p>
      </div>
    );
  }

  if (etat === "acheve") {
    return (
      <div className="vide vide-acheve" role="status">
        <p className="vide-titre">Tout ce qui était suivi ici est terminé.</p>
        <p className="vide-suite">Aucune carte en attente ni en cours.</p>
      </div>
    );
  }

  // "muet" : FR-050 - le tableau a l'air aussi vide qu'un tableau acheve,
  // mais aucun signal recent ne permet de s'y fier. Ne jamais le presenter
  // comme un travail acheve : le dire franchement, comme un repo muet ailleurs
  // dans l'application (accueil.tsx, direct.tsx).
  return (
    <div className="vide vide-muet" role="status">
      <p className="vide-titre">Aucune carte en attente ni en cours - mais aucun signal récent non plus.</p>
      <p className="vide-suite">
        Impossible de garantir que ce dépôt est vraiment à jour tant qu&apos;il reste muet.
      </p>
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
 * d'ecran les recoivent gratuitement, sans geste dedie a coder. Le groupe se
 * tient par un tabindex glissant (#35, patron WAI-ARIA des barres d'outils) :
 * cinq boutons qui ne peuvent jamais etre actifs ensemble n'ont besoin que
 * d'un seul arret de Tab pour tout le groupe - les fleches font circuler ce
 * curseur, Entree/Espace restent le comportement natif du bouton pour
 * appliquer le filtre. Avant ce correctif, chaque bouton etait un arret
 * distinct : cinq `Tab` pour un seul choix, sur le chemin de la premiere
 * reference d'une carte que F11 veut le plus court possible.
 */
function FiltreType({
  filtre,
  onChoisir,
}: {
  filtre: TypeBloc | null;
  onChoisir: (type: TypeBloc | null) => void;
}) {
  const items: (TypeBloc | null)[] = [null, ...TYPES];
  const [indexFocalise, setIndexFocalise] = useState(() => {
    const i = items.indexOf(filtre);
    return i === -1 ? 0 : i;
  });
  const boutons = useRef<(HTMLButtonElement | null)[]>([]);

  function surTouche(e: React.KeyboardEvent<HTMLButtonElement>, index: number) {
    if (!estToucheFiltre(e.key)) return;
    // Empeche le defilement de la page sur Home/End, et la fleche d'agir
    // deux fois (une fois ici, une fois sur un eventuel autre gestionnaire).
    e.preventDefault();
    const suivant = indexSuivantFiltre(index, items.length, e.key);
    setIndexFocalise(suivant);
    boutons.current[suivant]?.focus();
  }

  return (
    <div className="filtre-type" role="group" aria-label="Filtrer par type">
      {items.map((type, index) => (
        <button
          key={type ?? "tous"}
          ref={(el) => {
            boutons.current[index] = el;
          }}
          type="button"
          className="filtre-bouton"
          aria-pressed={filtre === type}
          tabIndex={index === indexFocalise ? 0 : -1}
          onFocus={() => setIndexFocalise(index)}
          onKeyDown={(e) => surTouche(e, index)}
          onClick={() => onChoisir(type === null ? null : filtre === type ? null : type)}
        >
          {type === null ? "Tous" : LIBELLE_TYPE[type]}
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
  onRenomme,
  onSuppression,
}: {
  bloc: Bloc;
  issues: Issue[];
  ouverte: boolean;
  cheminsConnus: string[];
  onBasculer: () => void;
  onIssueCreee: () => void;
  onSortie: () => void;
  onRetype: () => void;
  onRenomme: () => void;
  onSuppression: () => void;
}) {
  const decoupe = estDecoupe(issues.length);
  const idPanneau = `issues-${bloc.id}`;
  // FR-048 : un bloc que le systeme a cree seul (agent, #38, ou PRD
  // brouillon, #36) reste renommable, retypable et supprimable - rien ne le
  // distingue d'un bloc saisi a la main. La retype (SelectionType,
  // FR-032) est deja ouverte a tout bloc ; renommer et supprimer, eux,
  // n'existaient pas encore avant #38 et n'ont de sens que pour un memo
  // (une exploration) : un bloc de vrai travail suivi (feature, correction,
  // technique) garde son seul geste existant, la sortie de Termine.
  const exploration = estExploration(bloc);

  return (
    <li className={`carte${bloc.prd_absent ? " carte-prd-absent" : ""}`}>
      <div className="carte-tete">
        {exploration ? (
          <TitreRenommable bloc={bloc} onRenomme={onRenomme} />
        ) : (
          <span className="carte-titre">{bloc.titre}</span>
        )}
        <BoutonCopierReference numeroRef={bloc.ref} titre={bloc.titre} classeTexte="carte-ref" />
      </div>
      {/* La provenance PRD (#37, FR-039 a FR-042) : sa propre ligne, jamais
          melangee a `.carte-meta` qui porte deja le type, l'emplacement (ou
          l'avancement) et les boutons - un bloc saisi a la main (`prd_cle`
          nul) n'affiche cette ligne nulle part, elle n'existe que pour ce
          qu'un PRD a vraiment produit. */}
      {bloc.prd_cle && (
        <div className="carte-prd">
          {/* FR-042 : la priorite se LIT ici, aucun geste de cette carte ne
              l'ecrit - ni un champ de saisie, ni un select, contrairement au
              type juste en dessous (FR-032, qui LUI se retype librement). */}
          {bloc.prd_priorite && <span className="carte-badge carte-priorite">{bloc.prd_priorite}</span>}
          <span className="carte-badge carte-origine" title={`Issue du PRD ${origineCourte(bloc.prd_cle)}`}>
            PRD {origineCourte(bloc.prd_cle)}
          </span>
          {bloc.prd_converti && (
            <span className="carte-badge carte-converti" title="Ce bloc portait deja un travail humain quand son PRD a ete valide : conserve plutot que retire (FR-039)">
              Convertie
            </span>
          )}
          {bloc.prd_absent && (
            <span className="carte-badge carte-absent" role="status">
              Retirée du PRD
            </span>
          )}
        </div>
      )}
      <div className="carte-meta">
        {/* Le type se lit et se retype au meme endroit (F10, FR-032) : un
            bloc decoupe se retype tout autant qu'un bloc simple - seul son
            STATUT est derive de ses issues (etat_bloc(), #30), jamais son
            type. Et une carte deja Terminee se retype aussi (FR-032,
            "a tout moment") : rien ici ne le distingue d'un bloc a faire, le
            serveur ne protege que `statut`, jamais `type` (#33). */}
        <SelectionType bloc={bloc} onRetype={onRetype} />
        {/* FR-047 : une exploration decoupee (#37 le permet, pour la marquer
            "non vierge" avant une conversion de PRD) n'affiche jamais son
            "X / Y" - ce serait la compter dans un reste a faire. */}
        {afficherAvancement(bloc, issues.length) ? (
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
        {blocPeutSortirDeTermine(bloc, issues.length) && (
          <BoutonSortieTermine table="blocs" id={bloc.id} libelle="Ramener en cours" onSortie={onSortie} />
        )}
        {/* FR-048 : la suppression d'un bloc d'exploration - le renommage vit
            dans le titre lui-meme (TitreRenommable), le retypage dans
            SelectionType juste au-dessus, deja ouvert a tout bloc. */}
        {exploration && <BoutonSupprimerExploration bloc={bloc} onSuppression={onSuppression} />}
      </div>

      {ouverte && (
        <div className="bloc-issues-panneau" id={idPanneau}>
          {issues.length > 0 && (
            <ul className="bloc-issues-liste">
              {issues.map((issue) => (
                <li key={issue.id} className={`bloc-issue bloc-issue-${issue.statut}`}>
                  <div className="bloc-issue-tete">
                    <span className="bloc-issue-titre">{issue.titre}</span>
                    <BoutonCopierReference
                      numeroRef={issue.ref}
                      titre={issue.titre}
                      classeTexte="bloc-issue-ref"
                    />
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
 * Le titre d'une exploration, renommable sur place (F13, FR-048). Un simple
 * `<span>` devient un champ de saisie au clic - pas de bouton "Renommer"
 * separe : le titre EST l'affordance, comme la reference EST deja le bouton
 * de copie (`BoutonCopierReference`). Reserve aux explorations : un bloc de
 * travail suivi (feature, correction, technique) n'a jamais eu ce geste et
 * F13 ne le lui demande pas.
 */
function TitreRenommable({ bloc, onRenomme }: { bloc: Bloc; onRenomme: () => void }) {
  const [enEdition, setEnEdition] = useState(false);
  const [valeur, setValeur] = useState(bloc.titre);
  const [enCours, setEnCours] = useState(false);
  const [echec, setEchec] = useState<string | null>(null);
  const champRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (enEdition) champRef.current?.focus();
  }, [enEdition]);

  function ouvrir() {
    setValeur(bloc.titre);
    setEchec(null);
    setEnEdition(true);
  }

  function annuler() {
    setValeur(bloc.titre);
    setEchec(null);
    setEnEdition(false);
  }

  async function valider(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!titreValide(valeur) || enCours) return;

    const titreTrim = valeur.trim();
    if (titreTrim === bloc.titre) {
      setEnEdition(false);
      return;
    }

    setEnCours(true);
    setEchec(null);
    const { error } = await createClient().from("blocs").update({ titre: titreTrim }).eq("id", bloc.id);
    setEnCours(false);

    if (error) {
      setEchec(error.message);
      return;
    }
    setEnEdition(false);
    onRenomme();
  }

  if (!enEdition) {
    return (
      <button type="button" className="carte-titre carte-titre-renommable" onClick={ouvrir}>
        {bloc.titre}
      </button>
    );
  }

  return (
    <form className="renommer-form" onSubmit={valider}>
      <label className="visually-hidden" htmlFor={`renommer-${bloc.id}`}>
        Nouveau titre de « {bloc.titre} »
      </label>
      <input
        ref={champRef}
        id={`renommer-${bloc.id}`}
        className="champ champ-renommer"
        type="text"
        value={valeur}
        onChange={(e) => setValeur(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") annuler();
        }}
        maxLength={200}
        disabled={enCours}
        required
      />
      <button type="submit" className="bouton bouton-discret" disabled={!titreValide(valeur) || enCours}>
        {enCours ? "…" : "OK"}
      </button>
      <button type="button" className="bouton bouton-discret" onClick={annuler} disabled={enCours}>
        Annuler
      </button>
      {echec && (
        <span className="echec" role="alert">
          {echec}
        </span>
      )}
    </form>
  );
}

/**
 * Supprime un bloc d'exploration (F13, FR-048) : un DELETE direct, protege
 * par une confirmation native - une exploration se supprime souvent (le PRD
 * l'attend meme, "moins d'une sur cinq" survit a la relecture), mais un
 * memo reste un artefact qu'on ne veut pas perdre a un clic malheureux.
 * Reservee aux explorations, meme raison que `TitreRenommable`.
 */
function BoutonSupprimerExploration({ bloc, onSuppression }: { bloc: Bloc; onSuppression: () => void }) {
  const [enCours, setEnCours] = useState(false);

  async function supprimer() {
    if (enCours) return;
    if (typeof window !== "undefined" && !window.confirm(`Supprimer « ${bloc.titre} » ?`)) return;

    setEnCours(true);
    const { error } = await createClient().from("blocs").delete().eq("id", bloc.id);
    setEnCours(false);
    if (!error) onSuppression();
  }

  return (
    <button type="button" className="bouton-supprimer" onClick={supprimer} disabled={enCours}>
      {enCours ? "Suppression…" : "Supprimer"}
    </button>
  );
}

/**
 * Copie un texte dans le presse-papier (F11, FR-034), avec repli quand
 * `navigator.clipboard` manque. Cette API exige un contexte securise
 * (HTTPS ou localhost) et n'existe pas dans tous les navigateurs ; plutot
 * que d'echouer en silence dans ce cas, on retombe sur `execCommand`, deprecie
 * mais encore largement supporte et - contrairement a la Clipboard API - pas
 * limite aux contextes securises. Si meme ce repli echoue (navigateur trop
 * ancien, ou API retiree), la fonction rend `false` et c'est l'appelant qui
 * le dit a l'utilisateur : il reste alors la selection manuelle a la main,
 * jamais un bouton qui prétend avoir réussi.
 */
async function copierTexte(texte: string): Promise<boolean> {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(texte);
      return true;
    } catch {
      // Un contexte non securise peut exposer l'API tout en refusant
      // l'ecriture (permission refusee) : on tombe au repli ci-dessous
      // plutot que de remonter un echec qui aurait peut-etre un recours.
    }
  }
  return copierViaSelectionCachee(texte);
}

/** Le repli historique : une zone de texte hors ecran, selectionnee puis
 *  copiee par `execCommand`. Fonctionne sans Clipboard API et sans contexte
 *  securise - exactement le cas que ce repli doit couvrir. */
function copierViaSelectionCachee(texte: string): boolean {
  if (typeof document === "undefined") return false;

  const zone = document.createElement("textarea");
  zone.value = texte;
  zone.setAttribute("readonly", "");
  // Hors ecran plutot que masquee : une zone `display: none` ne peut pas
  // etre selectionnee par certains navigateurs.
  zone.style.position = "fixed";
  zone.style.top = "-1000px";
  zone.style.left = "-1000px";
  document.body.appendChild(zone);
  zone.select();
  zone.setSelectionRange(0, texte.length);

  let succes = false;
  try {
    succes = document.execCommand("copy");
  } catch {
    succes = false;
  }
  document.body.removeChild(zone);
  return succes;
}

/**
 * Le bouton de copie d'une reference (F11, FR-034) : la reference affichee
 * EST le bouton, pas une icone separee qu'il faudrait deviner ou viser en
 * plus du texte. Partage entre la carte d'un bloc et le detail d'une issue -
 * meme geste, que la reference designe l'un ou l'autre (le compteur des deux
 * est le meme, cf. la conception du 2026-08-11 §4.2).
 *
 * Une copie reussie doit se voir (consigne #35) : le texte du bouton bascule
 * un instant sur « Copié », et une region `aria-live="polite"` annonce le
 * meme resultat pour qui ne le voit pas. Les deux sont necessaires - la
 * bascule visuelle ne suffit pas a un lecteur d'ecran, qui n'annonce pas
 * spontanement le changement de texte d'un bouton deja focalise.
 */
function BoutonCopierReference({
  numeroRef,
  titre,
  classeTexte,
}: {
  numeroRef: number;
  titre: string;
  classeTexte: string;
}) {
  const [etat, setEtat] = useState<"repos" | "copie" | "echec">("repos");
  const boutonRef = useRef<HTMLButtonElement>(null);

  async function copier() {
    const succes = await copierTexte(reference(numeroRef));
    setEtat(succes ? "copie" : "echec");
    // Le repli sans Clipboard API (`copierViaSelectionCachee`) passe par une
    // zone de texte hors ecran qui recoit brievement le focus pour pouvoir
    // etre selectionnee, puis est retiree du DOM aussitot copiee - et
    // retirer du DOM l'element focalise renvoie toujours le focus au
    // `<body>` (constate a l'usage, #35). Sans ce retour explicite, un
    // clavier qui vient de copier une reference se retrouve perdu en haut
    // de la page, sans plus aucun repere sur la carte qu'il consultait.
    boutonRef.current?.focus();
    // Le retour reste affiche assez longtemps pour se lire, y compris a la
    // vitesse d'un lecteur d'ecran, avant de redevenir un simple bouton.
    window.setTimeout(() => setEtat("repos"), 2500);
  }

  const texteAffiche =
    etat === "copie" ? "Copié" : etat === "echec" ? "Copie impossible" : reference(numeroRef);

  return (
    <span className="copier-ref">
      <button
        ref={boutonRef}
        type="button"
        className={`${classeTexte} ref-bouton ref-bouton-${etat}`}
        onClick={copier}
        aria-label={libelleCopier(numeroRef, titre)}
        data-ref={numeroRef}
      >
        {texteAffiche}
      </button>
      {/* Region annoncee par un lecteur d'ecran, toujours presente pour que le
          navigateur la surveille des le premier changement (une region
          ajoutee apres coup n'est pas garantie d'etre suivie). */}
      <span className="visually-hidden" role="status" aria-live="polite">
        {etat !== "repos" ? messageCopie(numeroRef, titre, etat === "copie") : ""}
      </span>
    </span>
  );
}

/**
 * Le geste de sortie de « Terminé » (F8, FR-025) : un PATCH direct qui pose
 * `doing`, jamais l'inverse - la base refuse de toute facon `done` depuis le
 * web (policy `update`, #33). Partagee entre un bloc simple et une issue :
 * la meme ecriture, sur l'une ou l'autre table.
 *
 * C'est aussi « le déplacement » de FR-035 (#35). Ce tableau n'a qu'un seul
 * mouvement manuel : celui-ci. Tout le reste avance tout seul (F3, F4) et le
 * glisser-deposer vers « Terminé » n'existe pas, ni au clavier ni a la
 * souris (hors scope global du PRD). Rendre « le déplacement » accessible au
 * clavier revient donc a rendre CE bouton accessible - un `<button>` natif
 * l'est deja, sans geste dedie a coder : Tab pour l'atteindre, Entree ou
 * Espace pour l'actionner.
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
  const refTitre = useRef<HTMLInputElement>(null);
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
      // Vider les champs desactive le bouton « Ajouter » (plus rien a
      // soumettre) : s'il portait le focus, le navigateur le renvoie au
      // `<body>` (constate a l'usage, #35 - un clavier qui vient de creer
      // une issue ne doit pas se retrouver sans repere). Reposer le focus
      // sur le titre a la fois corrige cela et prepare la saisie suivante.
      refTitre.current?.focus();
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
          ref={refTitre}
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
  const refTitre = useRef<HTMLInputElement>(null);
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
      // Meme raison que dans `FormulaireIssue` : vider le titre desactive
      // « Créer » (FR-035, #35) - sans ce retour explicite, le clavier qui
      // vient de creer un bloc atterrit sans repere au sommet de la page.
      refTitre.current?.focus();
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
          ref={refTitre}
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
