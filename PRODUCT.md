# Product

## Register

product

## Users

Un développeur seul, deux comptes GitHub, une cinquantaine de repos clonés, plusieurs agents Claude Code lancés en parallèle sur des machines différentes.

Son contexte d'usage est une **ouverture ponctuelle avant d'agir** : il est en plein travail, des agents tournent, il s'apprête à en lancer un de plus. Il bascule sur Vibe Map, lit, décide, referme. Session de quelques dizaines de secondes, sur ordinateur, en pleine journée de travail. Il consulte aussi depuis son téléphone quand il est loin du bureau, mais ce n'est pas le cas principal.

Le travail à accomplir : **savoir où ça en est sans avoir à lire**. Pas fouiller trois terminaux, pas relire des journaux, pas reconstruire mentalement qui fait quoi.

## Product Purpose

Vibe Map montre l'état vivant de ses codebases : quels agents travaillent, sur quels dossiers, depuis combien de temps, pour combien de jetons, et où deux agents sont en train de se marcher dessus.

La v1 observe, elle ne pilote pas. Lancer un agent depuis la carte et prédire les conflits avant lancement viendront ensuite.

Le succès se mesure à une chose : après un coup d'œil de dix secondes, l'utilisateur sait s'il peut lancer son deuxième agent, et où. S'il doit ouvrir un terminal pour vérifier, l'outil a échoué.

## Brand Personality

**Précis, sobre, sans bruit.** Un instrument de mesure, pas un tableau de bord bavard.

Il montre l'état, il ne le commente pas. Il n'alerte que quand quelque chose le mérite vraiment, donc son alerte se croit. Le vocabulaire est celui du métier, sans pédagogie inutile et sans jargon décoratif.

Référence assumée : Linear. Densité maîtrisée, clavier d'abord, micro-détails exécutés au pixel, neutres à peine teintés.

## Anti-references

- **Grafana et Datadog.** Murs de graphiques, dix couleurs qui ne signifient rien, légendes illisibles, tout à configurer soi-même. Vibe Map ne se configure pas : il montre une chose, bien.
- **Le registre jeu vidéo.** Néon, animations permanentes, badges, scores. La direction prise par CodeMap Hotel.
- **Le réflexe console sombre.** Le sombre bleuté par défaut, les cartes arrondies toutes identiques, le gros chiffre avec sa petite courbe. C'est ce que produit tout outil de monitoring depuis cinq ans, et c'est ce que produit une IA à qui on dit « observabilité ». Le choix du thème doit venir de la scène d'usage réelle, jamais de la catégorie.

## Design Principles

1. **La couleur ne dit que l'état.** Quatre états, quatre couleurs, rien de décoratif. Le rouge reste rare, c'est ce qui le rend lisible.
2. **Deux dimensions, deux moyens.** Le remplissage porte l'activité de l'agent, la bordure ou la trame porte l'état git. Jamais les deux dans le même canal.
3. **Lisible en dix secondes.** Chaque écran répond d'abord à « puis-je lancer un agent ici », le reste vient après.
4. **Rien ne bouge sans raison.** Le mouvement signale un changement d'état réel. Aucune animation d'ambiance.
5. **La carte est le produit.** Elle occupe la place, elle n'est pas un widget parmi d'autres.

## Accessibility & Inclusion

WCAG AA visé.

Contrainte déjà actée en conception : **jamais la couleur seule**. Chaque état est doublé d'un texte ou d'une forme (bordure, trame, étiquette), ce qui couvre le daltonisme sans mode spécial.

Le mouvement se coupe entièrement sous `prefers-reduced-motion`. Les deux thèmes clair et sombre sont traités avec le même soin, aucun n'est un rabais sur l'autre.
