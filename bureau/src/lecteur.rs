//! Le lecteur, embarque dans l'application.
//!
//! Il demarre quand l'application s'ouvre et meurt quand elle se ferme
//! (FR-006, FR-007). Ce n'est pas un programme lance a cote : c'est le meme
//! lecteur que `vibemap`, appele comme bibliotheque, et il tourne DANS le
//! processus de l'application.
//!
//! POURQUOI dans le meme processus. #57 a paye cher la lecon inverse avec le
//! service d'interface : un programme enfant survit a une application tuee par
//! un signal, parce qu'aucun destructeur ne se deroule alors. Ici, il n'y a pas
//! d'enfant a tuer. L'application qui disparait - proprement, par un signal, ou
//! par `kill -9` - emmene le lecteur avec elle, et le noyau ferme le
//! descripteur qui portait le verrou du poste. Aucun processus de lecture ne
//! peut donc rester derriere, et il n'y a rien a nettoyer soi-meme.
//!
//! Le verrou, lui, n'est pas pose ici : c'est le lecteur partage qui le prend,
//! par le module `verrou` du daemon, exactement comme le fait le binaire
//! (FR-008, FR-054). L'application ne fait que rapporter son refus a la
//! fenetre.
//!
//! Ce que ce module sait dire d'autre : ou en est le lecteur a l'instant ou on
//! le demande, et comment le remettre en marche sans fermer l'application
//! (FR-009, FR-010). L'etat n'est jamais fige au demarrage - une boucle peut
//! cesser de tourner bien apres -, et la relance passe par la meme porte que le
//! demarrage : elle laisse tomber le lecteur d'avant, verrou compris, avant d'en
//! prendre un autre.

use std::path::Path;
use std::time::Duration;

use vibemap::lecteur::{Arret, Lecteur, LecteurError};
use vibemap::{Config, Tenant, Verrou, VerrouError};

/// Le nom sous lequel l'application se presente dans le verrou du poste. C'est
/// lui que le binaire affiche quand il refuse de demarrer a cote d'elle.
pub const VEHICULE: &str = "l'application de bureau";

/// Ce que l'application laisse au lecteur pour s'arreter avant de l'abandonner.
///
/// Meme duree que pour le service d'interface : de quoi finir un envoi en
/// cours, pas de quoi retenir une application qui se ferme.
const DELAI_D_ARRET: Duration = Duration::from_secs(3);

/// Ce qui tient le poste, quand on sait le nommer.
///
/// Les faits bruts, pas une phrase : c'est la fenetre qui l'ecrit, en francais
/// accentue et avec l'heure du poste. Meme partage qu'en #57 pour les cas
/// nommes de l'interface.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TenantDuPoste {
    /// « vibemap », « l'application de bureau »…
    pub vehicule: String,
    /// Le processus, tel que le Moniteur d'activite le montre.
    pub pid: i32,
    /// Depuis quand, en RFC 3339.
    pub depuis: String,
}

/// Pourquoi le lecteur ne tourne pas, dans les termes ou l'utilisateur peut y
/// faire quelque chose.
///
/// Les cas nommes ne sont pas des details de plus : chacun se corrige
/// autrement, et la fenetre en ecrit la phrase elle-meme, en francais accentue.
/// `Panne` est le fourre-tout de ce qui reste, ou seule `raison` parle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CasEchec {
    /// Un autre lecteur tient le poste : il n'y a rien a reparer, il y a un
    /// lecteur a arreter.
    PosteTenu,
    /// Le jeton de la machine n'a pas pu etre lu au trousseau - le plus
    /// souvent, une autorisation refusee apres une recompilation.
    JetonRefuse,
    /// Le lecteur tournait, et sa boucle s'est arretee sans qu'on l'ait
    /// demande. Rien ne part plus de cette machine, et rien ne le dirait.
    ArretInattendu,
    /// Tout le reste : configuration illisible, machinerie qui ne se monte pas.
    Panne,
}

