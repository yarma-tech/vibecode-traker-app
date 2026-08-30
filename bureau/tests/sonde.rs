//! La sonde dit si le service d'interface repond, et nomme le port quand il
//! ne repond pas.
//!
//! C'est ce qui separe une fenetre qui s'explique d'une fenetre blanche
//! (FR-004, FR-070). Aucun port n'est ecrit en dur ici : chaque cas prend un
//! port que le systeme vient de donner, sans quoi le test dependrait de ce
//! qui tourne sur le poste.

use std::io::Write;
use std::net::TcpListener;
use std::time::Duration;

use bureau::sonde::{self, Indisponibilite};

/// Un port pris par un programme qui n'a rien a dire. Le `TcpListener` rendu
/// doit rester en vie : le laisser tomber libererait le port.
fn port_occupe_sans_reponse() -> (TcpListener, u16) {
    let ecoute = TcpListener::bind(("127.0.0.1", 0)).expect("prendre un port libre");
    let port = ecoute.local_addr().expect("adresse du port pris").port();
    (ecoute, port)
}

/// Un port ou un serveur repond, comme le ferait le service d'interface.
fn port_qui_repond() -> u16 {
    let ecoute = TcpListener::bind(("127.0.0.1", 0)).expect("prendre un port libre");
    let port = ecoute.local_addr().expect("adresse du serveur").port();

    std::thread::spawn(move || {
        for flux in ecoute.incoming() {
            let Ok(mut flux) = flux else { continue };
            let _ = flux.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        }
    });

    port
}

#[test]
fn un_port_pris_rend_une_indisponibilite_qui_nomme_ce_port() {
    let (_ecoute, port) = port_occupe_sans_reponse();

    let raison = sonde::sonder(port, Duration::from_millis(300))
        .expect_err("un programme qui tient le port sans repondre n'est pas l'interface");

    assert_eq!(
        raison,
        Indisponibilite::PortPris { port },
        "le port est tenu par un autre programme, pas simplement muet"
    );
    assert!(
        raison.to_string().contains(&port.to_string()),
        "le message doit nommer le port pour que l'utilisateur puisse aller y voir : {raison}"
    );
}

#[test]
fn un_service_qui_repond_rend_la_disponibilite() {
    let port = port_qui_repond();

    sonde::sonder(port, Duration::from_secs(2))
        .expect("un serveur qui repond en HTTP est une interface joignable");
}

/// Personne n'ecoute : ce n'est pas la meme chose qu'un port tenu par un
/// autre programme, et cela ne se corrige pas de la meme facon.
#[test]
fn un_port_ou_personne_n_ecoute_se_distingue_d_un_port_pris() {
    let ecoute = TcpListener::bind(("127.0.0.1", 0)).expect("prendre un port libre");
    let port = ecoute.local_addr().expect("adresse du port").port();
    drop(ecoute);

    let raison = sonde::sonder(port, Duration::from_millis(300))
        .expect_err("aucun service n'ecoute sur ce port");

    assert!(
        matches!(raison, Indisponibilite::SansReponse { port: p, .. } if p == port),
        "un port sans personne dessus est un service qui n'a pas repondu : {raison}"
    );
    assert!(
        raison.to_string().contains(&port.to_string()),
        "ce message aussi nomme le port : {raison}"
    );
}

#[test]
fn l_attente_renonce_au_bout_du_delai_en_nommant_le_port() {
    let (_ecoute, port) = port_occupe_sans_reponse();

    let raison = sonde::attendre_le_service(port, Duration::from_millis(200))
        .expect_err("le port est tenu par un programme qui ne repondra jamais");

    assert_eq!(raison.port(), port);
}

#[test]
fn l_attente_rend_la_main_des_que_le_service_repond() {
    let port = port_qui_repond();

    sonde::attendre_le_service(port, Duration::from_secs(5))
        .expect("le service repond des la premiere sonde");
}

/// Avant de lancer quoi que ce soit, l'application demande si le port est
/// deja tenu - par une connexion, jamais en prenant le port elle-meme.
#[test]
fn quelqu_un_ecoute_voit_le_port_pris_et_le_port_libre() {
    let (_ecoute, port_pris) = port_occupe_sans_reponse();
    assert!(sonde::quelqu_un_ecoute(port_pris));

    let libre = TcpListener::bind(("127.0.0.1", 0)).expect("prendre un port libre");
    let port_libre = libre.local_addr().expect("adresse du port").port();
    drop(libre);
    assert!(!sonde::quelqu_un_ecoute(port_libre));
}
