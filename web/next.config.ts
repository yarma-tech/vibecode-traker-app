import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // En developpement, Next ne sert ses ressources internes qu'a l'hote qui l'a
  // demarre (localhost). Sans cette ligne, une visite sur 127.0.0.1 voit le
  // bundle client bloque : la page s'affiche mais ne s'hydrate pas, et les
  // boutons ne repondent pas, sans aucune erreur visible.
  allowedDevOrigins: ["127.0.0.1"],
};

export default nextConfig;
