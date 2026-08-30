import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { PUBLICATIONS } from "./premier-lancement";

/**
 * Le chemin par code ne doit plus avoir d'appelant côté web (FR-082, #83).
 *
 * POURQUOI un test qui lit les sources plutôt qu'un test de rendu : l'exigence
 * porte sur une ABSENCE, et une absence ne se prouve pas en regardant un écran
 * en particulier - il suffirait qu'un autre écran garde le bouton. La seule
 * façon honnête de la tenir est de balayer tout ce que `web/` sert, et de
 * vérifier que rien n'appelle la fonction de création ni ne lit la table des
 * codes. La table et la fonction restent en base : c'est leur dernier appelant
 * qui disparaît, pas elles (hors scope).
 */

const RACINE = fileURLToPath(new URL("..", import.meta.url));

/**
 * Tous les fichiers servis par l'application, sans les artefacts ni les tests.
 *
 * Les tests sont écartés parce qu'ils ne sont servis à personne, et parce que
 * celui-ci nomme forcément ce qu'il traque : sans cette exclusion, il tomberait
 * sur lui-même et ne dirait plus rien de l'application.
 */
function sources(dossier: string): string[] {
  const trouves: string[] = [];

  for (const entree of readdirSync(dossier, { withFileTypes: true })) {
    if (entree.name === "node_modules" || entree.name.startsWith(".")) continue;
    const chemin = join(dossier, entree.name);

    if (entree.isDirectory()) trouves.push(...sources(chemin));
    else if (/\.(ts|tsx|css)$/.test(entree.name) && !/\.test\.tsx?$/.test(entree.name)) {
      trouves.push(chemin);
    }
  }

  return trouves;
}

const FICHIERS = [...sources(join(RACINE, "app")), ...sources(join(RACINE, "lib"))].map(
  (chemin) => [chemin.slice(RACINE.length), readFileSync(chemin, "utf8")] as const,
);

describe("plus aucun appelant du chemin par code — FR-082", () => {
  it("les sources balayées ne sont pas vides : le test regarde bien quelque part", () => {
    expect(FICHIERS.length).toBeGreaterThan(20);
  });

  it("aucun écran ne demande la création d'un code d'appairage", () => {
    for (const [chemin, contenu] of FICHIERS) {
      expect(contenu, `${chemin} appelle encore creer_code_appairage`).not.toContain(
        "creer_code_appairage",
      );
    }
  });

  it("aucun écran ne lit la table des codes", () => {
    for (const [chemin, contenu] of FICHIERS) {
      expect(contenu, `${chemin} lit encore pairing_codes`).not.toContain("pairing_codes");
    }
  });

  it("aucun écran n'affiche la commande d'appairage du binaire", () => {
    for (const [chemin, contenu] of FICHIERS) {
      expect(contenu, `${chemin} affiche encore une commande d'appairage`).not.toContain(
        "vibemap pair",
      );
    }
  });
});

/**
 * Le fichier sans ses commentaires de bloc.
 *
 * Ce qui suit interroge ce que l'écran MONTRE. Un commentaire qui explique
 * pourquoi le code d'appairage a disparu nomme forcément ce qui a disparu :
 * l'y chercher ferait tomber le test sur l'explication de son propre sujet.
 */
function sansCommentaires(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, "");
}

/** L'écran de premier lancement, tel qu'il est servi. */
const PREMIER_LANCEMENT = sansCommentaires(
  readFileSync(join(RACINE, "app/premier-lancement.tsx"), "utf8"),
);
const TELECHARGEMENT = sansCommentaires(
  readFileSync(join(RACINE, "app/telechargement.tsx"), "utf8"),
);

describe("l'écran de premier lancement — FR-024, FR-025, FR-058", () => {
  it("porte le lien vers les publications, à un clic", () => {
    expect(TELECHARGEMENT).toContain("PUBLICATIONS");
    expect(TELECHARGEMENT).toMatch(/<a\b[^>]*href=\{PUBLICATIONS\}/);
    expect(PUBLICATIONS).toContain("/releases");
  });

  it("ne montre aucun bloc de commande à recopier", () => {
    for (const source of [PREMIER_LANCEMENT, TELECHARGEMENT]) {
      expect(source).not.toContain("<pre");
      expect(source).not.toMatch(/\bbrew\b/i);
      expect(source).not.toMatch(/VIBEMAP_SUPABASE/);
    }
  });

  it("ne parle plus d'un code", () => {
    for (const source of [PREMIER_LANCEMENT, TELECHARGEMENT]) {
      expect(source).not.toMatch(/\bcodes?\b/i);
    }
  });

  it("dit d'installer et d'ouvrir l'application de bureau", () => {
    const vu = `${PREMIER_LANCEMENT} ${TELECHARGEMENT}`;
    expect(vu).toMatch(/application de bureau/i);
    expect(vu).toMatch(/télécharger|télécharge/i);
  });
});
