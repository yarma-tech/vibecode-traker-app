//! Les dossiers surveilles, tels que l'ecran Reglages les voit.
//!
//! FR-059 partage les sources, et ce partage est la raison d'etre de ce
//! module : la liste des dossiers, leur compte de depots, leur lisibilite et
//! leur autorisation sont des faits du POSTE. Ils n'existent nulle part
//! ailleurs que sur cette machine - la base ne porte ni les chemins surveilles,
//! ni le fait qu'un dossier ait ete renomme il y a dix secondes -, et ils ne
//! sortent pas d'ici. L'HEURE de la derniere cartographie, elle, ne passe pas
//! par la : elle vit en base avec le catalogue, et l'ecran l'y lit.
//!
//! Le partage n'est pas pour autant « tout ce qui touche a la cartographie va
//! en base » : le fait qu'il n'y ait RIEN a cartographier ici (FR-087) est un
//! fait du poste, et il passe par ce module. Quand une cartographie aboutit sans
//! trouver aucun depot, la base n'a aucune heure a donner - il n'y a pas de
//! depot pour en porter une -, et l'ecran conclurait « jamais cartographie » a
//! un poste qui l'a bien ete.
//!
//! POURQUOI ces faits se relevent a chaque demande, plutot que de se noter au
//! demarrage : la question que l'utilisateur pose en ouvrant Reglages est
//! « pourquoi ce depot n'apparait pas », et la reponse la plus frequente est un
//! dossier renomme depuis. Un etat fige repondrait a cote.
//!
//! La borne du pont tient ici comme ailleurs : ce module ne sait pas ouvrir un
//! chemin qu'on lui donne. Il lit la liste que la configuration du lecteur
//! porte deja, et il n'y ajoute que ce que le selecteur du systeme a rendu -
//! un dossier que l'utilisateur a designe lui-meme, jamais un chemin que la
//! fenetre aurait choisi (FR-031).
//!
//! POURQUOI l'ajout et le retrait passent par le fichier de configuration :
//! c'est la meme liste que le lecteur lit, et il n'y en a pas deux. Le fichier
//! reste donc le support de stockage - il cesse seulement d'etre une porte :
//! personne n'a plus a l'ouvrir pour surveiller un dossier de plus, ni pour
//! cesser d'en surveiller un (FR-036).
//!
//! Et ce que ce module ecrit, il ne l'ecrit QUE la. Un dossier retire quitte la
//! liste et rien d'autre : ni le disque, ni le catalogue ne perdent quoi que ce
//! soit (FR-035). Cesser de surveiller, ce n'est pas effacer ce qu'on a deja
//! observe - les depots deja cartographies restent au catalogue, dates de leur
//! derniere cartographie, et cessent seulement d'etre rafraichis.

use std::path::{Path, PathBuf};

use toml_edit::{Array, DocumentMut, Item, Value};
use vibemap::Config;

/// Ce que le poste repond quand on va regarder un dossier surveille.
///
/// Les quatre cas ne se corrigent pas de la meme facon, et c'est la seule
/// raison de les distinguer : un dossier introuvable se retire ou se recree, un
/// acces refuse se redemande au systeme, un chemin qui n'est pas un dossier a
/// ete mal choisi. Les confondre sous « illisible » renverrait l'utilisateur
/// chercher lui-meme lequel des trois il a sous les yeux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lisibilite {
    Lisible,
    /// Plus rien a ce chemin : le dossier a ete renomme, deplace ou supprime.
    Introuvable,
    /// Le systeme refuse l'acces. Sur macOS, c'est le cas des dossiers proteges
    /// tant que l'application n'a pas recu l'autorisation.
    AutorisationRefusee,
    /// Quelque chose existe la, mais ce n'est pas un dossier ouvrable.
    PasUnDossier,
}

/// Un dossier surveille, tel que l'ecran le montre sur une ligne.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DossierSurveille {
    /// Le chemin tel qu'il est ecrit dans la configuration - `~/Developer` reste
    /// `~/Developer`. C'est celui que l'utilisateur reconnait.
    pub chemin: String,
    /// Le meme, deplie. Il ne sert qu'a lever un doute quand le `~` cache ou
    /// l'on regarde vraiment.
    pub emplacement: String,
    pub lisibilite: Lisibilite,
    /// `None` des que le dossier ne se lit pas : un zero se lirait comme un
    /// dossier bien ouvert et vide, ce qui est une tout autre affaire (FR-029).
    pub depots: Option<usize>,
}

