"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { createClient } from "@/lib/supabase/client";
import { depuisTexte, estFige } from "@/lib/figement";

/**
 * Une ligne de l'apercu : tout ce qu'il faut pour peindre la bande d'etat d'un
 * repo, le trier et deduire son badge de compte. La base la rend deja triee ;
 * l'ecran ne re-trie pas, il ne recompte pas.
 */
export type Apercu = {
  id: string;
  name: string;
  remote_owner: string | null;
  /** @perso, @pro, ou rien : la correspondance se regle une fois dans les reglages. */
  compte: "perso" | "pro" | null;
  loc_total: number;
  file_count: number;
  agents: number;
  conflits: number;
  modules_ecrits: number;
  modules_lus: number;
  etat: "conflit" | "ecrit" | "lu" | "inactif";
  dernier_evenement: string | null;
  /** Dernier battement de la machine du repo, ou `null` si jamais vue. */
  derniere_presence: string | null;
};

type Filtre = "tous" | "perso" | "pro";

/** La fenetre finit par s'ecouler meme quand plus rien n'arrive : une relecture
 *  reguliere eteint les bandes restees allumees apres la fin du travail. */
const RELECTURE_MS = 15_000;

/** Ce que la bande d'etat dit, en un mot, a droite du nom. */
const DIT: Record<Apercu["etat"], string> = {
  conflit: "conflit",
  ecrit: "en cours",
  lu: "en lecture",
  inactif: "au repos",
};

function correspond(repo: Apercu, requete: string): boolean {
  // Sur le nom seul, insensible a la casse, sans correction de frappe : une
  // sous-chaine, rien de plus. Ni la description ni les topics n'entrent ici,
  // et pour cause : l'apercu ne les porte pas.
  return repo.name.toLowerCase().includes(requete);
}

