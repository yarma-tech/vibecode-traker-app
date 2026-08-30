import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // En developpement, Next ne sert ses ressources internes qu'a l'hote qui l'a
  // demarre (localhost). Sans cette ligne, une visite sur 127.0.0.1 voit le
  // bundle client bloque : la page s'affiche mais ne s'hydrate pas, et les
  // boutons ne repondent pas, sans aucune erreur visible.
  allowedDevOrigins: ["127.0.0.1"],

  // La sortie autonome (FR-069) : elle depose dans `.next/standalone` un
  // serveur complet, ses dependances tracees comprises, qui se lance par un
  // simple `node server.js` sans `node_modules` autour. C'est ce dossier que
  // l'application de bureau embarque dans son paquet ; sans lui, il faudrait y
  // recopier tout `node_modules` et un `npm`.
  //
  // POURQUOI seulement sur demande : cette sortie est faite pour etre lancee
  // par son `server.js`, et Next avertit que `next start` « ne fonctionne pas »
  // avec elle. Or `next start` est exactement ce que sert le site heberge, et
  // ce que lance l'application depuis le depot en developpement. La reclamer
  // toujours, ce serait faire dire a Next le contraire de ce qu'on fait le
  // reste du temps. Seul `bureau/embarquer-le-service.sh` pose cette variable.
  output: process.env.VIBEMAP_SERVICE_EMBARQUE ? "standalone" : undefined,

  // La racine a partir de laquelle les chemins de la sortie autonome sont
  // calcules. Sans elle, Next la devine en remontant jusqu'au premier fichier
  // de verrouillage trouve : un `package-lock.json` oublie dans le dossier
  // personnel du developpeur suffit a faire sortir `standalone/server.js`
  // trois niveaux plus bas. Le paquet de bureau va chercher ce fichier a un
  // emplacement fixe ; la disposition ne peut pas dependre de ce qui traine
  // au-dessus du depot.
  outputFileTracingRoot: __dirname,
};

export default nextConfig;