/// Y a-t-il quelque chose a cartographier sur ce poste ? (FR-087)
///
/// POURQUOI ce fait-la passe par le pont alors que l'heure de la cartographie
/// vit en base : quand une cartographie a abouti sans trouver aucun depot, la
/// base ne porte AUCUNE heure - il n'y a pas de depot pour en porter une. Sans
/// ce fait, l'ecran conclurait « jamais cartographie » a un poste qui l'a ete,
/// et enverrait chercher une panne la ou il n'y en a pas. La base ne peut pas
/// le dire ; le poste, si.
///
/// « On ne sait pas » se garde pour un dossier qu'on n'a pas pu ouvrir :
/// conclure « rien » a sa place ferait dire a l'ecran qu'une cartographie a
/// abouti sur du vide alors qu'elle a peut-etre trouve cinquante depots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ADecouvrir {
    /// Au moins un dossier surveille porte un depot : une cartographie qui
    /// aboutit ici laisse forcement une heure en base.
    DesDepots,
    /// Tous les dossiers surveilles s'ouvrent, et aucun ne porte de depot. Une
    /// cartographie a beau aboutir, elle ne rapporte rien.
    Rien,
    /// Un dossier au moins ne s'ouvre pas : on ne conclut pas a sa place.
    Inconnu,
}

impl ADecouvrir {
    fn depuis(dossiers: &[DossierSurveille]) -> ADecouvrir {
        if dossiers.iter().any(|dossier| dossier.depots.is_none()) {
            return ADecouvrir::Inconnu;
        }
        if dossiers
            .iter()
            .any(|dossier| dossier.depots.unwrap_or(0) > 0)
        {
            return ADecouvrir::DesDepots;
        }
        ADecouvrir::Rien
    }
}

/// Ce que la commande du pont rend a l'ecran.
///
/// Une configuration illisible n'est pas une liste vide : l'une dit « aucun
/// dossier n'est surveille », l'autre « on ne sait pas ce qui est surveille ».
/// L'ecran ne doit pas pouvoir les confondre, d'ou deux formes distinctes
/// plutot qu'un vecteur vide et un message a cote.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum Surveillance {
    Lue {
        dossiers: Vec<DossierSurveille>,
        /// FR-087 : ce fait du poste voyage avec la liste, et non par une
        /// commande a part. Il se lit sur les MEMES dossiers, au meme instant :
        /// deux appels separes pourraient tomber de part et d'autre d'un ajout
        /// et se contredire.
        a_decouvrir: ADecouvrir,
    },
    SansConfiguration {
        raison: String,
    },
}

/// Les dossiers surveilles du poste, lus a l'emplacement habituel de la
/// configuration du lecteur.
///
/// Aucun chemin ne vient de la fenetre : c'est ce qui fait de cette commande un
/// geste nomme et non un acces au disque.
pub fn dossiers_du_poste() -> Surveillance {
    dossiers_surveilles(&Config::chemin_par_defaut())
}

/// La meme lecture, sur une configuration donnee.
///
/// Le chemin est passe explicitement pour que la liste s'eprouve sur des
/// dossiers temporaires, sans rien devoir a ce qui vit sur la machine.
pub fn dossiers_surveilles(chemin_config: &Path) -> Surveillance {
    let config = match Config::load(chemin_config) {
        Ok(config) => config,
        Err(erreur) => {
            return Surveillance::SansConfiguration {
                raison: erreur.to_string(),
            }
        }
    };

    // `roots` et `racines()` marchent du meme pas : le premier garde l'ecriture
    // de l'utilisateur, le second la deplie. On tient les deux, parce que
    // l'ecran montre l'une et regarde l'autre.
    let dossiers: Vec<DossierSurveille> = config
        .roots
        .iter()
        .zip(config.racines())
        .map(|(brut, emplacement)| regarder(brut, &emplacement))
        .collect();

    let a_decouvrir = ADecouvrir::depuis(&dossiers);
    Surveillance::Lue {
        dossiers,
        a_decouvrir,
    }
}

