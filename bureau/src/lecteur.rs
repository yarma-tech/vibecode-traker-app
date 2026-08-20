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

/// Pourquoi le lecteur n'a pas demarre, tel que la fenetre le montre.
///
/// `poste_tenu` n'est pas un detail de plus : c'est le seul cas ou il n'y a
/// rien a reparer mais un lecteur a arreter, et la fenetre le dit autrement
/// d'une panne. `raison` ne sert qu'au reste, que seule l'application sait
/// dire.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EchecLecteur {
    pub raison: String,
    pub poste_tenu: bool,
    /// `None` quand le poste n'est pas en cause, ou quand celui qui le tient
    /// n'a pas laisse de marque lisible : le refus tient quand meme, mais la
    /// fenetre ne peut alors nommer personne.
    pub tenant: Option<TenantDuPoste>,
}

impl From<&LecteurError> for EchecLecteur {
    fn from(erreur: &LecteurError) -> Self {
        EchecLecteur {
            raison: erreur.to_string(),
            poste_tenu: erreur.poste_tenu(),
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
/// Trois etats, et pas un de plus : il tourne, il ne tourne pas, ou il n'a pas
/// pu demarrer et on dit pourquoi. Rien de tout cela ne passe par la base : ce
/// sont des faits du poste, que seule l'application connait.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatLecteur {
    EnMarche,
    Arrete,
    EnEchec(EchecLecteur),
}

/// Le lecteur en marche. Il s'arrete avec cette valeur.
///
/// Elle tient la machinerie d'execution du lecteur, et non le fil qui l'a
/// lancee : c'est ce qui le fait durer autant que l'application, et disparaitre
/// avec elle.
pub struct LecteurEmbarque {
    arret: Arret,
    /// `None` seulement pendant le destructeur, le temps de rendre la
    /// machinerie a la fonction qui l'arrete.
    machinerie: Option<tokio::runtime::Runtime>,
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
            raison: format!(
                "le lecteur n'a pas pu demarrer sur cette machine : {erreur}. \
                 Relance l'application."
            ),
            poste_tenu: false,
            tenant: None,
        })?;

    let arret = Arret::new();
    let sien = arret.clone();
    machinerie.spawn(async move { lecteur.tourner(sien).await });

    Ok(LecteurEmbarque {
        arret,
        machinerie: Some(machinerie),
    })
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
