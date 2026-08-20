"use client";

/**
 * Entrer avec GitHub (issue #63, FR-012, FR-013, FR-016, FR-071, FR-072).
 *
 * Le même écran sert les deux cas, et c'est le pont qui les sépare :
 *
 * - dans un navigateur ordinaire, la page part elle-même vers GitHub, comme
 *   n'importe quel site ;
 * - dans la fenêtre de l'application, elle ne part pas. GitHub refuse les vues
 *   embarquées, et la fenêtre refuse de son côté toute origine étrangère : une
 *   navigation vers `github.com` ne ferait rien du tout, en silence. L'adresse
 *   d'autorisation est donc demandée à Supabase sans partir, puis ouverte dans
 *   le navigateur du système par l'application.
 *
 * Le retour ne revient pas ici : il revient au navigateur du système, qui n'a
 * pas de quoi finir l'échange. Le code y est repris pour la fenêtre, qui vient
 * le chercher au relais et l'échange avec son vérificateur PKCE - puis demande
 * à l'application de repasser au premier plan (FR-072).
 *
 * Ce que cet écran DÉCIDE ne vit pas ici : `lib/autorisation.ts` le porte, et
 * ce module ne fait que l'exécuter. C'est ce qui rend le chemin éprouvable
 * sans GitHub.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import { pont } from "./pont";
import {
  AUTORISATION_JAMAIS_REVENUE,
  attenteExpiree,
  departAutorisation,
  ecranDeConnexion,
  repriseDeLaFenetre,
  type PhaseConnexion,
} from "@/lib/autorisation";

/** Le rythme auquel la fenêtre va voir si le retour est arrivé. */
const CADENCE_DU_GUET_MS = 700;

type OuvertureDehors = { etat: "ouverte" } | { etat: "refusee"; raison: string };

export function Connexion({ erreur }: { erreur?: string }) {
  const router = useRouter();
  const [phase, setPhase] = useState<PhaseConnexion>("repos");
  const [echec, setEchec] = useState<string | null>(erreur ?? null);

  /** Quand l'attente a commencé, pour savoir quand cesser de guetter. */
  const debutDeLAttente = useRef(0);

  const echouer = useCallback((raison: string) => {
    setEchec(raison);
    setPhase("repos");
  }, []);

  async function seConnecter() {
    setPhase("ouverture");
    setEchec(null);

    const appel = pont();
    const supabase = createClient();

    // Une session déjà ouverte ne se redemande pas (FR-014) : la carte est
    // derrière, il suffit de la laisser s'afficher.
    const {
      data: { session },
    } = await supabase.auth.getSession();

    const depart = departAutorisation({
      sessionOuverte: session !== null,
      dansLApplication: appel !== null,
      origineDeLaPage: window.location.origin,
    });

    if (depart.quoi === "deja_connecte") {
      router.refresh();
      return;
    }

    const dehors = depart.quoi === "au_navigateur_du_systeme";
    const { data, error } = await supabase.auth.signInWithOAuth({
      provider: "github",
      options: { redirectTo: depart.retour, skipBrowserRedirect: dehors },
    });

    if (error) {
      echouer(error.message);
      return;
    }

    // Navigateur ordinaire : Supabase a déjà emmené la page. Il n'y a rien à
    // attendre ici, et la phase reste celle de l'ouverture le temps que la
    // navigation se fasse.
    if (!dehors) return;

    if (!data?.url || !appel) {
      echouer("aucune adresse d'autorisation n'a été rendue.");
      return;
    }

    const ouverture = (await appel("ouvrir_l_autorisation", {
      url: data.url,
    })) as OuvertureDehors;

    if (ouverture?.etat !== "ouverte") {
      echouer(ouverture?.raison ?? "le navigateur du système n'a pas pu être ouvert.");
      return;
    }

    debutDeLAttente.current = Date.now();
    setPhase("attente");
  }

  // Le guet du retour. Il ne tourne QUE pendant l'attente : hors de
  // l'application, et tant que personne n'a rien demandé, ce chemin n'existe
  // pas et rien n'est appelé.
  useEffect(() => {
    if (phase !== "attente") return;

    let vivant = true;

    async function guetter() {
      let reponse: unknown = null;
      try {
        reponse = await (await fetch("/auth/reprise", { cache: "no-store" })).json();
      } catch {
        // Le service d'interface est celui qui sert cette page : une lecture
        // ratée est un accident de passage, pas un échec d'autorisation. On
        // regardera au tour suivant, et l'échéance finira par trancher.
        reponse = null;
      }
      if (!vivant) return;

      const repris = repriseDeLaFenetre(reponse);

      if (repris.quoi === "refus") {
        echouer(repris.raison);
        return;
      }

      if (repris.quoi === "rien") {
        if (attenteExpiree(debutDeLAttente.current, Date.now())) {
          echouer(AUTORISATION_JAMAIS_REVENUE);
        }
        return;
      }

      // Le code, enfin. C'est ici, et nulle part ailleurs, qu'il peut être
      // échangé : le vérificateur PKCE est dans les cookies de cette fenêtre.
      const supabase = createClient();
      const { error } = await supabase.auth.exchangeCodeForSession(repris.code);
      if (!vivant) return;

      if (error) {
        echouer(error.message);
        return;
      }

      // FR-072 : l'application reprend la main dans sa propre fenêtre. Le
      // navigateur du système est au premier plan à cet instant, et personne
      // ne doit avoir à revenir chercher Vibe Map dans le Dock.
      await pont()?.("revenir_au_premier_plan");

      // La session est posée dans les cookies : l'accueil, relu côté serveur,
      // rend la carte. Pas d'étape de plus (FR-013).
      router.refresh();
    }

    const battement = setInterval(guetter, CADENCE_DU_GUET_MS);
    guetter();

    return () => {
      vivant = false;
      clearInterval(battement);
    };
  }, [phase, echouer, router]);

  const ecran = ecranDeConnexion(phase, echec);

  return (
    <div className="accueil">
      <h1>Vibe Map</h1>
      <p className="pitch">
        Une carte de tes repos qui s&apos;allume dès qu&apos;un agent se met au
        travail. Ton code reste chez toi : seules les couleurs voyagent.
      </p>

      <button className="bouton" onClick={seConnecter} disabled={!ecran.bouton.actif}>
        {ecran.bouton.texte}
      </button>

      {ecran.attente && <p className="pitch">{ecran.attente}</p>}

      {ecran.echec && (
        <p className="echec" role="alert">
          {ecran.echec}
        </p>
      )}
    </div>
  );
}
