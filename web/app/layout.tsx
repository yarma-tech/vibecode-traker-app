import type { Metadata } from "next";
import "./globals.css";
import { Lecteur } from "./lecteur";

export const metadata: Metadata = {
  title: "Vibe Map",
  description:
    "Une carte de tes repos qui s'allume dès qu'un agent se met au travail.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="fr">
      {/* Le bandeau du lecteur passe par-dessus tous les ecrans : un lecteur
          arrete ne se voit nulle part ailleurs, et il ne se repare pas depuis
          la carte. Hors de l'application, il n'y a pas de pont et il ne
          s'affiche jamais (FR-060). */}
      <body>
        <Lecteur />
        {children}
      </body>
    </html>
  );
}
