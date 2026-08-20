//! La fenetre se rouvre la ou on l'a laissee, et jamais au prix du demarrage.
//!
//! Lire cette geometrie, c'est lire un fichier que rien ne protege : le poste
//! peut n'en avoir aucun, l'application a pu etre tuee en pleine ecriture, ou
//! une main l'a edite. Aucun de ces cas ne doit empecher la fenetre de
//! s'ouvrir : ils rendent tous la geometrie par defaut (FR-005).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use bureau::geometrie::{
    self, Geometrie, Position, HAUTEUR_MINIMALE, HAUTEUR_PAR_DEFAUT, LARGEUR_MINIMALE,
    LARGEUR_PAR_DEFAUT,
};

/// Un dossier d'etat a soi, que les tests paralleles ne se partagent pas.
fn dossier_d_essai() -> PathBuf {
    static NUMERO: AtomicU32 = AtomicU32::new(0);
    let dossier = std::env::temp_dir().join(format!(
        "bureau-test-geometrie-{}-{}",
        std::process::id(),
        NUMERO.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dossier).expect("faux dossier d'etat");
    dossier
}

#[test]
fn un_poste_ou_l_application_n_a_jamais_tourne_ouvre_la_fenetre_par_defaut() {
    let dossier = dossier_d_essai();

    let geometrie = geometrie::lire(&geometrie::chemin(&dossier));

    assert_eq!(geometrie, Geometrie::default());
    assert_eq!(geometrie.largeur, LARGEUR_PAR_DEFAUT);
    assert_eq!(geometrie.hauteur, HAUTEUR_PAR_DEFAUT);
    assert_eq!(
        geometrie.position, None,
        "sans passage precedent, c'est au systeme de placer la fenetre"
    );

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn une_geometrie_ecrite_puis_relue_rend_les_memes_valeurs() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    let laissee = Geometrie {
        largeur: 1024.0,
        hauteur: 742.5,
        position: Some(Position { x: -320.0, y: 48.0 }),
    };
    geometrie::ecrire(&chemin, &laissee).expect("geometrie enregistree");

    assert_eq!(geometrie::lire(&chemin), laissee);

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn un_etat_illisible_rend_le_defaut_sans_paniquer() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    // Ce qu'on trouve apres un disque plein ou une machine coupee : des
    // octets qui ne sont plus du JSON.
    std::fs::write(&chemin, b"\x00\x01ceci n'a jamais ete une geometrie").expect("etat abime");

    assert_eq!(geometrie::lire(&chemin), Geometrie::default());

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn un_etat_tronque_rend_le_defaut_sans_paniquer() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    // Le debut exact d'un fichier valide, coupe en plein milieu : c'est ce
    // qu'une ecriture interrompue laisserait derriere elle.
    std::fs::write(&chemin, "{\n  \"largeur\": 1024.0,\n  \"hau").expect("etat tronque");

    assert_eq!(geometrie::lire(&chemin), Geometrie::default());

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn une_taille_invraisemblable_rend_le_defaut() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    // Du JSON parfaitement valide, et pourtant aucune fenetre : une fenetre
    // sans surface ou plus grande que tous les ecrans du monde.
    for absurde in [
        "{\"largeur\": 0.0, \"hauteur\": 860.0}",
        "{\"largeur\": -1280.0, \"hauteur\": -860.0}",
        "{\"largeur\": 1280.0, \"hauteur\": 12.0}",
        "{\"largeur\": 9.9e9, \"hauteur\": 9.9e9}",
    ] {
        std::fs::write(&chemin, absurde).expect("etat absurde");
        assert_eq!(
            geometrie::lire(&chemin),
            Geometrie::default(),
            "une taille que la fenetre ne peut pas prendre renvoie au defaut : {absurde}"
        );
    }

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn une_position_invraisemblable_est_oubliee_mais_la_taille_reste() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    std::fs::write(
        &chemin,
        "{\"largeur\": 1100.0, \"hauteur\": 700.0, \"position\": {\"x\": 9.9e9, \"y\": 0.0}}",
    )
    .expect("position absurde");

    let geometrie = geometrie::lire(&chemin);

    assert_eq!(geometrie.largeur, 1100.0);
    assert_eq!(geometrie.hauteur, 700.0);
    assert_eq!(
        geometrie.position, None,
        "un coin hors de tout ecran laisse le systeme replacer la fenetre, \
         mais la taille relue reste bonne a prendre"
    );

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn la_geometrie_minimale_est_acceptee_telle_quelle() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    // La fenetre reduite au plancher qu'elle impose : c'est une geometrie que
    // l'utilisateur peut reellement choisir, elle doit se retrouver intacte.
    let plancher = Geometrie {
        largeur: LARGEUR_MINIMALE,
        hauteur: HAUTEUR_MINIMALE,
        position: Some(Position { x: 0.0, y: 0.0 }),
    };
    geometrie::ecrire(&chemin, &plancher).expect("geometrie enregistree");

    assert_eq!(geometrie::lire(&chemin), plancher);

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn une_seconde_ecriture_remplace_la_premiere_sans_laisser_de_reste() {
    let dossier = dossier_d_essai();
    let chemin = geometrie::chemin(&dossier);

    geometrie::ecrire(&chemin, &Geometrie::default()).expect("premiere geometrie");
    let deplacee = Geometrie {
        largeur: 1400.0,
        hauteur: 900.0,
        position: Some(Position { x: 120.0, y: 60.0 }),
    };
    geometrie::ecrire(&chemin, &deplacee).expect("seconde geometrie");

    assert_eq!(geometrie::lire(&chemin), deplacee);

    // Le fichier voisin de l'ecriture atomique ne survit pas a son renommage :
    // un reste sur le disque signalerait une ecriture qui n'a pas abouti.
    let restes: Vec<_> = std::fs::read_dir(&dossier)
        .expect("dossier d'etat")
        .flatten()
        .map(|entree| entree.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(restes, vec!["fenetre.json".to_string()]);

    std::fs::remove_dir_all(&dossier).ok();
}

#[test]
fn le_dossier_d_etat_est_cree_s_il_manque() {
    let dossier = dossier_d_essai();
    let absent = dossier.join("jamais-creee");

    geometrie::ecrire(&geometrie::chemin(&absent), &Geometrie::default())
        .expect("le dossier d'etat se cree au premier enregistrement");

    assert_eq!(
        geometrie::lire(&geometrie::chemin(&absent)),
        Geometrie::default()
    );

    std::fs::remove_dir_all(&dossier).ok();
}