/// Ce qu'un dossier surveille donne quand on va le voir.
pub fn regarder(brut: &str, emplacement: &Path) -> DossierSurveille {
    let lisibilite = lisibilite(emplacement);

    DossierSurveille {
        chemin: brut.to_string(),
        emplacement: emplacement.display().to_string(),
        lisibilite,
        // Compter demande d'ouvrir le dossier : il n'y a rien a compter dans un
        // dossier qu'on ne sait pas ouvrir.
        depots: (lisibilite == Lisibilite::Lisible).then(|| compter_les_depots(emplacement)),
    }
}

/// Le dossier s'ouvre-t-il, et sinon pourquoi.
///
/// La question se pose en l'ouvrant, jamais en interrogeant ses metadonnees :
/// un dossier dont on peut lire l'existence sans pouvoir lister le contenu est
/// exactement le cas que FR-029 demande de signaler, et `exists()` le dirait
/// present.
fn lisibilite(chemin: &Path) -> Lisibilite {
    match std::fs::read_dir(chemin) {
        Ok(_) => Lisibilite::Lisible,
        Err(erreur) => match erreur.kind() {
            std::io::ErrorKind::NotFound => Lisibilite::Introuvable,
            std::io::ErrorKind::PermissionDenied => Lisibilite::AutorisationRefusee,
            // Un fichier ordinaire donne a la place d'un dossier : il est bien
            // la, et il ne s'ouvrira jamais.
            _ if chemin.exists() => Lisibilite::PasUnDossier,
            // Le reste - un montage disparu, un lien casse - se lit du point de
            // vue de l'utilisateur : il n'y a rien a cet endroit.
            _ => Lisibilite::Introuvable,
        },
    }
}

/// Combien de depots ce dossier porte.
///
/// Exactement le critere de la cartographie du lecteur : ses enfants DIRECTS
/// qui contiennent un `.git`, et rien de plus profond. C'est ce qui fait que ce
/// compte repond a la question posee - « pourquoi ce depot n'apparait pas » -
/// au lieu d'annoncer des depots que la cartographie ne verra jamais.
fn compter_les_depots(chemin: &Path) -> usize {
    let Ok(entrees) = std::fs::read_dir(chemin) else {
        return 0;
    };

    entrees
        .flatten()
        .filter(|entree| porte_un_depot(&entree.path()))
        .count()
}

fn porte_un_depot(enfant: &Path) -> bool {
    enfant.join(".git").exists()
}

/* ---------- ajouter un dossier (FR-031, FR-036) ---------- */

/// Ce que l'ajout d'un dossier a donne, tel que l'ecran le lit.
///
/// Quatre issues, et elles ne se ressemblent pas. Un selecteur ferme sans rien
/// choisir n'est pas un echec : c'est un geste repris, et l'ecran n'a rien a
/// annoncer. Une ecriture qui echoue, si : la liste n'a pas bouge, et
/// l'utilisateur doit savoir que son dossier n'est pas surveille. Un refus n'est
/// ni l'un ni l'autre : rien ne s'est casse, et le poste surveille deja ce
/// dossier - encore faut-il dire par ou (FR-037, FR-075).
///
/// L'ajout rend la liste entiere plutot que le seul dossier ajoute : l'ecran
/// montre alors le nouveau venu ET son compte de depots d'un seul tenant, sans
/// redemander - et sans passer par un instant ou la ligne existe deja mais
/// n'aurait pas encore de compte.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "issue", rename_all = "snake_case")]
pub enum Ajout {
    /// Le selecteur s'est ferme sans choix. Rien n'a ete ecrit.
    Annule,
    Ajoute {
        /// Le chemin tel qu'il vient d'etre ecrit dans la configuration.
        chemin: String,
        surveillance: Surveillance,
    },
    /// Le dossier choisi est deja couvert par la surveillance (FR-037,
    /// FR-075). Rien n'a ete ecrit, et le refus NOMME le dossier surveille qui
    /// s'y oppose : « deja surveille » sans dire lequel laisserait l'utilisateur
    /// ouvrir le fichier de configuration pour le chercher, ce que FR-036
    /// interdit.
    Refuse {
        /// Le dossier choisi, ecrit comme l'utilisateur ecrit les siens.
        chemin: String,
        /// Le dossier deja surveille qui s'y oppose, tel qu'il est ecrit dans
        /// la configuration.
        deja: String,
        cas: Emboitement,
    },
    /// Rien n'a pu etre ecrit. `raison` nomme ce qui s'y oppose, dans les
    /// termes de la configuration du lecteur.
    Echoue { raison: String },
}