/// Pourquoi le lecteur ne tourne pas, tel que la fenetre le montre.
///
/// `raison` dit les faits que seule l'application connait ; `cas` dit lequel
/// des chemins connus a echoue, pour que la fenetre sache quoi proposer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EchecLecteur {
    pub cas: CasEchec,
    pub raison: String,
    /// `None` quand le poste n'est pas en cause, ou quand celui qui le tient
    /// n'a pas laisse de marque lisible : le refus tient quand meme, mais la
    /// fenetre ne peut alors nommer personne.
    pub tenant: Option<TenantDuPoste>,
}

impl EchecLecteur {
    /// Le lecteur tournait, et sa boucle ne tourne plus.
    ///
    /// Personne ne l'a demande : ni la fermeture de l'application, qui emporte
    /// tout avec elle, ni une relance, qui remet aussitot un lecteur en place.
    /// Reste ce qui casse - une panique dans la boucle, une machinerie tombee -
    /// et cela ne se voit nulle part ailleurs : la carte continue de s'afficher,
    /// alimentee par ce que la machine avait deja envoye.
    pub fn arret_inattendu() -> Self {
        EchecLecteur {
            cas: CasEchec::ArretInattendu,
            raison: "le lecteur s'est arrete de lui-meme : cette machine n'envoie plus rien."
                .to_string(),
            tenant: None,
        }
    }
}

impl From<&LecteurError> for EchecLecteur {
    fn from(erreur: &LecteurError) -> Self {
        let cas = match erreur {
            _ if erreur.poste_tenu() => CasEchec::PosteTenu,
            LecteurError::Jeton { .. } => CasEchec::JetonRefuse,
            _ => CasEchec::Panne,
        };

        EchecLecteur {
            cas,
            raison: erreur.to_string(),
            tenant: tenant_du_poste(erreur),
        }
    }
}

/// Les faits du tenant, quand l'echec est un poste deja pris et que la marque
/// se relit.
fn tenant_du_poste(erreur: &LecteurError) -> Option<TenantDuPoste> {
    let LecteurError::Poste(VerrouError::DejaPris {
        tenant: Tenant::Nomme(marque),
        ..
    }) = erreur
    else {
        return None;
    };

    Some(TenantDuPoste {
        vehicule: marque.vehicule.clone(),
        pid: marque.pid,
        depuis: marque.depuis.clone(),
    })
}

/// Ou en est le lecteur, tel que la fenetre le lit.
///
/// Quatre etats, et pas un de plus : il s'installe, il tourne, il ne tourne
/// pas, ou il n'a pas pu tourner et on dit pourquoi. Rien de tout cela ne passe
/// par la base : ce sont des faits du poste, que seule l'application connait.
///
/// `EnDemarrage` n'est pas une nuance de `Arrete` : le demarrage lit le jeton
/// au trousseau, ce qui peut ouvrir une boite de dialogue du systeme et durer
/// aussi longtemps qu'il faut a l'utilisateur pour y repondre. La fenetre a
/// besoin de le distinguer d'un lecteur qui ne demarrera jamais.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatLecteur {
    EnDemarrage,
    EnMarche,
    Arrete,
    EnEchec(EchecLecteur),
}

/// L'etat a montrer, selon ce qu'on vient d'observer du lecteur.
///
/// `tourne` vaut `None` quand l'application ne tient aucun lecteur - il n'a pas
/// encore demarre, ou il n'a pas pu -, et sinon dit si sa boucle tourne
/// toujours. Le dernier etat connu ne sert que dans ce premier cas : des qu'il
/// y a un lecteur a regarder, c'est lui qui a raison, et non ce qu'on avait
/// note de son demarrage.
///
/// POURQUOI decider ici, et a chaque fois qu'on demande : un lecteur peut
/// cesser de tourner longtemps apres avoir demarre, et un etat fige au
/// demarrage laisserait la fenetre annoncer une machine qui bat alors qu'elle
/// s'est tue (FR-010).
pub fn etat_a_montrer(tourne: Option<bool>, dernier: &EtatLecteur) -> EtatLecteur {
    match tourne {
        Some(true) => EtatLecteur::EnMarche,
        Some(false) => EtatLecteur::EnEchec(EchecLecteur::arret_inattendu()),
        None => dernier.clone(),
    }
}

