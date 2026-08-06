/**
 * Découpage en parcelles, algorithme « squarified » de Bruls, Huizing et
 * van Wijk. Il cherche à donner à chaque parcelle une forme proche du carré :
 * des rectangles trop allongés seraient illisibles et fausseraient la
 * comparaison des surfaces à l'œil.
 *
 * La surface de chaque parcelle est proportionnelle à sa valeur.
 */

export type Parcelle<T> = {
  donnee: T;
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
};

type Cadre = { x: number; y: number; largeur: number; hauteur: number };

/** Le pire rapport d'aspect d'une rangée : plus il est bas, mieux c'est. */
function pireRapport(rangee: number[], cote: number): number {
  if (rangee.length === 0 || cote === 0) return Infinity;

  const somme = rangee.reduce((total, v) => total + v, 0);
  if (somme === 0) return Infinity;

  const maximum = Math.max(...rangee);
  const minimum = Math.min(...rangee);
  const cote2 = cote * cote;
  const somme2 = somme * somme;

  return Math.max((cote2 * maximum) / somme2, somme2 / (cote2 * minimum));
}

export function decouper<T>(
  elements: { donnee: T; valeur: number }[],
  cadre: Cadre,
): Parcelle<T>[] {
  const utiles = elements.filter((e) => e.valeur > 0);
  if (utiles.length === 0) return [];

  const total = utiles.reduce((somme, e) => somme + e.valeur, 0);
  const aire = cadre.largeur * cadre.hauteur;

  // On travaille en aires, pas en valeurs brutes : la conversion se fait ici,
  // une fois pour toutes.
  const restants = [...utiles]
    .sort((a, b) => b.valeur - a.valeur)
    .map((e) => ({ donnee: e.donnee, aire: (e.valeur / total) * aire }));

  const parcelles: Parcelle<T>[] = [];
  let libre = { ...cadre };
  let rangee: typeof restants = [];

  while (restants.length > 0) {
    const cote = Math.min(libre.largeur, libre.hauteur);
    const candidat = restants[0];

    const aires = rangee.map((r) => r.aire);
    const avant = pireRapport(aires, cote);
    const apres = pireRapport([...aires, candidat.aire], cote);

    // Tant qu'ajouter améliore (ou n'aggrave pas) la forme, on empile.
    if (rangee.length === 0 || apres <= avant) {
      rangee.push(candidat);
      restants.shift();
      continue;
    }

    libre = poserRangee(rangee, libre, parcelles);
    rangee = [];
  }

  poserRangee(rangee, libre, parcelles);
  return parcelles;
}

/** Pose une rangée le long du plus petit côté et rend l'espace qui reste. */
function poserRangee<T>(
  rangee: { donnee: T; aire: number }[],
  libre: Cadre,
  sortie: Parcelle<T>[],
): Cadre {
  if (rangee.length === 0) return libre;

  const aireRangee = rangee.reduce((somme, r) => somme + r.aire, 0);
  const horizontale = libre.largeur >= libre.hauteur;

  if (horizontale) {
    const largeur = aireRangee / libre.hauteur;
    let y = libre.y;

    for (const element of rangee) {
      const hauteur = element.aire / largeur;
      sortie.push({ donnee: element.donnee, x: libre.x, y, largeur, hauteur });
      y += hauteur;
    }

    return {
      x: libre.x + largeur,
      y: libre.y,
      largeur: libre.largeur - largeur,
      hauteur: libre.hauteur,
    };
  }

  const hauteur = aireRangee / libre.largeur;
  let x = libre.x;

  for (const element of rangee) {
    const largeur = element.aire / hauteur;
    sortie.push({ donnee: element.donnee, x, y: libre.y, largeur, hauteur });
    x += largeur;
  }

  return {
    x: libre.x,
    y: libre.y + hauteur,
    largeur: libre.largeur,
    hauteur: libre.hauteur - hauteur,
  };
}
