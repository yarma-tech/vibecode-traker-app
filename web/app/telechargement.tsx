import { PUBLICATIONS } from "@/lib/premier-lancement";

/**
 * L'encart de téléchargement du premier lancement (issue #83, FR-058).
 *
 * Ce fichier portait le code d'appairage et la commande à recopier. Les deux
 * ont disparu du parcours (FR-023, FR-024) : ce qu'il reste à offrir à un
 * compte sans machine, c'est un seul geste - aller chercher l'application.
 *
 * Un lien, et un seul : la page des publications du dépôt, atteignable en un
 * clic. `rel="noreferrer"` parce qu'un lien qui sort n'a rien à dire d'où il
 * vient.
 */
export function Telechargement() {
  return (
    <div className="telechargement">
      <p className="telechargement-quoi">
        <b>Vibe Map pour macOS</b>
        <span className="telechargement-precision">Apple Silicon</span>
      </p>

      <a
        className="bouton telechargement-lien"
        href={PUBLICATIONS}
        target="_blank"
        rel="noreferrer"
      >
        Télécharger l&apos;application
      </a>

      <p className="telechargement-suite">
        Ouvre-la une fois installée&nbsp;: elle relie cette machine à ton compte
        et cette page s&apos;allume d&apos;elle-même.
      </p>
    </div>
  );
}
