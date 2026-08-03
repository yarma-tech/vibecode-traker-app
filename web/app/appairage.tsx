"use client";

import { useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";

type Code = { code: string; expires_at: string };

/** Minutes restantes avant peremption, arrondies vers le haut. */
function minutesRestantes(expires_at: string, maintenant: number): number {
  return Math.max(0, Math.ceil((Date.parse(expires_at) - maintenant) / 60000));
}

export function Appairage() {
  const [code, setCode] = useState<Code | null>(null);
  const [enCours, setEnCours] = useState(false);
  const [echec, setEchec] = useState<string | null>(null);
  const [maintenant, setMaintenant] = useState(() => Date.now());

  useEffect(() => {
    if (!code) return;
    const horloge = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => clearInterval(horloge);
  }, [code]);

  // Le code disparait de l'ecran des qu'une machine l'a consomme : la liste
  // juste au-dessus s'occupe d'afficher la nouvelle venue.
  useEffect(() => {
    if (!code) return;
    const supabase = createClient();
    const canal = supabase
      .channel("appairage")
      .on(
        "postgres_changes",
        {
          event: "UPDATE",
          schema: "public",
          table: "pairing_codes",
          filter: `code=eq.${code.code}`,
        },
        (message) => {
          if ((message.new as { consumed_at: string | null }).consumed_at) {
            setCode(null);
          }
        },
      )
      .subscribe();

    return () => {
      supabase.removeChannel(canal);
    };
  }, [code]);

  async function demanderUnCode() {
    setEnCours(true);
    setEchec(null);

    const { data, error } = await createClient().rpc("creer_code_appairage");

    if (error) setEchec(error.message);
    else setCode(data as Code);

    setEnCours(false);
  }

  if (!code) {
    return (
      <div className="appairage">
        <button className="bouton" onClick={demanderUnCode} disabled={enCours}>
          {enCours ? "Création du code…" : "Relier une machine"}
        </button>
        {echec && (
          <p className="echec" role="alert">
            {echec}
          </p>
        )}
      </div>
    );
  }

  const restant = minutesRestantes(code.expires_at, maintenant);

  return (
    <div className="appairage">
      <p className="etape">Sur la machine à relier :</p>

      <pre className="commande">
        <span className="invite">$</span> VIBEMAP_SUPABASE_URL=
        {process.env.NEXT_PUBLIC_SUPABASE_URL}
        {"\n"}
        {"  "}VIBEMAP_SUPABASE_ANON_KEY={process.env.NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY}
        {"\n"}
        {"  "}vibemap pair {code.code}
      </pre>

      <p className="attente">
        <span className="pastille" aria-hidden="true" />
        En attente d&apos;une machine. Ce code vaut encore {restant} min et ne
        sert qu&apos;une fois.
      </p>

      {/* Tant qu'il n'y a ni formule Homebrew ni binaire publie, `vibemap`
          n'existe dans aucun PATH. Le dire vaut mieux que de laisser
          l'utilisateur chercher une commande introuvable. */}
      <p className="pas-encore">
        <code>vibemap</code> n&apos;est pas encore distribué. En attendant,
        lance le binaire du projet :{" "}
        <code>./daemon/target/release/vibemap</code>
      </p>
    </div>
  );
}
