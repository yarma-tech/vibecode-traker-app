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

/** Combien d'événements le journal reçoit au premier rendu. */
const LIGNES_DU_JOURNAL = 60;

export default async function PageRepo({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) notFound();

  // La RLS suffit à garantir que ce repo appartient bien à cet utilisateur.
  const { data: repo } = await supabase
    .from("repos")
    .select("id,name,remote_owner,current_branch,loc_total,file_count,scanned_at")
    .eq("id", id)
    .maybeSingle();

  if (!repo) notFound();

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

  return (
    <main className="tableau large">
      <header className="entete">
        <Link className="lien" href="/">
          ← tous les repos
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
        modules={(modules.data ?? []) as Module[]}
        locTotal={repo.loc_total}
        etatsInitiaux={(etats.data ?? []) as Etat[]}
        evenementsInitiaux={(evenements.data ?? []) as Evenement[]}
        conflitsInitiaux={(conflits.data ?? []) as Conflit[]}
        agentsInitiaux={(agents.data as number | null) ?? 0}
        releveInitial={(releve.data as Releve[] | null)?.[0] ?? null}
        worktreesInitiaux={(worktrees.data ?? []) as Worktree[]}
        fenetreSecondes={(fenetre.data as number | null) ?? 600}
      />
    </main>
  );
}