/// Ajoute un dossier a la surveillance du poste, a l'emplacement habituel de la
/// configuration du lecteur.
pub fn ajouter_au_poste(choisi: &Path) -> Ajout {
    ajouter(&Config::chemin_par_defaut(), choisi)
}

/// Le meme ajout, sur une configuration donnee.
///
/// Le chemin est passe explicitement pour que l'ecriture s'eprouve sur des
/// dossiers temporaires, sans toucher a la configuration de l'utilisateur.
///
/// Le dossier est juge AVANT d'etre ecrit (FR-037, FR-075) : un refus ne doit
/// pas laisser la configuration gagner une ligne qu'il faudrait ensuite
/// reprendre.
pub fn ajouter(chemin_config: &Path, choisi: &Path) -> Ajout {
    let ecrit = abreger(choisi, &maison());

    // La configuration se lit une premiere fois pour juger. C'est la meme porte
    // que `inscrire` reouvrira : une configuration que le lecteur refuse ne
    // rend pas de racines, et l'ajout echoue alors en le disant plutot que de
    // se prononcer sur un doublon a partir de rien.
    if let Ok(config) = Config::load(chemin_config) {
        let surveilles: Vec<(String, PathBuf)> =
            config.roots.iter().cloned().zip(config.racines()).collect();

        if let Some(conflit) = deja_surveille(&surveilles, choisi) {
            return Ajout::Refuse {
                chemin: ecrit,
                deja: conflit.deja,
                cas: conflit.cas,
            };
        }
    }

    match inscrire(chemin_config, &ecrit) {
        Ok(()) => Ajout::Ajoute {
            chemin: ecrit,
            surveillance: dossiers_surveilles(chemin_config),
        },
        Err(raison) => Ajout::Echoue { raison },
    }
}

/* ---------- retirer un dossier (FR-032, FR-035, FR-036) ---------- */

/// Ce que le retrait d'un dossier a donne, tel que l'ecran le lit.
///
/// Trois issues. Le retrait d'un dossier qui n'est plus dans la liste n'est ni
/// un succes ni une panne : rien n'a ete ecrit, et l'ecran affichait une liste
/// qui avait vieilli - d'ou la liste rendue avec le refus, pour qu'il se remette
/// d'aplomb au lieu de proposer une seconde fois un geste sans objet.
///
/// POURQUOI aucune variante ne parle du catalogue : le retrait n'y touche pas,
/// et c'est tout le sens de FR-035. Ce module ecrit dans la configuration du
/// lecteur, nulle part ailleurs ; les depots deja cartographies restent ou ils
/// sont, avec leur derniere heure connue, et cessent seulement d'etre
/// rafraichis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "issue", rename_all = "snake_case")]
pub enum Retrait {
    /// Le dossier ne figure plus dans la liste que le lecteur lit.
    Retire {
        /// Le chemin tel qu'il etait ecrit dans la configuration.
        chemin: String,
        surveillance: Surveillance,
    },
    /// Aucune racine ne portait ce chemin. Rien n'a ete ecrit.
    Inconnu {
        chemin: String,
        surveillance: Surveillance,
    },
    /// Rien n'a pu etre ecrit. `raison` nomme ce qui s'y oppose, dans les
    /// termes de la configuration du lecteur.
    Echoue { raison: String },
}

/// Retire un dossier de la surveillance du poste, a l'emplacement habituel de la
/// configuration du lecteur.
pub fn retirer_du_poste(ecrit: &str) -> Retrait {
    retirer(&Config::chemin_par_defaut(), ecrit)
}

