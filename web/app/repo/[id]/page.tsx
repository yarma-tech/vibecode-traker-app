import Link from "next/link";
import { notFound } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import {
  Direct,
  type Conflit,
  type Etat,
  type Evenement,
  type Releve,
  type Worktree,
} from "./direct";
import { type Module } from "./plan";
import { SqueletteRepo } from "./squelette";
import { demoDemande } from "@/lib/ecrans";

/** Combien d'événements le journal reçoit au premier rendu. */
const LIGNES_DU_JOURNAL = 60;

export default async function PageRepo({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ demo?: string }>;
}) {
  const { id } = await params;
  const { demo } = await searchParams;
  const demoEcran = demoDemande(demo);

  // Écran de chargement atteignable en dev sans provoquer la vraie attente
  // (issue #12, critère 6). En vrai, c'est `loading.tsx` qui l'affiche.
  if (demoEcran === "chargement") {
    return <SqueletteRepo />;
  }

  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) notFound();

  // La RLS suffit à garantir que ce repo appartient bien à cet utilisateur.
  const { data: repo } = await supabase
    .from("repos")
    .select("id,name,remote_owner,current_branch,loc_total,file_count,scanned_at,machine_id")
    .eq("id", id)
    .maybeSingle();

  if (!repo) notFound();

  // Le dernier battement de la machine qui porte ce repo : c'est lui, comparé à
  // maintenant, qui décide si l'écran est figé. La base porte l'heure, l'écran
  // juge de l'âge.
  const { data: machine } = await supabase
    .from("machines")
    .select("last_seen_at")
    .eq("id", repo.machine_id)
    .maybeSingle();

  // La fenêtre d'activité est un réglage de la base : l'écran la lit, il ne la
  // décide pas. Ainsi une seule valeur gouverne les couleurs et ce qu'on en dit.
  const [modules, etats, evenements, conflits, agents, releve, worktrees, fenetre] =
    await Promise.all([
      supabase
        .from("modules")
        .select("path,parent_path,depth,loc,file_count")
        .eq("repo_id", id)
        .order("loc", { ascending: false }),
      supabase.rpc("etat_modules", { p_repo_id: id }),
      supabase
        .from("activity_events")
        .select("id,session_id,module_path,file_path,kind,occurred_at")
        .eq("repo_id", id)
        .order("occurred_at", { ascending: false })
        .limit(LIGNES_DU_JOURNAL),
      supabase.rpc("conflits", { p_repo_id: id }),
      supabase.rpc("agents_actifs", { p_repo_id: id }),
      supabase.rpc("releve_repo", { p_repo_id: id }),
      supabase.rpc("worktrees_ouverts", { p_repo_id: id }),
      supabase.rpc("fenetre_activite_secondes"),
    ]);

  // Repo sans activité, forcé en dev : on vide les canaux d'activité pour
  // montrer la géométrie pleine et sans couleur, sans toucher aux modules
  // (issue #12, critère 6). En vrai, un repo neuf est déjà dans cet état.
  const sansActivite = demoEcran === "sans-activite";
  const etatsInitiaux = sansActivite ? [] : ((etats.data ?? []) as Etat[]);
  const evenementsInitiaux = sansActivite ? [] : ((evenements.data ?? []) as Evenement[]);
  const conflitsInitiaux = sansActivite ? [] : ((conflits.data ?? []) as Conflit[]);
  const agentsInitiaux = sansActivite ? 0 : ((agents.data as number | null) ?? 0);
  const releveInitial = sansActivite ? null : ((releve.data as Releve[] | null)?.[0] ?? null);
  // En démonstration, on fait battre la machine à l'instant : l'écran sans
  // activité doit rester atteignable même si le vrai daemon s'est tu depuis
  // (sinon on basculerait en état gelé, un autre écran — issue #12, critère 6).
  const dernierBattementInitial = sansActivite
    ? new Date().toISOString()
    : ((machine?.last_seen_at as string | null) ?? null);

  return (
    <main className="tableau large">
      <header className="entete">
        <Link className="lien" href="/">
          ← tous les repos
        </Link>
        <Link className="lien lien-projet" href={`/repo/${id}/projet`}>
          espace projet →
        </Link>
        <span className="compte">{user.email}</span>
      </header>

      <h1 className="titre-repo">
        {repo.remote_owner && <span className="proprietaire">{repo.remote_owner} /</span>}{" "}
        {repo.name}
        {repo.current_branch && <span className="branche">{repo.current_branch}</span>}
      </h1>

      <p className="mesures">
        {(modules.data ?? []).length} dossiers ·{" "}
        {repo.loc_total.toLocaleString("fr-FR")} lignes ·{" "}
        {repo.file_count.toLocaleString("fr-FR")} fichiers · surface
        proportionnelle aux lignes
      </p>

      <Direct
        repoId={id}
        machineId={repo.machine_id}
        modules={(modules.data ?? []) as Module[]}
        locTotal={repo.loc_total}
        etatsInitiaux={etatsInitiaux}
        evenementsInitiaux={evenementsInitiaux}
        conflitsInitiaux={conflitsInitiaux}
        agentsInitiaux={agentsInitiaux}
        releveInitial={releveInitial}
        worktreesInitiaux={(worktrees.data ?? []) as Worktree[]}
        fenetreSecondes={(fenetre.data as number | null) ?? 600}
        dernierBattementInitial={dernierBattementInitial}
      />
    </main>
  );
}
