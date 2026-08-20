//! Les dossiers surveilles, tels que l'ecran Reglages les voit.
//!
//! FR-059 partage les sources, et ce partage est la raison d'etre de ce
//! module : la liste des dossiers, leur compte de depots, leur lisibilite et
//! leur autorisation sont des faits du POSTE. Ils n'existent nulle part
//! ailleurs que sur cette machine - la base ne porte ni les chemins surveilles,
//! ni le fait qu'un dossier ait ete renomme il y a dix secondes -, et ils ne
//! sortent pas d'ici. L'heure de la derniere cartographie, elle, ne passe pas
//! par la : elle vit en base avec le catalogue, et l'ecran l'y lit.
//!
//! POURQUOI ces faits se relevent a chaque demande, plutot que de se noter au
//! demarrage : la question que l'utilisateur pose en ouvrant Reglages est
//! « pourquoi ce depot n'apparait pas », et la reponse la plus frequente est un
//! dossier renomme depuis. Un etat fige repondrait a cote.
//!
//! La borne du pont tient ici comme ailleurs : ce module ne sait pas ouvrir un
//! chemin qu'on lui donne. Il lit la liste que la configuration du lecteur
//! porte deja, et rien d'autre.

use std::path::Path;

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

/// Ce que la commande du pont rend a l'ecran.
///
/// Une configuration illisible n'est pas une liste vide : l'une dit « aucun
/// dossier n'est surveille », l'autre « on ne sait pas ce qui est surveille ».
/// L'ecran ne doit pas pouvoir les confondre, d'ou deux formes distinctes
/// plutot qu'un vecteur vide et un message a cote.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum Surveillance {
    Lue { dossiers: Vec<DossierSurveille> },
    SansConfiguration { raison: String },
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
    let dossiers = config
        .roots
        .iter()
        .zip(config.racines())
        .map(|(brut, emplacement)| regarder(brut, &emplacement))
        .collect();

    Surveillance::Lue { dossiers }
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