/// Le meme retrait, sur une configuration donnee.
///
/// `ecrit` est le chemin TEL QU'IL EST ECRIT dans la configuration - celui que
/// la liste rendue par ce module porte deja, et celui que l'ecran affiche. La
/// borne du pont tient donc au retrait comme a l'ajout : la fenetre ne designe
/// pas un endroit du disque, elle designe une ligne de la liste que le poste lui
/// a donnee. Un chemin qui n'y figure pas ne fait rien du tout - il n'ouvre
/// rien, il ne supprime rien.
///
/// Rien n'est efface d'autre que cette ligne (FR-035) : ni le dossier sur le
/// disque, ni les depots au catalogue. Le retrait est un arret de surveillance,
/// pas un effacement, et le rajouter suffit a reprendre.
pub fn retirer(chemin_config: &Path, ecrit: &str) -> Retrait {
    match radier(chemin_config, ecrit) {
        Ok(true) => Retrait::Retire {
            chemin: ecrit.to_string(),
            surveillance: dossiers_surveilles(chemin_config),
        },
        Ok(false) => Retrait::Inconnu {
            chemin: ecrit.to_string(),
            surveillance: dossiers_surveilles(chemin_config),
        },
        Err(raison) => Retrait::Echoue { raison },
    }
}

/// Retire le dossier de la liste que le lecteur lit, sans toucher au reste.
///
/// Rend `true` quand la liste a bouge, `false` quand elle ne portait pas ce
/// chemin - et dans ce dernier cas le fichier n'est pas meme reecrit : une
/// reecriture a l'identique changerait sa date sans rien changer d'autre, et
/// ferait douter de ce qui s'est passe.
///
/// Comme `inscrire`, le fichier est EDITE : l'adresse de la base, les cadences
/// et les commentaires de l'utilisateur doivent se retrouver intacts apres le
/// retrait.
fn radier(chemin_config: &Path, ecrit: &str) -> Result<bool, String> {
    // La meme porte que le lecteur, d'abord : une configuration qu'il refuserait
    // ne s'edite pas, et c'est aussi la seule facon de connaitre les racines
    // qu'il surveille par defaut.
    let config = Config::load(chemin_config).map_err(|erreur| erreur.to_string())?;

    let restantes: Vec<&String> = config
        .roots
        .iter()
        .filter(|racine| *racine != ecrit)
        .collect();
    if restantes.len() == config.roots.len() {
        return Ok(false);
    }

    let brut = std::fs::read_to_string(chemin_config).map_err(|erreur| {
        format!(
            "configuration illisible a {} : {erreur}",
            chemin_config.display()
        )
    })?;
    let mut document: DocumentMut = brut.parse().map_err(|erreur| {
        format!(
            "configuration invalide a {} : {erreur}",
            chemin_config.display()
        )
    })?;

    match document.get_mut("roots").and_then(Item::as_array_mut) {
        // Toutes les occurrences, et pas seulement la premiere : une
        // configuration ecrite a la main peut porter deux fois le meme chemin, et
        // n'en retirer qu'une laisserait le dossier surveille apres un geste qui
        // annonce le contraire.
        Some(liste) => liste.retain(|valeur| valeur.as_str() != Some(ecrit)),
        // La liste n'est pas ecrite, et le lecteur en surveille quand meme une
        // par defaut. On l'ecrit noir sur blanc, privee du dossier retire : sans
        // cela, le retrait n'aurait aucun effet sur ce que le lecteur lit.
        None => {
            let mut liste = Array::new();
            for racine in &restantes {
                liste.push(racine.as_str());
            }
            document["roots"] = Item::Value(Value::Array(liste));
        }
    }

    ecrire_sans_perdre(chemin_config, &document.to_string())?;
    Ok(true)
}

/* ---------- redemander une autorisation refusee (FR-061) ---------- */

/// Ce que « redemander l'autorisation » a donne.
///
/// POURQUOI passer par le selecteur du systeme plutot que par une demande
/// d'acces : sur macOS, un refus deja donne ne se redemande pas - le systeme ne
/// repose plus la question. Ce que l'utilisateur designe lui-meme au selecteur,
/// en revanche, lui est accorde. Redemander l'autorisation, c'est donc rouvrir
/// le selecteur sur ce meme dossier, et c'est aussi ce qui garde la borne du
/// pont : aucun chemin ne vient de la fenetre.
///
/// Les cinq issues appellent cinq gestes differents, et c'est la seule raison de
/// les distinguer : un selecteur referme n'appelle rien, un autre dossier
/// designe se recommence, un refus qui tient demande un detour par les reglages
/// du systeme.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "issue", rename_all = "snake_case")]
pub enum Reautorisation {
    /// Le selecteur s'est ferme sans choix. L'autorisation n'a pas bouge.
    Annulee,
    /// Ce dossier n'est plus surveille : il n'y a plus d'autorisation a
    /// redemander pour lui.
    Inconnu {
        chemin: String,
        surveillance: Surveillance,
    },
    /// L'utilisateur a designe un AUTRE dossier. Le systeme n'a donc accorde
    /// l'acces qu'a celui-la, et le dossier attendu reste illisible.
    AutreDossier {
        /// Le dossier dont on redemandait l'autorisation.
        attendu: String,
        /// Celui qui a ete designe a la place, ecrit comme l'utilisateur ecrit
        /// les siens.
        choisi: String,
    },
    /// Le dossier se lit de nouveau.
    Accordee {
        chemin: String,
        surveillance: Surveillance,
    },
    /// Le bon dossier a ete designe, et il ne se lit toujours pas.
    Refusee {
        chemin: String,
        surveillance: Surveillance,
    },
}

