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
//! porte deja, et il n'y ajoute que ce que le selecteur du systeme a rendu -
//! un dossier que l'utilisateur a designe lui-meme, jamais un chemin que la
//! fenetre aurait choisi (FR-031).
//!
//! POURQUOI l'ajout passe par le fichier de configuration : c'est la meme
//! liste que le lecteur lit, et il n'y en a pas deux. Le fichier reste donc le
//! support de stockage - il cesse seulement d'etre une porte : personne n'a
//! plus a l'ouvrir pour surveiller un dossier de plus (FR-036).

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

/* ---------- ajouter un dossier (FR-031, FR-036) ---------- */

/// Ce que l'ajout d'un dossier a donne, tel que l'ecran le lit.
///
/// Trois issues, et elles ne se ressemblent pas. Un selecteur ferme sans rien
/// choisir n'est pas un echec : c'est un geste repris, et l'ecran n'a rien a
/// annoncer. Une ecriture qui echoue, si : la liste n'a pas bouge, et
/// l'utilisateur doit savoir que son dossier n'est pas surveille.
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
/// Ce que cette tranche ne fait pas : juger le dossier choisi. Un doublon, un
/// dossier emboite dans un autre - c'est la regle de FR-037 et FR-075, et elle
/// vient apres. `Echoue` ne dit ici que ce qui empeche d'ecrire.
pub fn ajouter(chemin_config: &Path, choisi: &Path) -> Ajout {
    let ecrit = abreger(choisi, &maison());

    match inscrire(chemin_config, &ecrit) {
        Ok(()) => Ajout::Ajoute {
            chemin: ecrit,
            surveillance: dossiers_surveilles(chemin_config),
        },
        Err(raison) => Ajout::Echoue { raison },
    }
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
