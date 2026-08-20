import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Connexion } from "./connexion";
import { Machines, type Machine } from "./machines";
import { Accueil, type Apercu } from "./accueil";
import { Deconnexion } from "./deconnexion";
import { DeclarationDeLaMachine } from "./declaration";
import { PremierLancement } from "./premier-lancement";
import { BaseInjoignable } from "./base-injoignable";
import {
  baseInjoignable,
  demoDemande,
  montrerPremierLancement,
  premiereInjoignable,
  raisonInjoignable,
} from "@/lib/ecrans";
import Link from "next/link";

/**
 * L'accueil quand la base ne répond pas. La fenêtre garde son entête - elle est
 * servie depuis la machine, elle ne dépend de rien - et ne porte plus que
 * l'annonce et son recours : ni liste de repos vide, ni squelette qui attend.
 * Ni lien vers les réglages ni bouton de déconnexion, qui passent tous deux par
 * la base : n'offrir que ce qui peut aboutir.
 */
function EcranInjoignable({ raison, compte }: { raison: string; compte?: string }) {
  return (
    <main className="tableau">
      <header className="entete">
        <span className="marque">Vibe Map</span>
        {compte && <span className="compte">{compte}</span>}
      </header>

      <BaseInjoignable raison={raison} />
    </main>
  );
}

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ erreur?: string; code?: string; demo?: string }>;
}) {
  const { erreur, code, demo } = await searchParams;

  // Filet de securite : si la liste blanche de Supabase change et qu'un code
  // d'autorisation atterrit ici, on l'emmene a l'echangeur plutot que
  // d'afficher un ecran de connexion qui ne dit rien de ce qui s'est passe.
  if (code) {
    redirect(`/auth/callback?code=${encodeURIComponent(code)}`);
  }

  const supabase = await createClient();

  const {
    data: { user },
    error: erreurAuth,
  } = await supabase.auth.getUser();

  // Sans réponse de la base, on ne sait pas s'il y a une session : on sait
  // seulement qu'on n'a pas pu le demander. Le dire, plutôt que de conclure
  // « personne » et de présenter un écran de connexion qui ne mènerait nulle
  // part (FR-085).
  if (baseInjoignable(erreurAuth)) {
    return <EcranInjoignable raison={raisonInjoignable(erreurAuth)} />;
  }

  if (!user) {
    return <Connexion erreur={erreur} />;
  }

  // La RLS fait le tri : cette requete ne peut rendre que les machines
  // de l'utilisateur connecte, meme si elle ne le precise pas.
  const { data, error } = await supabase
    .from("machines")
    .select("id,label,platform,last_seen_at,revoked_at")
    .order("label");

  // L'apercu rend tous les repos deja tries par activite, avec leur bande
  // d'etat et leur badge de compte : l'ecran lit cette seule fonction.
  const { data: repos, error: erreurRepos } = await supabase.rpc("apercu_repos");

  const machines = (data ?? []) as Machine[];
  const demoEcran = demoDemande(demo);

  // Il suffit qu'une des deux lectures soit restée sans réponse : le compte de
  // machines vaudrait alors zéro sans que personne ne l'ait dit, et l'accueil
  // basculerait en premier lancement chez un utilisateur qui en a trois.
  const sansReponse = premiereInjoignable([error, erreurRepos]);
  if (demoEcran === "injoignable" || sansReponse) {
    return <EcranInjoignable raison={raisonInjoignable(sansReponse)} compte={user.email} />;
  }

  // Premier lancement : tant qu'aucune machine n'est déclarée, l'accueil cède la
  // place à l'onboarding, qui dirige vers l'application de bureau et se remplit
  // tout seul dès qu'une machine répond (issue #12, #83). `?demo=onboarding` le
  // force en dev.
  if (montrerPremierLancement(machines.length, demoEcran)) {
    return (
      <main className="tableau">
        <header className="entete">
          <span className="marque">Vibe Map</span>
          <Link className="lien" href="/reglages">
            réglages
          </Link>
          <span className="compte">{user.email}</span>
          <Deconnexion />
        </header>

        {/* La machine se presente a chaque ouverture (FR-055) : c'est ici, et
            non a la connexion, parce qu'un lancement ordinaire ne repasse
            jamais par l'ecran de connexion (FR-014). Sur un Mac neuf, c'est
            elle qui fait apparaitre la premiere machine sous cet ecran. */}
        <DeclarationDeLaMachine />

        <PremierLancement demo={demoEcran === "onboarding"} />
      </main>
    );
  }

  return (
    <main className="tableau">
      <header className="entete">
        <span className="marque">Vibe Map</span>
        <Link className="lien" href="/reglages">
          réglages
        </Link>
        <span className="compte">{user.email}</span>
        <Deconnexion />
      </header>

      <DeclarationDeLaMachine />

      <h2 className="titre">Repos</h2>

      <Accueil initiaux={(repos ?? []) as Apercu[]} />

      <h2 className="titre">Machines</h2>

      {error ? (
        <p className="echec" role="alert">
          Impossible de lire les machines : {error.message}
        </p>
      ) : (
        <Machines initiales={(data ?? []) as Machine[]} />
      )}
    </main>
  );
}