/// Redemande l'autorisation d'un dossier surveille du poste.
pub fn redemander_au_poste(attendu: &str, choisi: Option<&Path>) -> Reautorisation {
    redemander(&Config::chemin_par_defaut(), attendu, choisi)
}

/// La meme redemande, sur une configuration donnee.
///
/// `choisi` est ce que le selecteur du systeme a rendu, `None` quand il s'est
/// referme sans choix. Le selecteur est passe en parametre plutot qu'ouvert
/// ici : c'est ce qui permet d'eprouver les cinq issues sur des dossiers
/// temporaires, la ou personne ne peut cliquer dans une fenetre du systeme.
///
/// La lisibilite se releve APRES le choix, jamais avant : c'est precisement le
/// choix qui accorde l'acces, et une lisibilite relevee trop tot dirait encore
/// « refuse » d'un dossier qui vient de s'ouvrir.
pub fn redemander(chemin_config: &Path, attendu: &str, choisi: Option<&Path>) -> Reautorisation {
    // Un selecteur referme d'abord : rien n'a ete demande au systeme, et l'etat
    // de la configuration n'y change rien.
    let Some(choisi) = choisi else {
        return Reautorisation::Annulee;
    };

    let config = match Config::load(chemin_config) {
        Ok(config) => config,
        // Sans configuration lisible, on ne sait pas quel dossier etait attendu.
        // Le dire comme un dossier qu'on ne surveille plus est le plus proche de
        // la verite : il n'y a pas d'autorisation a redemander ici.
        Err(erreur) => {
            return Reautorisation::Inconnu {
                chemin: attendu.to_string(),
                surveillance: Surveillance::SansConfiguration {
                    raison: erreur.to_string(),
                },
            }
        }
    };

    let Some(emplacement) = config
        .roots
        .iter()
        .zip(config.racines())
        .find(|(ecrit, _)| ecrit.as_str() == attendu)
        .map(|(_, deplie)| deplie)
    else {
        return Reautorisation::Inconnu {
            chemin: attendu.to_string(),
            surveillance: dossiers_surveilles(chemin_config),
        };
    };

    // Le meme jugement que le doublon (FR-075) : liens resolus, barre finale
    // normalisee. Le selecteur rend un chemin deplie, la configuration porte
    // souvent un `~`, et une comparaison de texte les dirait differents alors
    // qu'ils designent le meme endroit.
    if forme_comparable(choisi) != forme_comparable(&emplacement) {
        return Reautorisation::AutreDossier {
            attendu: attendu.to_string(),
            choisi: abreger(choisi, &maison()),
        };
    }

    let vu = regarder(attendu, &emplacement);
    let surveillance = dossiers_surveilles(chemin_config);
    if vu.lisibilite == Lisibilite::Lisible {
        Reautorisation::Accordee {
            chemin: attendu.to_string(),
            surveillance,
        }
    } else {
        Reautorisation::Refusee {
            chemin: attendu.to_string(),
            surveillance,
        }
    }
}

/* ---------- le doublon, et l'emboitement (FR-037, FR-075) ---------- */

/// Comment le dossier choisi rencontre un dossier deja surveille.
///
/// Les trois se refusent, et ils ne se disent pas de la meme facon : le premier
/// est un geste sans effet, le deuxieme des depots qui remontent deja d'ailleurs,
/// le troisieme une racine plus large qui cartographierait les memes depots deux
/// fois. L'ecran a besoin de savoir lequel pour dire quoi faire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Emboitement {
    /// Le meme dossier, sous une autre ecriture.
    Meme,
    /// Le dossier choisi est DEDANS un dossier deja surveille.
    Contenu,
    /// Le dossier choisi CONTIENT un dossier deja surveille.
    Contient,
}

