//! Le service d'interface repond-il, oui ou non.
//!
//! La fenetre ne charge jamais une page venue d'ailleurs : elle charge
//! l'interface servie depuis la machine. Tant que ce service ne repond pas,
//! il n'y a rien a charger, et une fenetre blanche ne dit rien a personne.
//! Cette sonde est ce qui permet de dire ce qui cloche a la place.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// L'hote du service d'interface. La boucle locale, jamais une interface
/// exposee : ce service n'a rien a dire au reseau.
pub const HOTE_INTERFACE: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// Le port du service d'interface. Fixe, jamais tire au hasard ni deplace
/// quand il est pris (FR-070) : l'URL de retour d'autorisation figure telle
/// quelle dans la liste blanche du fournisseur d'identite, et le cookie de
/// session est lie a l'origine, donc au port. Glisser sur un autre port
/// casserait les deux en silence.
pub const PORT_INTERFACE: u16 = 51789;

/// Ce que l'application attend du service avant de renoncer. Le demarrage se
/// compte en secondes, pas en millisecondes : renoncer trop tot afficherait
/// une indisponibilite a un service qui allait repondre.
pub const DELAI_DEMARRAGE: Duration = Duration::from_secs(20);

/// Budget d'une sonde isolee, connexion et reponse comprises.
const DELAI_UNE_SONDE: Duration = Duration::from_millis(800);

/// Temps de repos entre deux sondes, tant que le service demarre.
const REPOS_ENTRE_SONDES: Duration = Duration::from_millis(150);

/// L'origine que charge la fenetre.
///
/// Elle ne lit ni fichier de configuration, ni variable d'environnement :
/// c'est ce qui garantit que la meme origine vaut sur tous les postes, et
/// donc que l'URL de retour d'autorisation reste celle qui est declaree.
pub fn url_de_la_fenetre() -> String {
    format!("http://{HOTE_INTERFACE}:{PORT_INTERFACE}")
}

/// Pourquoi le service d'interface n'est pas joignable.
///
/// Les deux cas se distinguent parce qu'ils ne se corrigent pas de la meme
/// facon : l'un demande de liberer le port, l'autre d'attendre ou de
/// reessayer. Les deux nomment le port, parce que c'est le seul fait que
/// l'utilisateur peut aller verifier lui-meme.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "raison")]
pub enum Indisponibilite {
    /// Quelque chose ecoute sur le port sans repondre a une requete HTTP :
    /// un autre programme le tient.
    PortPris { port: u16 },
    /// Personne n'ecoute, ou le service n'a pas fini de demarrer.
    SansReponse { port: u16, secondes: u64 },
}

impl Indisponibilite {
    /// Le port en cause, quel que soit le cas.
    pub fn port(&self) -> u16 {
        match self {
            Indisponibilite::PortPris { port } => *port,
            Indisponibilite::SansReponse { port, .. } => *port,
        }
    }
}

impl std::fmt::Display for Indisponibilite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Indisponibilite::PortPris { port } => write!(
                f,
                "le port {port} est deja pris par un autre programme. Vibe Map n'en \
                 utilise aucun autre : ferme ce programme, puis reessaie."
            ),
            Indisponibilite::SansReponse { port, secondes } => write!(
                f,
                "le service d'interface n'a pas repondu sur {HOTE_INTERFACE}:{port} en \
                 {secondes} secondes. Reessaie ; s'il ne repond toujours pas, relance \
                 l'application."
            ),
        }
    }
}

/// Vrai quand une connexion s'etablit sur ce port : quelque chose ecoute.
///
/// La question se pose avant de lancer le service, et elle se pose par une
/// connexion, jamais par une prise de port : prendre le port ne serait-ce
/// qu'un instant, c'est risquer que le service lance juste apres le trouve
/// occupe et glisse sur un autre - exactement ce que FR-070 interdit.
pub fn quelqu_un_ecoute(port: u16) -> bool {
    let adresse = SocketAddr::from((HOTE_INTERFACE, port));
    TcpStream::connect_timeout(&adresse, DELAI_UNE_SONDE).is_ok()
}

/// Sonde le service une fois, dans le budget donne.
///
/// Une connexion qui s'etablit ne prouve rien : un programme peut tenir le
/// port sans jamais repondre. Ce qui prouve que l'interface est la, c'est une
/// reponse HTTP.
pub fn sonder(port: u16, budget: Duration) -> Result<(), Indisponibilite> {
    let adresse = SocketAddr::from((HOTE_INTERFACE, port));
    let secondes = budget.as_secs();

    let Ok(mut flux) = TcpStream::connect_timeout(&adresse, budget) else {
        return Err(Indisponibilite::SansReponse { port, secondes });
    };

    let _ = flux.set_write_timeout(Some(budget));
    let _ = flux.set_read_timeout(Some(budget));

    let requete = format!(
        "GET / HTTP/1.1\r\nHost: {HOTE_INTERFACE}:{port}\r\nConnection: close\r\n\r\n"
    );
    if flux.write_all(requete.as_bytes()).is_err() {
        return Err(Indisponibilite::PortPris { port });
    }

    // La ligne de statut suffit : on demande si un service HTTP est la, pas
    // ce qu'il a a dire.
    let mut debut = [0u8; 5];
    match flux.read(&mut debut) {
        Ok(5) if &debut == b"HTTP/" => Ok(()),
        _ => Err(Indisponibilite::PortPris { port }),
    }
}

/// Sonde jusqu'a ce que le service reponde, ou jusqu'a l'expiration du delai.
pub fn attendre_le_service(port: u16, delai: Duration) -> Result<(), Indisponibilite> {
    let echeance = Instant::now() + delai;

    loop {
        match sonder(port, DELAI_UNE_SONDE) {
            Ok(()) => return Ok(()),
            Err(raison) if Instant::now() >= echeance => {
                // Le delai qu'on annonce est celui qu'on a reellement attendu,
                // pas celui d'une sonde isolee.
                return Err(match raison {
                    Indisponibilite::SansReponse { port, .. } => Indisponibilite::SansReponse {
                        port,
                        secondes: delai.as_secs(),
                    },
                    pris => pris,
                });
            }
            Err(_) => std::thread::sleep(REPOS_ENTRE_SONDES),
        }
    }
}
