import Link from "next/link";
import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { Deconnexion } from "../deconnexion";
import { DerniereCartographie } from "./cartographie";
import { Comptes, type Correspondance } from "./comptes";
import { DossiersSurveilles } from "./dossiers";

export default async function PageReglages() {
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) redirect("/");

  // Les proprietaires distincts des remotes, tels que le daemon les a poses.
  // C'est sur eux que porte le classement : un owner, une fois, pour tous ses
  // repos. La RLS borne la lecture aux repos de l'utilisateur.
  //
  // `scanned_at` vient de la meme lecture : l'heure de la derniere cartographie
  // est en base, avec le catalogue, et pas sur le pont (FR-059). Chaque depot
  // porte la sienne ; l'ecran en fait un agregat (FR-086).
  const { data: repos } = await supabase.from("repos").select("remote_owner,scanned_at");

  const { data: correspondances } = await supabase
    .from("account_mappings")
    .select("owner,label");

  const heures = (repos ?? []).map((r) => r.scanned_at as string | null);

  const owners = Array.from(
    new Set((repos ?? []).map((r) => r.remote_owner).filter((o): o is string => Boolean(o))),
  ).sort((a, b) => a.localeCompare(b, "fr"));

  return (
    <main className="tableau">
      <header className="entete">
        <Link className="lien" href="/">
          ← tous les repos
        </Link>
        <span className="compte">{user.email}</span>
        <Deconnexion />
      </header>

      <h2 className="titre">Comptes</h2>

      <p className="etape">
        À qui appartient chaque propriétaire de remote&nbsp;? Le classement se
        règle une fois ici&nbsp;; il donne ensuite le badge <b>@perso</b> ou{" "}
        <b>@pro</b>{" "}
        de chaque repo sur l&apos;accueil. Rien n&apos;est déduit d&apos;un appel
        à GitHub&nbsp;: seul le propriétaire du remote compte.
      </p>

      {owners.length === 0 ? (
        <div className="vide">
          <p className="vide-titre">Aucun propriétaire de remote pour l&apos;instant.</p>
          <p className="vide-suite">
            Dès qu&apos;un repo relié aura un remote <code>origin</code>, son
            propriétaire apparaîtra ici, prêt à être classé.
          </p>
        </div>
      ) : (
        <Comptes
          owners={owners}
          initiales={(correspondances ?? []) as Correspondance[]}
        />
      )}

      <h2 className="titre">Dossiers surveillés</h2>

      <p className="etape">
        Où cette machine va chercher des dépôts, et ce qu&apos;elle y a trouvé.
        Un dépôt est un enfant <b>direct</b>{" "}
        d&apos;un de ces dossiers&nbsp;: rien de plus profond n&apos;est cartographié.
      </p>

      <DerniereCartographie heures={heures} />

      <DossiersSurveilles />
    </main>
  );
}