/// Le dossier surveille qui s'oppose a l'ajout, et pourquoi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflit {
    /// Tel qu'il est ecrit dans la configuration : c'est l'ecriture que
    /// l'utilisateur reconnait, et celle qu'il doit retrouver dans le refus.
    pub deja: String,
    pub cas: Emboitement,
}

/// Le premier dossier surveille qui s'oppose a l'ajout du dossier choisi.
///
/// `surveilles` porte les deux ecritures de chaque racine : celle de la
/// configuration - qui sert a NOMMER le coupable - et la meme, `~` deplie - qui
/// sert a JUGER. Les separer est ce qui permet de refuser en disant
/// « ~/Developer » plutot que « /Users/lea/Developer », que l'utilisateur n'a
/// jamais ecrit.
///
/// Le premier, et non le plus proche : la configuration est une liste ordonnee,
/// et en nommer un seul suffit a expliquer le refus. Un refus qui les
/// enumererait tous en dirait plus long sans rien ajouter au geste a faire.
pub fn deja_surveille(surveilles: &[(String, PathBuf)], choisi: &Path) -> Option<Conflit> {
    let choisi = forme_comparable(choisi);
    if choisi.as_os_str().is_empty() {
        return None;
    }

    surveilles.iter().find_map(|(ecrit, deplie)| {
        let surveille = forme_comparable(deplie);
        // Une racine vide n'est pas un dossier : la laisser passer ferait
        // d'elle le prefixe de tout, et plus rien ne serait jamais ajoutable.
        if surveille.as_os_str().is_empty() {
            return None;
        }

        emboitement(&choisi, &surveille).map(|cas| Conflit {
            deja: ecrit.clone(),
            cas,
        })
    })
}

/// Comment deux chemins deja normalises se rencontrent, ou `None` s'ils ne se
/// rencontrent pas.
///
/// La comparaison porte sur les COMPOSANTS du chemin, jamais sur le texte :
/// `~/Developer` ne contient pas `~/Developer-2`, alors qu'un prefixe de texte
/// le dirait - et refuserait un dossier voisin parfaitement legitime.
fn emboitement(choisi: &Path, surveille: &Path) -> Option<Emboitement> {
    if choisi == surveille {
        Some(Emboitement::Meme)
    } else if choisi.starts_with(surveille) {
        Some(Emboitement::Contenu)
    } else if surveille.starts_with(choisi) {
        Some(Emboitement::Contient)
    } else {
        None
    }
}

/// Le chemin sous la forme qui juge : liens symboliques resolus, barre finale
/// et `.` normalises.
///
/// POURQUOI passer par le disque : un meme dossier s'ecrit de plusieurs facons
/// qui ne se ressemblent pas - `~/Developer`, `~/Developer/`, son chemin
/// absolu, un lien symbolique pose ailleurs qui pointe dessus. Aucune
/// comparaison de texte ne les rapproche ; seul le systeme sait qu'elles
/// designent le meme endroit, et FR-075 demande de juger la.
///
/// Quand il ne sait pas repondre - le dossier n'existe pas, ou pas encore -, on
/// s'en tient a une normalisation de composants : elle attrape la barre finale
/// et les `.`, qui sont les cas courants, et ne pretend rien de plus. Un dossier
/// surveille qui a ete renomme continue ainsi de s'opposer a lui-meme.
fn forme_comparable(chemin: &Path) -> PathBuf {
    std::fs::canonicalize(chemin).unwrap_or_else(|_| chemin.components().collect())
}

