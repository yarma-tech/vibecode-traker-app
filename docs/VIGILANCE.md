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
port**. Trois choses en dependent : l'adresse de retour de l'autorisation, que
le fournisseur exige de connaitre a l'avance ; une fenetre qui recharge la meme
origine a chaque lancement ; et le pont de commandes locales, ouvert a cette
seule origine.

**Une precision qui a d'abord ete ecrite a l'envers, ici et dans le PRD :** un
cookie n'a **jamais** de port dans sa portee. `127.0.0.1:51789` et
`127.0.0.1:3000` partagent donc leur session ; `localhost` et `127.0.0.1`, non,
parce que l'hote differe. Ce n'est pas le cookie que le port fixe protege.

C'est aussi pourquoi le service d'interface, qui ecoute sur `127.0.0.1` mais
fabriquait ses adresses en `localhost`, posait la session sur une origine que la
fenetre ne voyait pas (corrige avec l'issue #63).

## Eprouver une policy d'INSERTION en RLS (`supabase`, `daemon/tests`)

Une insertion faite avec `Prefer: return=representation` passe par un
`returning`, et un `returning` fait relire la ligne a travers la policy de
**lecture**. Sur `machines`, celle-ci ne montre a un jeton de machine que SA
ligne : une machine que ce jeton viendrait de creer est donc annulee par le
refus de lecture, avant meme qu'on sache ce que la policy d'insertion en aurait
fait.

**Le piege :** un test ecrit ainsi reste vert alors que la policy d'insertion a
ete videe de sa garde. Il a ete constate en sabotant `machines_insert_own` :
le test passait, et la ligne n'apparaissait pas. Pour eprouver l'insertion
seule, poster **sans** representation et compter les lignes ensuite - c'est ce
que fait `TestContext::tenter_inserer_machine_avec_jeton`.

## Les deux identites du poste (`bureau/src/machine.rs`)

La **session** de l'utilisateur vit dans les cookies de la fenetre. Le **jeton
de machine** vit au trousseau du systeme, sous `fr.yarma.vibemap` et sous
l'identifiant de la machine - la meme entree, exactement, que celle du binaire
en ligne de commande. La fenetre presente sa session au pont ; le pont ne lui
rend jamais le jeton de la machine, et `EtatMachine` n'a aucun champ ou le
loger. Ajouter un champ qui le porterait ferait sortir un secret de machine dans
une page web.

**Et ce qui ne doit jamais arriver :** un acces au trousseau REFUSE n'est pas un
jeton ABSENT. Le premier s'annonce avec un bouton pour reessayer (FR-080) ; le
second redemande un jeton pour LA MEME machine. Les confondre redeclare la
machine d'un utilisateur qui a simplement clique « Refuser », et fait apparaitre
un doublon dans sa liste.

## Revoquee n'est pas disparue (`bureau/src/machine.rs`, `geste_de_reprise`)

Depuis l'issue #66, une seule reponse de la base fait CREER une machine :
`DansLaBase::Inconnue`, c'est-a-dire un identifiant conserve qui ne designe plus
aucune ligne. Tout ce qui se lirait « inconnue » a tort fabrique donc une machine
de plus a chaque lancement.

**Les deux pieges, et ils sont opposes :**

- **redeclarer une machine REVOQUEE** la ferait revenir sous une autre identite,
  jeton neuf compris : la revocation serait defaite par le seul fait de rouvrir
  l'application. La base rend la difference elle-meme - une machine revoquee
  reste VISIBLE a la session de son proprietaire, `machines_select_own` ne la
  cache pas -, et c'est `representer` qui la traduit. Ne jamais ajouter de filtre
  `revoked_at is null` a cette lecture : la ligne deviendrait invisible, donc
  « inconnue », donc redeclaree ;
- **lire un refus comme une identite perdue.** Session expiree, jeton illisible,
  pile qui repond de travers : tout cela doit remonter en `Err`, jamais en
  `Ok(Inconnue)`. `daemon/tests/declaration.rs` le fige
  (`une_session_qui_ne_vaut_rien_ne_se_lit_pas_comme_une_identite_perdue`).

## L'adresse de la base, cote poste (`bureau/src/machine.rs`, FR-073)

La fenetre tient son adresse de la compilation ; le lecteur embarque, lui, ne
connait que `supabase_url` dans `~/.config/vibemap/config.toml`. Les deux
partagent ce fichier, et une configuration heritee du binaire en ligne de
commande y designe une pile locale de developpement.

**Le piege :** si l'alignement (`aligner_l_adresse_de_la_base`) disparait du
chemin de reprise, rien ne casse visiblement - la machine apparait bien dans la
liste, puisque c'est la fenetre qui l'y met - mais le lecteur pousse ses cartes
dans une autre base, et l'utilisateur voit sa machine sans jamais voir sa carte.
Un test qui ne regarde que la liste des machines ne peut pas voir ce
probleme.