/// Le lecteur en marche. Il s'arrete avec cette valeur.
///
/// Elle tient la machinerie d'execution du lecteur, et non le fil qui l'a
/// lancee : c'est ce qui le fait durer autant que l'application, et disparaitre
/// avec elle.
pub struct LecteurEmbarque {
    arret: Arret,
    /// La boucle du lecteur, telle que la machinerie la fait tourner. Elle
    /// n'est jamais attendue : on ne la garde que pour savoir si elle tourne
    /// encore.
    boucle: tokio::task::JoinHandle<()>,
    /// `None` seulement pendant le destructeur, le temps de rendre la
    /// machinerie a la fonction qui l'arrete.
    machinerie: Option<tokio::runtime::Runtime>,
}

impl LecteurEmbarque {
    /// Vrai tant que la boucle du lecteur n'a pas rendu la main.
    ///
    /// C'est la seule question que l'application sait poser sans rien
    /// interrompre : la boucle vit dans ce processus, et il n'y a pas de
    /// processus voisin a aller sonder.
    pub fn tourne_encore(&self) -> bool {
        !self.boucle.is_finished()
    }
}

/// Demarre le lecteur du poste, ou dit ce qui l'en empeche.
///
/// La configuration et le verrou sont a leurs emplacements habituels : ceux du
/// binaire en ligne de commande, les memes pour les deux vehicules.
pub fn demarrer_le_lecteur() -> Result<LecteurEmbarque, EchecLecteur> {
    demarrer(&Config::chemin_par_defaut(), &Verrou::chemin_par_defaut())
}

/// Le meme demarrage, sur des emplacements donnes.
///
/// Les chemins sont passes explicitement pour que le refus d'un poste deja tenu
/// s'eprouve sans toucher au poste reel de la machine.
pub fn demarrer(
    chemin_config: &Path,
    chemin_du_verrou: &Path,
) -> Result<LecteurEmbarque, EchecLecteur> {
    // La preparation d'abord : elle prend le verrou, et un poste deja tenu se
    // dit sans avoir monte la moindre machinerie.
    let lecteur = Lecteur::preparer(chemin_config, chemin_du_verrou, VEHICULE)
        .map_err(|erreur| EchecLecteur::from(&erreur))?;

    // Un seul fil d'execution : le lecteur attend des horloges et le reseau, il
    // ne calcule rien. La machinerie est construite ici, avant de rien
    // promettre : une machinerie qu'on ne sait pas monter est un echec de
    // demarrage, pas un lecteur silencieusement mort.
    let machinerie = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .thread_name("lecteur-vibemap")
        .build()
        .map_err(|erreur| EchecLecteur {
            cas: CasEchec::Panne,
            raison: format!("le lecteur n'a pas pu demarrer sur cette machine : {erreur}."),
            tenant: None,
        })?;

    let arret = Arret::new();
    let sien = arret.clone();
    let boucle = machinerie.spawn(async move { lecteur.tourner(sien).await });

    Ok(LecteurEmbarque {
        arret,
        boucle,
        machinerie: Some(machinerie),
    })
}

/// Remet le lecteur en marche sans quitter l'application (FR-010), et rend
/// l'etat qui en resulte.
///
/// Sans effet sur un lecteur qui tourne encore : le clic de trop ne doit pas
/// couper le battement pour le seul plaisir de le reprendre.
pub fn relancer_le_lecteur(lecteur: &mut Option<LecteurEmbarque>) -> EtatLecteur {
    relancer(
        lecteur,
        &Config::chemin_par_defaut(),
        &Verrou::chemin_par_defaut(),
    )
}

