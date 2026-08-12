import Link from "next/link";
import { notFound } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Tableau } from "./tableau";
import type { Bloc } from "@/lib/blocs";
import type { Issue } from "@/lib/issues";

export default async function PageProjet({
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

  // La RLS suffit a garantir que ce repo appartient bien a cet utilisateur.
  // `scanned_at` et `machine_id` : les deux colonnes dont F14 derive la
  // fraicheur (issue #39, conception section 7) - rien de neuf en base.
  const { data: repo } = await supabase
    .from("repos")
    .select("id,name,remote_owner,current_branch,scanned_at,machine_id")
    .eq("id", id)
    .maybeSingle();

  if (!repo) notFound();

  // Le dernier battement de la machine qui porte ce repo, comme sur l'ecran
  // de plan (`page.tsx` du repo) : c'est le second signal dont F14 derive la
  // fraicheur, en plus de `repo.scanned_at`.
  const { data: machine } = await supabase
    .from("machines")
    .select("last_seen_at")
    .eq("id", repo.machine_id)
    .maybeSingle();

  const [blocs, issues, modules] = await Promise.all([
    supabase
      .from("blocs")
      .select(
        "id,ref,type,titre,statut,version,chemin,position,created_at,prd_cle,prd_priorite,prd_a_clarifier,prd_absent,prd_converti",
      )
      .eq("repo_id", id)
      .order("position", { ascending: true })
      .order("created_at", { ascending: true }),
    // Les issues d'un bloc decoupe (#30) : jamais rendues seules dans une
    // colonne (FR-021), elles ne servent qu'a l'avancement et au detail
    // ouvert depuis leur bloc.
    supabase
      .from("issues")
      .select("id,ref,bloc_id,titre,chemin,statut,version,position,created_at")
      .eq("repo_id", id)
      .order("position", { ascending: true })
      .order("created_at", { ascending: true }),
    // Les dossiers connus du depot : la source de l'autocompletion de
    // l'emplacement (FR-002). Le chemin vide designe la racine du depot, il
    // ne fait pas une suggestion utile.
    supabase.from("modules").select("path").eq("repo_id", id).neq("path", ""),
  ]);

  const cheminsConnus = (modules.data ?? [])
    .map((m) => m.path as string)
    .sort((a, b) => a.localeCompare(b));

  return (
    <main className="tableau large">
      <header className="entete">
        <Link className="lien" href={`/repo/${id}`}>
          ← {repo.name}
        </Link>
        <span className="compte">{user.email}</span>
      </header>

      <h1 className="titre-repo">
        {repo.remote_owner && <span className="proprietaire">{repo.remote_owner} /</span>}{" "}
        {repo.name}
        <span className="branche">projet</span>
      </h1>

      <p className="mesures">
        Le tableau de ce dépôt : ce qui reste à faire, ce qui avance, ce qui est fini.
      </p>

      <Tableau
        repoId={id}
        machineId={repo.machine_id}
        scannedAtInitial={repo.scanned_at}
        dernierBattementInitial={(machine?.last_seen_at as string | null) ?? null}
        blocsInitiaux={(blocs.data ?? []) as Bloc[]}
        issuesInitiales={(issues.data ?? []) as Issue[]}
        cheminsConnus={cheminsConnus}
      />
    </main>
  );
}
