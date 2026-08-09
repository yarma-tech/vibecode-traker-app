import { SqueletteRepo } from "./squelette";

/**
 * L'écran de chargement d'un repo (issue #12). Next l'affiche d'emblée pendant
 * que la page serveur va chercher ses données, puis échange le contenu une fois
 * prêt. Le squelette conserve les proportions réelles des parcelles (via le
 * cache local) : rien ne saute à l'arrivée des données.
 */
export default function Chargement() {
  return <SqueletteRepo />;
}
