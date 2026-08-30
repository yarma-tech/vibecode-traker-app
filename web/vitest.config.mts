import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

/**
 * Les tests lisent `@/…` comme le reste de l'interface.
 *
 * Sans cet alias, seuls les modules qui s'importent par chemin relatif
 * seraient éprouvables - c'est-à-dire les fonctions pures de `lib/`, et jamais
 * un gestionnaire de route, qui importe forcément par `@/`. L'échangeur
 * d'autorisation (issue #63) en est un.
 */
export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL(".", import.meta.url)),
    },
  },
});