export function Accueil({ initiaux }: { initiaux: Apercu[] }) {
  const router = useRouter();
  const [repos, setRepos] = useState(initiaux);
  const [requete, setRequete] = useState("");
  const [filtre, setFiltre] = useState<Filtre>("tous");
  const [vise, setVise] = useState(0);
  const champRef = useRef<HTMLInputElement>(null);

  // L'horloge ne démarre qu'après le montage, comme ailleurs : rendue sur le
  // serveur, elle ne correspondrait pas à celle du navigateur. Tant qu'elle
  // vaut `null`, aucune machine n'est déclarée muette : le premier rendu reste
  // celui du serveur. Ensuite elle avance seule, pour que la bascule en muet se
  // produise au fil du temps, même sans nouvel événement.
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
    const { data } = await createClient().rpc("apercu_repos");
    if (data) setRepos(data as Apercu[]);
  }, []);

  // Le temps reel n'est qu'un signal : sa charge utile arrive incomplete. On
  // relit donc l'apercu par la fonction, qui passe par la RLS et rend tout.
  useEffect(() => {
    const supabase = createClient();

    const canal = supabase
      .channel("accueil")
      .on("postgres_changes", { event: "*", schema: "public", table: "activity_events" }, relire)
      .on("postgres_changes", { event: "*", schema: "public", table: "sessions" }, relire)
      .on("postgres_changes", { event: "*", schema: "public", table: "repos" }, relire)
      .on("postgres_changes", { event: "*", schema: "public", table: "account_mappings" }, relire)
      // Le battement des machines passe par `machines` : c'est ce signal qui
      // ranime une bande figée dès que le daemon repart, sans rechargement.
      .on("postgres_changes", { event: "*", schema: "public", table: "machines" }, relire)
      .subscribe();

    const horloge = setInterval(relire, RELECTURE_MS);

    return () => {
      supabase.removeChannel(canal);
      clearInterval(horloge);
    };
  }, [relire]);

  // ⌘K (ou Ctrl+K) pose le curseur dans le champ, d'ou qu'on parte. C'est le
  // geste attendu d'une recherche : on n'a pas a viser le champ a la souris.
  useEffect(() => {
    function surTouche(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        champRef.current?.focus();
        champRef.current?.select();
      }
    }
    window.addEventListener("keydown", surTouche);
    return () => window.removeEventListener("keydown", surTouche);
  }, []);

  const terme = requete.trim().toLowerCase();
  // Ce qui repond au nom cherche, tous comptes confondus.
  const nommes = terme ? repos.filter((r) => correspond(r, terme)) : repos;
  // Ce que le filtre de compte laisse passer.
  const visibles = filtre === "tous" ? nommes : nommes.filter((r) => r.compte === filtre);
  // Ce que le filtre de compte ecarte, alors meme que le nom correspond : c'est
  // ce qu'on nommera si la recherche ne rend rien.
  const ecartes = filtre === "tous" ? [] : nommes.filter((r) => r.compte !== filtre);

  const selection = Math.min(vise, Math.max(0, visibles.length - 1));

  function surRequete(valeur: string) {
    setRequete(valeur);
    setVise(0);
  }

  function surFiltre(f: Filtre) {
    setFiltre(f);
    setVise(0);
  }

  function surTouche(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setVise((v) => Math.min(v + 1, visibles.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setVise((v) => Math.max(v - 1, 0));
    } else if (e.key === "Enter") {
      const cible = visibles[selection];
      if (cible) {
        e.preventDefault();
        router.push(`/repo/${cible.id}`);
      }
    }
  }

  if (repos.length === 0) {
    return (
      <div className="vide">
        <p className="vide-titre">Aucun repo cartographié.</p>
        <p className="vide-suite">
          <code>vibemap</code> explore les dossiers listés dans sa configuration
          toutes les cinq minutes. Le premier passage a lieu dès son démarrage.
        </p>
      </div>
    );
  }

  return (
    <div className="accueil-repos">
      <div className="barre">
        <div className="champ-recherche">
          <span className="champ-loupe" aria-hidden="true">
            ⌘K
          </span>
          <input
            ref={champRef}
            className="champ"
            type="search"
            value={requete}
            onChange={(e) => surRequete(e.target.value)}
            onKeyDown={surTouche}
            placeholder="Chercher un repo par son nom…"
            aria-label="Chercher un repo par son nom"
            autoComplete="off"
            spellCheck={false}
          />
        </div>

        <div className="filtre-compte" role="group" aria-label="Filtrer par compte">
          {(["tous", "perso", "pro"] as const).map((f) => (
            <button
              key={f}
              type="button"
              className={filtre === f ? "onglet actif" : "onglet"}
              aria-pressed={filtre === f}
              onClick={() => surFiltre(f)}
            >
              {f === "tous" ? "tous" : `@${f}`}
            </button>
          ))}
        </div>
      </div>

      {visibles.length === 0 ? (
        <div className="vide sans-resultat" role="status">
          {ecartes.length > 0 ? (
            <>
              <p className="vide-titre">
                Aucun repo <b>@{filtre}</b> ne répond à «&nbsp;{requete.trim()}&nbsp;».
              </p>
              <p className="vide-suite">
                {ecartes.length === 1 ? (
                  <>
                    <code>{ecartes[0].name}</code> correspond, mais il est classé{" "}
                    <b>@{ecartes[0].compte}</b>, que ton filtre écarte.
                  </>
                ) : (
                  <>
                    {ecartes.map((r, i) => (
                      <span key={r.id}>
                        {i > 0 && (i === ecartes.length - 1 ? " et " : ", ")}
                        <code>{r.name}</code>
                      </span>
                    ))}{" "}
                    correspondent, mais ton filtre <b>@{filtre}</b> les écarte.
                  </>
                )}
              </p>
            </>
          ) : (
            <>
              <p className="vide-titre">Aucun repo ne répond à «&nbsp;{requete.trim()}&nbsp;».</p>
              <p className="vide-suite">
                La recherche porte sur le nom seul&nbsp;: ni la description, ni les
                sujets.
              </p>
            </>
          )}
        </div>
      ) : (
        <ul className="rangs">
          {visibles.map((repo, i) => {
            // Muette : la machine se tait au-delà du seuil. L'état d'activité
            // (« en cours », « au repos »…) devient alors une photo périmée ;
            // le rang le dit franchement — muette, et depuis quand — au
            // lieu de laisser croire à un direct. C'est ce qui distingue une
            // machine figée d'un repo simplement au repos (machine à jour).
            const muette = maintenant !== null && estFige(repo.derniere_presence, maintenant);

            return (
              <li
                key={repo.id}
                className={
                  `${i === selection ? "rang vise" : "rang"}${muette ? " muette" : ""}`
                }
                aria-current={i === selection ? "true" : undefined}
              >
                <span
                  className={`bande ${muette ? "muette" : repo.etat}`}
                  aria-hidden="true"
                />
                <Link className="rang-lien" href={`/repo/${repo.id}`}>
                  {repo.remote_owner && <span className="proprietaire">{repo.remote_owner} /</span>}
                  <span className="repo-nom">{repo.name}</span>
                </Link>
                {repo.compte && <span className={`badge ${repo.compte}`}>@{repo.compte}</span>}
                {muette ? (
                  <span className="dit-etat muette">
                    muette
                    <span className="agents">
                      {" · "}
                      {maintenant !== null ? depuisTexte(repo.derniere_presence, maintenant) : ""}
                    </span>
                  </span>
                ) : (
                  <span className={`dit-etat ${repo.etat}`}>
                    {repo.etat === "conflit" && <span aria-hidden="true">⚠ </span>}
                    {DIT[repo.etat]}
                    {repo.agents > 0 && (
                      <span className="agents">
                        {" · "}
                        {repo.agents} agent{repo.agents > 1 ? "s" : ""}
                      </span>
                    )}
                  </span>
                )}
                <span className="rang-poids">{repo.loc_total.toLocaleString("fr-FR")} lignes</span>
                <Link className="rang-projet" href={`/repo/${repo.id}/projet`}>
                  projet
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
