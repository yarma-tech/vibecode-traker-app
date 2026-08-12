import { describe, it, expect } from "vitest";
import { emplacementDisparu } from "./modules";

// F16 (issue #40) : signaler qu'un emplacement vise le vide (FR-059), sans
// jamais le corriger a la place de l'utilisateur (FR-060 - cette fonction ne
// fait que lire, elle n'ecrit jamais). Une jointure a la lecture entre le
// `chemin` deja charge sur un bloc ou une issue et le dernier `modules` connu
// du depot (conception du 2026-08-11, section 7 : "meme mecanisme que la
// fraicheur").

describe("emplacementDisparu — FR-059, le dossier d'un travail vivant a disparu", () => {
  it("signale un dossier qui n'est plus dans les modules connus", () => {
    // Le scenario nomme par l'issue : une issue ancree a un dossier renomme.
    expect(emplacementDisparu("web/app/hero", true, ["web/app/banniere"])).toBe(true);
  });

  it("ne signale rien quand le dossier est toujours dans les modules connus", () => {
    expect(emplacementDisparu("web/app/hero", true, ["web/app/hero", "web/app/banniere"])).toBe(
      false,
    );
  });

  it("ne signale jamais un travail termine — controle inverse de l'issue", () => {
    // Meme dossier disparu, mais le travail n'attend plus rien de son
    // emplacement : FR-059 ne parle que du travail vivant.
    expect(emplacementDisparu("web/app/hero", false, ["web/app/banniere"])).toBe(false);
  });

  it("ne signale jamais un bloc sans emplacement propre (decoupe en issues, #30)", () => {
    // `chemin === null` n'est jamais un proxy de quoi que ce soit ici : un
    // bloc decoupe n'a simplement plus rien a comparer (bloc_coherent() vide
    // sa colonne des la premiere issue). Ne pas confondre avec le piege deja
    // corrige en #37 ("chemin === null" comme approximation de "a des
    // issues") - ici on lit chemin pour ce qu'il est, jamais comme un signal
    // sur autre chose.
    expect(emplacementDisparu(null, true, ["web/app/banniere"])).toBe(false);
  });

  it("le chemin vide designe la racine : elle existe toujours", () => {
    expect(emplacementDisparu("", true, ["web/app/banniere"])).toBe(false);
    expect(emplacementDisparu("", true, [])).toBe(false);
  });

  it("un depot jamais cartographie n'a aucun module — silence, pas mensonge", () => {
    // Tout signaler ici confondrait "ca n'existe plus" et "on ne sait pas
    // encore" (meme piege que #39 pour la fraicheur) : un `modules` vide veut
    // dire soit qu'aucune cartographie n'a jamais eu lieu, soit qu'une
    // cartographie est EN COURS d'ecriture (le daemon efface `modules` avant
    // de le repeupler en entier, deux appels HTTP sans transaction entre eux -
    // `pousser_plan`, daemon/src/lib.rs).
    expect(emplacementDisparu("web/app/hero", true, [])).toBe(false);
  });

  it("un chemin de fichier ne se cherche jamais lui-meme dans les modules", () => {
    // Le piege central de F16 : `modules` ne liste que des dossiers
    // (migration 20260803000002). `creer_bloc` accepte pourtant la saisie
    // libre d'un chemin de fichier (FR-002) - "web/lib/blocs.ts" ne
    // trouvera donc JAMAIS sa propre valeur dans `modules`, meme vivant. On
    // verifie alors son DOSSIER PARENT : le fichier reste illisible
    // directement, mais son dossier, lui, est connu.
    expect(emplacementDisparu("web/lib/blocs.ts", true, ["web/lib"])).toBe(false);
  });

  it("signale un chemin de fichier dont meme le dossier parent a disparu", () => {
    expect(emplacementDisparu("web/lib/blocs.ts", true, ["web/app"])).toBe(true);
  });

  it("un dossier cache (point en tete) reste un dossier, pas un fichier", () => {
    // ".github" : `Path::extension()` cote daemon (plan.rs) rend `None` pour
    // un nom qui commence par un point sans second point - meme critere ici.
    expect(emplacementDisparu(".github", true, ["src"])).toBe(true);
    expect(emplacementDisparu(".github", true, [".github"])).toBe(false);
  });

  it("un fichier a la racine (parent vide) n'est jamais signale a tort", () => {
    // dossierParent("README.md") === "" : la racine existe toujours, comme
    // pour un chemin vide.
    expect(emplacementDisparu("README.md", true, [])).toBe(false);
    expect(emplacementDisparu("README.md", true, ["web"])).toBe(false);
  });

  it("compare insensible a la casse, comme suggestionsEmplacement (blocs.ts)", () => {
    // Un systeme de fichiers insensible a la casse (HFS+/APFS, par defaut
    // sur macOS) peut faire cohabiter une saisie utilisateur et la casse que
    // git a retenue sans que le dossier ait reellement bouge.
    expect(emplacementDisparu("Web/App", true, ["web/app"])).toBe(false);
    expect(emplacementDisparu("web/APP", true, ["Web/App"])).toBe(false);
  });
});
