//! La reprise apres une coupure : file d'attente locale et temporisation.
//!
//! Rien ici ne parle au reseau. Une file bornee et un backoff sont deux petites
//! machines a etats : on les eprouve a sec, ce qui les rend verifiables.

use std::time::Duration;
use vibemap::reprise::{Backoff, Debordement, FileAttente};

#[test]
fn la_file_rend_les_elements_dans_l_ordre_d_arrivee() {
    let mut file = FileAttente::new(3);
    file.pousser(1);
    file.pousser(2);
    file.pousser(3);

    assert_eq!(file.len(), 3);
    assert_eq!(file.tirer(), Some(1));
    assert_eq!(file.tirer(), Some(2));
    assert_eq!(file.tirer(), Some(3));
    assert_eq!(file.tirer(), None);
    assert!(file.est_vide());
}

#[test]
fn la_file_plafonne_et_jette_le_plus_ancien_quand_elle_deborde() {
    let mut file = FileAttente::new(2);
    assert_eq!(file.pousser(1), Debordement::Aucun);
    assert_eq!(file.pousser(2), Debordement::Aucun);

    // Pleine : pour laisser entrer 3, le plus ancien (1) tombe. On garde le
    // plus recent, car c'est lui qui represente l'etat courant de la carte.
    assert_eq!(file.pousser(3), Debordement::Jete(1));
    assert_eq!(file.len(), 2);
    assert_eq!(file.tirer(), Some(2));
    assert_eq!(file.tirer(), Some(3));
}

#[test]
fn regarder_le_prochain_ne_le_retire_pas() {
    let mut file = FileAttente::new(2);
    file.pousser(7);
    assert_eq!(file.premier(), Some(&7));
    assert_eq!(file.len(), 1, "regarder ne consomme pas");
}

#[test]
fn le_backoff_double_a_chaque_echec_puis_plafonne() {
    let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));

    assert_eq!(backoff.prochain(), Duration::from_secs(1));
    assert_eq!(backoff.prochain(), Duration::from_secs(2));
    assert_eq!(backoff.prochain(), Duration::from_secs(4));
    assert_eq!(backoff.prochain(), Duration::from_secs(8));
    assert_eq!(backoff.prochain(), Duration::from_secs(16));
    // 32 depasserait le plafond : on ne martele jamais plus vite que 30 s.
    assert_eq!(backoff.prochain(), Duration::from_secs(30));
    assert_eq!(backoff.prochain(), Duration::from_secs(30));
}

#[test]
fn un_succes_remet_le_backoff_a_son_pas_de_depart() {
    let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
    backoff.prochain();
    backoff.prochain();

    backoff.reset();

    assert_eq!(backoff.prochain(), Duration::from_secs(1), "apres un succes on repart doucement");
}
