import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Connexion } from "./connexion";
import { Machines, type Machine } from "./machines";
import { Accueil, type Apercu } from "./accueil";
import { Deconnexion } from "./deconnexion";
import { Appairage } from "./appairage";
import { PremierLancement } from "./premier-lancement";
import { demoDemande, montrerPremierLancement } from "@/lib/ecrans";
import Link from "next/link";

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
  } = await supabase.auth.getUser();

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
  const { data: repos } = await supabase.rpc("apercu_repos");

  const machines = (data ?? []) as Machine[];
  const demoEcran = demoDemande(demo);

  // Premier lancement : tant qu'aucune machine n'est appairée, l'accueil cède la
  // place à l'onboarding, qui explique le produit et se remplit tout seul dès
  // qu'une machine répond (issue #12). `?demo=onboarding` le force en dev.
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

      <Appairage />
    </main>
  );
}