/// Le chemin choisi, ecrit comme l'utilisateur ecrit les siens.
///
/// POURQUOI reabreger en `~` ce que le selecteur rend deplie : la configuration
/// est relue par un humain de temps en temps, et une liste ou `~/Developer`
/// voisine avec `/Users/lea/Sites` se lit comme deux choses differentes alors
/// que c'en est une seule. Le `~` suit aussi le dossier personnel d'un compte a
/// l'autre, la ou un chemin absolu le fige.
///
/// La comparaison se fait par COMPOSANTS et non sur le texte : `/Users/lea` ne
/// prefixe pas `/Users/leandre/Sites`, et une abreviation faite sur le texte
/// rendrait la un `~ndre/Sites` qui ne designe rien.
pub fn abreger(choisi: &Path, maison: &Path) -> String {
    if maison.as_os_str().is_empty() {
        return choisi.display().to_string();
    }

    match choisi.strip_prefix(maison) {
        Ok(reste) if reste.as_os_str().is_empty() => "~".to_string(),
        Ok(reste) => format!("~/{}", reste.display()),
        // Hors du dossier personnel - un disque externe, `/opt` : le chemin
        // s'ecrit tel quel.
        Err(_) => choisi.display().to_string(),
    }
}

fn maison() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

/// Ecrit le dossier dans la liste que le lecteur lit, sans toucher au reste.
///
/// Le fichier est EDITE, jamais reecrit a partir de ce qu'on a su en relire :
/// `supabase_url`, l'identifiant de la machine, les cadences, et jusqu'aux
/// commentaires que l'utilisateur y a laisses doivent se retrouver intacts
/// apres l'ajout. Une reserialisation les ferait disparaitre en silence, et
/// c'est une configuration perdue.
fn inscrire(chemin_config: &Path, ecrit: &str) -> Result<(), String> {
    // La meme porte que le lecteur, d'abord : une configuration qu'il
    // refuserait ne doit pas gagner une ligne de plus - elle serait ecrite pour
    // un lecteur qui ne demarrera pas -, et c'est aussi la seule facon de
    // connaitre les racines qu'il surveille par defaut.
    let config = Config::load(chemin_config).map_err(|erreur| erreur.to_string())?;

    let brut = std::fs::read_to_string(chemin_config).map_err(|erreur| {
        format!(
            "configuration illisible a {} : {erreur}",
            chemin_config.display()
        )
    })?;
    let mut document: DocumentMut = brut.parse().map_err(|erreur| {
        format!(
            "configuration invalide a {} : {erreur}",
            chemin_config.display()
        )
    })?;

    match document.get_mut("roots").and_then(Item::as_array_mut) {
        // La liste est ecrite : on y ajoute, et ce qui l'entoure ne bouge pas.
        Some(liste) => liste.push(ecrit),
        // Elle ne l'est pas, et le lecteur en surveille quand meme une par
        // defaut. L'ecrire sans elle retirerait de la surveillance un dossier
        // que personne n'a demande a retirer.
        None => {
            let mut liste = Array::new();
            for racine in &config.roots {
                liste.push(racine.as_str());
            }
            liste.push(ecrit);
            document["roots"] = Item::Value(Value::Array(liste));
        }
    }

    ecrire_sans_perdre(chemin_config, &document.to_string())
}

/// Ecrit a cote, puis remplace.
///
/// POURQUOI : une ecriture interrompue en plein milieu - plus de place, machine
/// eteinte - laisserait une configuration tronquee, et le lecteur ne
/// redemarrerait plus du tout. Le remplacement, lui, est atomique : a tout
/// instant, le fichier est soit l'ancien entier, soit le nouveau entier.
fn ecrire_sans_perdre(chemin: &Path, contenu: &str) -> Result<(), String> {
    let nom = chemin.file_name().unwrap_or_default().to_string_lossy();
    let voisin = chemin.with_file_name(format!("{nom}.en-cours-d-ecriture"));

    std::fs::write(&voisin, contenu)
        .map_err(|erreur| format!("ecriture impossible a {} : {erreur}", voisin.display()))?;

    // Le remplacant reprend les droits de l'ancien : une configuration que
    // l'utilisateur avait fermee a lui seul ne doit pas s'ouvrir au reste de la
    // machine parce qu'un dossier y a ete ajoute.
    if let Ok(anciens) = std::fs::metadata(chemin).map(|metadonnees| metadonnees.permissions()) {
        let _ = std::fs::set_permissions(&voisin, anciens);
    }

    std::fs::rename(&voisin, chemin).map_err(|erreur| {
        // Le brouillon ne reste pas a cote de la configuration : il n'a rien a
        // y faire, et il ferait douter de laquelle des deux fait foi.
        let _ = std::fs::remove_file(&voisin);
        format!(
            "la configuration a {} n'a pas pu etre remplacee : {erreur}",
            chemin.display()
        )
    })
}