/// La meme relance, sur des emplacements donnes.
pub fn relancer(
    lecteur: &mut Option<LecteurEmbarque>,
    chemin_config: &Path,
    chemin_du_verrou: &Path,
) -> EtatLecteur {
    if lecteur.as_ref().is_some_and(LecteurEmbarque::tourne_encore) {
        return EtatLecteur::EnMarche;
    }

    reprendre(lecteur, chemin_config, chemin_du_verrou)
}

/// Arrete le lecteur, et rend l'etat qui en resulte (FR-015).
///
/// C'est ce que la deconnexion declenche : plus aucune session ouverte ne
/// justifie qu'une machine continue d'emettre. Laisser tomber la valeur suffit :
/// c'est elle qui tient la machinerie d'execution, et le verrou du poste tombe
/// avec elle. Aucun processus de lecture ne reste derriere, et il n'y a rien a
/// nettoyer soi-meme.
///
/// Sans effet quand aucun lecteur ne tourne : se deconnecter d'une application
/// dont le lecteur n'avait pas demarre n'est pas un echec.
pub fn arreter(lecteur: &mut Option<LecteurEmbarque>) -> EtatLecteur {
    lecteur.take();
    EtatLecteur::Arrete
}

/// Fait repartir le lecteur de zero pour qu'il relise sa configuration, meme
/// s'il tourne encore (FR-033, FR-034).
///
/// POURQUOI un depart de zero et non un reglage a chaud : le lecteur lit sa
/// configuration - les dossiers surveilles compris - au moment de se preparer,
/// et une seule fois. Rien, dans sa boucle, ne va la relire. Le faire repartir
/// est donc ce qui fait qu'un dossier ajoute est surveille TOUT DE SUITE, sans
/// que l'application ait a se fermer.
///
/// Et la cartographie suit : elle a lieu des le demarrage du lecteur, avant sa
/// premiere boucle. Le dossier ajoute est donc parcouru dans la foulee, sans
/// attendre les cinq minutes de la cartographie periodique - c'est ce qui tient
/// la promesse de la minute (FR-034).
///
/// A n'appeler que quand la configuration a change : reprendre un lecteur en
/// marche pour rien couperait le battement de la machine le temps du
/// remplacement.
pub fn reprendre_la_configuration(lecteur: &mut Option<LecteurEmbarque>) -> EtatLecteur {
    reprendre(
        lecteur,
        &Config::chemin_par_defaut(),
        &Verrou::chemin_par_defaut(),
    )
}

/// Le meme depart de zero, sur des emplacements donnes.
pub fn reprendre(
    lecteur: &mut Option<LecteurEmbarque>,
    chemin_config: &Path,
    chemin_du_verrou: &Path,
) -> EtatLecteur {
    // Celui d'avant est laisse tomber d'abord, et jusqu'au bout : c'est lui qui
    // tient le verrou du poste, et le suivant se le refuserait a lui-meme.
    lecteur.take();

    match demarrer(chemin_config, chemin_du_verrou) {
        Ok(nouveau) => {
            *lecteur = Some(nouveau);
            EtatLecteur::EnMarche
        }
        Err(echec) => EtatLecteur::EnEchec(echec),
    }
}

impl Drop for LecteurEmbarque {
    fn drop(&mut self) {
        self.arret.demander();

        // On laisse le lecteur finir ce qu'il avait commence, puis on n'attend
        // plus : une application qui se ferme ne doit pas rester suspendue a un
        // envoi que le reseau ne rendra jamais. Ce qui reste tombe avec la
        // machinerie, le verrou du poste compris.
        if let Some(machinerie) = self.machinerie.take() {
            machinerie.shutdown_timeout(DELAI_D_ARRET);
        }
    }
}
