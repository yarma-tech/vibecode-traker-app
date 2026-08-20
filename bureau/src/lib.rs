//! Vibe Map, application de bureau.
//!
//! L'application ne dessine pas la carte : elle la sert. Le meme `web/` que
//! le site, demarre sur la boucle locale a un port fixe, puis charge dans sa
//! fenetre. Ce qui vit ici, c'est ce qui n'existe que sur le poste : le
//! demarrage du service, la sonde qui dit s'il repond, ce qu'on affiche quand
//! il ne repond pas, le lecteur qu'elle embarque, et ou l'utilisateur avait
//! laisse sa fenetre.

pub mod autorisation;
pub mod dossiers;
pub mod geometrie;
pub mod lecteur;
pub mod service;
pub mod sonde;

use service::{ErreurService, Service};
use sonde::{Indisponibilite, DELAI_DEMARRAGE, PORT_INTERFACE};

/// Ce que la fenetre affiche quand l'interface n'a pas pu s'ouvrir.
///
/// La fenetre compose elle-meme la phrase des deux cas nommes - port pris,
/// service muet -, parce que ce sont ceux que l'utilisateur rencontre et
/// qu'ils se lisent mieux en francais accentue. `detail` ne sert qu'au reste,
/// que seule l'application sait dire.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Echec {
    pub cas: &'static str,
    pub port: u16,
    pub secondes: u64,
    pub detail: String,
}

impl From<Indisponibilite> for Echec {
    fn from(raison: Indisponibilite) -> Self {
        let cas = match raison {
            Indisponibilite::PortPris { .. } => "port_pris",
            Indisponibilite::SansReponse { .. } => "sans_reponse",
        };
        let secondes = match raison {
            Indisponibilite::SansReponse { secondes, .. } => secondes,
            Indisponibilite::PortPris { .. } => 0,
        };
        Echec {
            cas,
            port: raison.port(),
            secondes,
            detail: raison.to_string(),
        }
    }
}

impl From<ErreurService> for Echec {
    fn from(erreur: ErreurService) -> Self {
        Echec {
            cas: "service",
            port: PORT_INTERFACE,
            secondes: 0,
            detail: erreur.to_string(),
        }
    }
}

/// Ouvre l'interface sur le port fixe et rend le service qui la sert.
///
/// Tant que la valeur rendue est tenue, le service tourne ; la laisser
/// tomber l'arrete.
///
/// L'ordre compte : on regarde d'abord si quelqu'un tient deja le port, et on
/// renonce dans ce cas sans rien lancer. Un serveur de developpement a qui on
/// donne un port occupe en prend un autre de lui-meme, et c'est precisement le
/// repli silencieux que FR-070 interdit.
pub fn ouvrir_l_interface() -> Result<Service, Echec> {
    if sonde::quelqu_un_ecoute(PORT_INTERFACE) {
        return Err(Indisponibilite::PortPris {
            port: PORT_INTERFACE,
        }
        .into());
    }

    let service = service::lancer(PORT_INTERFACE)?;
    // Un service qui ne repond pas dans le delai est arrete en meme temps que
    // l'echec est rendu : sans cela, il tiendrait le port et le prochain essai
    // se heurterait a lui.
    sonde::attendre_le_service(PORT_INTERFACE, DELAI_DEMARRAGE)?;
    Ok(service)
}
