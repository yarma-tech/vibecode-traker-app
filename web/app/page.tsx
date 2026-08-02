import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Connexion } from "./connexion";
import { Machines, type Machine } from "./machines";
import { Deconnexion } from "./deconnexion";

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ erreur?: string; code?: string }>;
}) {
  const { erreur, code } = await searchParams;

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
    .select("id,label,platform,last_seen_at")
    .order("label");

  return (
    <main className="tableau">
      <header className="entete">
        <span className="marque">Vibe Map</span>
        <span className="compte">{user.email}</span>
        <Deconnexion />
      </header>

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
