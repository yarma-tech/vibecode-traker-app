# Vigilance

Les endroits ou une modification innocente casse quelque chose de loin.
Une entree se lit avant de toucher la zone qu'elle nomme, pas apres.

## L'horizon de remontee des journaux (`daemon/src/journal.rs`)

Le lecteur vivant n'envoie jamais d'evenement plus vieux que
`journal_lookback_seconds` (600 s), et cette borne vaut pour **tous** les
journaux depuis l'issue #82 - avant, elle n'attrapait que ceux jamais vus, et
un journal deja connu rejouait tout son passe dans le journal direct apres une
longue fermeture.

**Le piege :** les couleurs de la carte ne trahissent pas ce defaut. La vue
`etat_modules` filtre deja sur sa propre fenetre, donc la carte reste juste
meme quand du passe est injecte. Ce sont le journal direct et les sessions qui
mentent. Un test qui ne regarde que les couleurs ne peut pas voir le probleme -
c'est ce qui l'avait laisse passer.

**Effet de bord assume :** si la base est injoignable plus longtemps que
l'horizon, les evenements plus vieux que l'horizon ne sont plus renvoyes une
fois qu'elle revient. Le rattrapage apres une panne longue est donc borne a 10
minutes. C'est le prix de la promesse « le passe n'est jamais rejoue en
direct ». Augmenter l'horizon rallonge le rattrapage et rallonge d'autant la
fenetre ou du passe peut remonter : les deux ne se separent pas.

## Les deux canaux de lecture des journaux

Deux chemins lisent `~/.claude/projects`, avec des regles **opposees**, et les
confondre casse le produit dans un sens ou dans l'autre :

- **le direct** (`daemon/src/journal.rs`) ecrit de l'activite, donc allume les
  couleurs. Il est borne a l'horizon, position tenue dans `offsets.json` ;
- **le depouillement du passe** (`daemon/src/depouillement.rs`) remonte a trente
  jours, et n'a le droit de rien allumer. Il n'ecrit que les deux dates de
  derniere touche, progression tenue dans `depouillement.json`.

**Les deux marques ne doivent jamais s'ecraser** : le compagnon en ligne de
commande peut avoir fait avancer la position du direct pendant que
l'application etait fermee. La separation est structurelle - le type
intermediaire du depouillement n'a de champ ni pour une session, ni pour un
identifiant d'appel d'outil - et c'est volontaire : elle rend l'erreur
impossible plutot que surveillee.

## Les deux dates de derniere touche (`supabase`, table `dernieres_touches`)

Elles ne peuvent pas vivre dans la table des modules : la cartographie la vide
et la repeuple **en bloc** toutes les cinq minutes, ce qui les effacerait. Elles
ne peuvent pas non plus se recalculer depuis les evenements d'activite, purges
a sept jours. D'ou une table a part, effacee seulement avec son depot.

**Une date ne recule jamais.** La regle est posee du cote de la base
(`greatest`), pas du cote des appelants, pour que le depouillement du passe
puisse ecrire dans le desordre sans rien casser, et qu'un envoi rejoue par la
file d'attente locale reste sans effet.

## Le verrou du poste (`daemon/src/verrou.rs`)

Un `flock` tenu par le noyau, jamais un fichier de PID : le noyau relache quoi
qu'il arrive au processus, donc aucun verrou orphelin ne condamne le poste.

**Le piege, pour les tests :** entre le `fork` et le `exec` d'un processus
enfant, l'enfant detient une copie de tous les descripteurs du parent, y compris
ceux marques `FD_CLOEXEC`. Un verrou relache pendant cette fenetre reste tenu le
temps que l'enfant franchisse son `exec`. Les tests de `daemon/tests/verrou.rs`
sont serialises pour cette raison - ne pas retirer cette serialisation en
croyant simplifier.

## Le port de l'interface locale (`bureau/`)

`127.0.0.1:51789`, fixe et documente, **jamais de repli silencieux sur un autre
port**. Trois choses en dependent : l'adresse de retour de l'autorisation
GitHub, que le fournisseur exige de connaitre a l'avance ; le cookie de session,
lie a l'origine et donc au port ; et le pont de commandes locales, ouvert a
cette seule origine.
