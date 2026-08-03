import Link from "next/link";
import { notFound } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Plan, type Module } from "./plan";

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

  const { data: modules } = await supabase
    .from("modules")
    .select("path,parent_path,depth,loc,file_count")
    .eq("repo_id", id)
    .order("loc", { ascending: false });

  return (
    <main className="tableau">
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
        {(modules ?? []).length} dossiers · {repo.loc_total.toLocaleString("fr-FR")} lignes ·{" "}
        {repo.file_count.toLocaleString("fr-FR")} fichiers · surface
        proportionnelle aux lignes
      </p>

      <Plan modules={(modules ?? []) as Module[]} locTotal={repo.loc_total} />
    </main>
  );
}
