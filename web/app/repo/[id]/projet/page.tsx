import Link from "next/link";
import { notFound } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Tableau } from "./tableau";
import type { Bloc } from "@/lib/blocs";

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
  const { data: repo } = await supabase
    .from("repos")
    .select("id,name,remote_owner,current_branch")
    .eq("id", id)
    .maybeSingle();

  if (!repo) notFound();

  const [blocs, modules] = await Promise.all([
    supabase
      .from("blocs")
      .select("id,ref,type,titre,statut,version,chemin,position,created_at")
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

      <Tableau repoId={id} blocsInitiaux={(blocs.data ?? []) as Bloc[]} cheminsConnus={cheminsConnus} />
    </main>
  );
}
