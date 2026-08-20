//! Comportement : l'application n'ouvre jamais un second poste, et la fenetre
//! sait toujours ou en est le lecteur.
//!
//! Le verrou lui-meme est eprouve dans `daemon/tests/verrou.rs`, et la prise du
//! poste par le lecteur dans `daemon/tests/lecteur.rs`. Ce qui se verifie ici,
//! c'est la consequence cote application : ce qu'elle fait du refus, et ce
//! qu'elle en montre.
//!
//! Toujours sur des fichiers temporaires : un test ne doit ni condamner le
//! poste de la machine, ni le prendre au lecteur de l'utilisateur pendant qu'il
//! tourne.

use bureau::lecteur::{CasEchec, EchecLecteur, EtatLecteur, TenantDuPoste};
use std::path::PathBuf;
use vibemap::lecteur::LecteurError;
use vibemap::trousseau::TrousseauError;
use vibemap::Verrou;

fn chemin_temporaire(quoi: &str) -> PathBuf {
    std::env::temp_dir().join(format!("bureau-lecteur-{}-{quoi}", std::process::id()))
}

/// Une configuration qui ne regarde rien de ce qui vit sur cette machine :
/// aucune racine a cartographier, un dossier de journaux vide, et une adresse
/// ou personne ne repond. Le lecteur tourne pour de vrai, sans rien toucher.
fn config_temporaire(quoi: &str) -> PathBuf {
    let journaux = chemin_temporaire(&format!("journaux-{quoi}"));
    std::fs::create_dir_all(&journaux).expect("dossier de journaux de test");

    let chemin = chemin_temporaire(&format!("{quoi}.toml"));
    std::fs::write(
        &chemin,
        format!(
            "supabase_url = \"http://127.0.0.1:1\"\n\
             machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
             label = \"machine de test\"\n\
             roots = []\n\
             claude_projects = \"{}\"\n\
             interval_seconds = 3600\n\
             scan_seconds = 3600\n\
             journal_seconds = 3600\n\
             worktree_seconds = 3600\n\
             commit_seconds = 3600\n",
            journaux.display()
        ),
    )
    .expect("ecriture de la configuration de test");
    chemin
}

/// Le lecteur embarque tourne pour de vrai : il tient le poste tant que
/// l'application le tient, et le rend des qu'elle le laisse tomber.
///
/// C'est la promesse de FR-006 et FR-007 vue de l'exterieur. Elle se verifie
/// ici sans processus a surveiller, et c'est le fond de l'affaire : le lecteur
/// vit DANS l'application, il n'y a pas d'enfant qui puisse lui survivre.
#[test]
fn le_lecteur_de_l_application_tient_le_poste_puis_le_rend_en_se_fermant() {
    // Le jeton vient de l'environnement : le trousseau du systeme n'a rien a
    // faire dans un test, et il n'existe pas sur une machine d'integration.
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");

    let config = config_temporaire("marche");
    let verrou = chemin_temporaire("marche.lock");

    let lecteur = bureau::lecteur::demarrer(&config, &verrou).expect("le poste est libre");

    Verrou::prendre(&verrou, "vibemap")
        .expect_err("tant que l'application tourne, personne d'autre ne prend le poste");

    // Fermer l'application, c'est laisser tomber cette valeur : rien d'autre
    // n'est a faire, et rien ne reste derriere.
    drop(lecteur);

    Verrou::prendre(&verrou, "vibemap")
        .expect("le poste doit etre libre des que l'application s'est fermee");

    let _ = std::fs::remove_file(&config);
    let _ = std::fs::remove_file(&verrou);
}

/// Le poste tenu par un `vibemap` de terminal : l'application ne demarre pas de
/// second lecteur, et elle nomme ce qui tient le poste - sans quoi l'utilisateur
/// n'aurait aucun moyen de savoir quoi arreter.
#[test]
fn un_poste_deja_tenu_refuse_le_lecteur_de_l_application_en_nommant_le_tenant() {
    let config = config_temporaire("refus");
    let verrou = chemin_temporaire("refus.lock");

    let _terminal = Verrou::prendre(&verrou, "vibemap").expect("le poste est libre");

    let echec = bureau::lecteur::demarrer(&config, &verrou)
        .err()
        .expect("l'application ne doit pas demarrer un second lecteur");

    assert_eq!(
        echec.cas,
        CasEchec::PosteTenu,
        "le cas se distingue d'une panne : il n'y a rien a reparer, il y a un lecteur a \
         arreter. Obtenu : {echec:?}"
    );
    let tenant = echec
        .tenant
        .as_ref()
        .expect("la fenetre doit pouvoir nommer ce qui tient le poste");
    assert_eq!(tenant.vehicule, "vibemap");
    assert_eq!(tenant.pid, std::process::id() as i32);
    assert!(
        !tenant.depuis.is_empty(),
        "la fenetre dit depuis quand le poste est tenu"
    );
    assert!(
        echec.raison.contains("vibemap"),
        "la raison reste lisible telle quelle, obtenu : {}",
        echec.raison
    );

    let _ = std::fs::remove_file(&config);
    let _ = std::fs::remove_file(&verrou);
}

/// Une configuration absente n'est pas un poste tenu : les deux se corrigent
/// autrement, et la fenetre ne doit pas envoyer chercher un lecteur imaginaire.
#[test]
fn une_panne_ordinaire_ne_se_donne_pas_pour_un_poste_tenu() {
    let config = chemin_temporaire("absente.toml");
    let verrou = chemin_temporaire("libre.lock");

    let echec = bureau::lecteur::demarrer(&config, &verrou)
        .err()
        .expect("sans configuration, le lecteur ne demarre pas");

    assert_eq!(echec.cas, CasEchec::Panne, "obtenu : {echec:?}");
    assert!(
        echec.tenant.is_none(),
        "personne ne tient le poste : la fenetre n'a personne a nommer"
    );
    assert!(
        !echec.raison.is_empty(),
        "un echec sans raison ne dit rien a l'utilisateur"
    );

    let _ = std::fs::remove_file(&verrou);
}

/// Les quatre etats, tels que la fenetre les recoit.
///
/// La forme compte autant que le fond : c'est le contrat que lit la page, et un
/// nom de champ qui bouge la laisserait muette sans que rien ne casse ici.
#[test]
fn les_etats_du_lecteur_se_lisent_dans_la_fenetre() {
    let en_demarrage = serde_json::to_value(EtatLecteur::EnDemarrage).expect("etat serialisable");
    assert_eq!(en_demarrage, serde_json::json!({ "etat": "en_demarrage" }));

    let en_marche = serde_json::to_value(EtatLecteur::EnMarche).expect("etat serialisable");
    assert_eq!(en_marche, serde_json::json!({ "etat": "en_marche" }));

    let arrete = serde_json::to_value(EtatLecteur::Arrete).expect("etat serialisable");
    assert_eq!(arrete, serde_json::json!({ "etat": "arrete" }));

    let en_echec = serde_json::to_value(EtatLecteur::EnEchec(EchecLecteur {
        cas: CasEchec::PosteTenu,
        raison: "un lecteur tourne deja sur cette machine : « vibemap »".to_string(),
        tenant: Some(TenantDuPoste {
            vehicule: "vibemap".to_string(),
            pid: 4242,
            depuis: "2026-08-20T05:00:00+00:00".to_string(),
        }),
    }))
    .expect("etat serialisable");
    assert_eq!(
        en_echec,
        serde_json::json!({
            "etat": "en_echec",
            "cas": "poste_tenu",
            "raison": "un lecteur tourne deja sur cette machine : « vibemap »",
            "tenant": {
                "vehicule": "vibemap",
                "pid": 4242,
                "depuis": "2026-08-20T05:00:00+00:00",
            },
        })
    );
}

/// Chaque cause connue rend un cas distinct et une raison qui dit quoi faire.
///
/// C'est ce qui separe une phrase utile d'une carte muette : la fenetre ecrit
/// la sienne a partir du `cas`, et n'a rien a deviner de la raison.
#[test]
fn chaque_cause_d_arret_se_nomme_et_dit_quoi_faire() {
    let jeton = EchecLecteur::from(&LecteurError::from(TrousseauError::Absent(
        "11111111-1111-1111-1111-111111111111".to_string(),
    )));
    assert_eq!(jeton.cas, CasEchec::JetonRefuse, "obtenu : {jeton:?}");
    assert!(
        jeton.raison.contains("autorisation"),
        "le refus du trousseau dit comment le lever, obtenu : {}",
        jeton.raison
    );
    assert!(
        jeton.tenant.is_none(),
        "personne ne tient le poste : la fenetre n'a personne a nommer"
    );

    let arret = EchecLecteur::arret_inattendu();
    assert_eq!(arret.cas, CasEchec::ArretInattendu);
    assert!(
        arret.raison.contains("n'envoie plus"),
        "un arret qui ne dit pas ce qu'on y perd ne dit rien, obtenu : {}",
        arret.raison
    );

    // Les deux autres causes - poste tenu, panne ordinaire - s'eprouvent sur un
    // vrai demarrage, plus haut dans ce fichier : c'est la seule facon de savoir
    // que le lecteur les rend bien telles quelles.
    assert_ne!(jeton.cas, arret.cas);
}

/// L'etat montre est celui du moment, et non celui du demarrage.
///
/// C'est le coeur de FR-010 : un lecteur qui cesse de tourner doit se voir a la
/// lecture suivante, sans quoi la fenetre annoncerait une machine qui bat alors
/// qu'elle s'est tue.
#[test]
fn l_etat_montre_suit_ce_que_le_lecteur_fait_a_l_instant() {
    let en_marche = bureau::lecteur::etat_a_montrer(Some(true), &EtatLecteur::EnDemarrage);
    assert_eq!(en_marche, EtatLecteur::EnMarche);

    let arrete_tout_seul = bureau::lecteur::etat_a_montrer(Some(false), &EtatLecteur::EnMarche);
    let EtatLecteur::EnEchec(echec) = arrete_tout_seul else {
        panic!("une boucle qui ne tourne plus est un echec, pas un lecteur en marche");
    };
    assert_eq!(echec.cas, CasEchec::ArretInattendu);
    assert!(
        !echec.raison.is_empty(),
        "un arret sans raison ne dit rien a l'utilisateur"
    );

    // Sans lecteur a regarder, c'est le dernier demarrage qui parle : lui seul
    // sait s'il attend une autorisation ou s'il a deja renonce.
    assert_eq!(
        bureau::lecteur::etat_a_montrer(None, &EtatLecteur::EnDemarrage),
        EtatLecteur::EnDemarrage
    );
    let refus = EtatLecteur::EnEchec(EchecLecteur {
        cas: CasEchec::Panne,
        raison: "aucune configuration".to_string(),
        tenant: None,
    });
    assert_eq!(bureau::lecteur::etat_a_montrer(None, &refus), refus);
}

/// La relance : elle repose le verrou et remet un lecteur en marche, sans que
/// l'application ait eu a se fermer (FR-010).
///
/// Et elle ne fait rien sur un lecteur qui tourne : couper le battement pour le
/// reprendre aussitot ferait perdre le poste a la machine le temps du
/// remplacement, pour rien.
#[test]
fn une_relance_apres_un_arret_rend_un_lecteur_en_marche() {
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");

    let config = config_temporaire("relance");
    let verrou = chemin_temporaire("relance.lock");

    let mut lecteur = None;
    assert_eq!(
        bureau::lecteur::relancer(&mut lecteur, &config, &verrou),
        EtatLecteur::EnMarche,
        "sans lecteur, la relance en demarre un"
    );
    Verrou::prendre(&verrou, "vibemap").expect_err("le lecteur relance tient le poste");

    // Un second clic sur un lecteur qui tourne ne le remplace pas. La preuve
    // ne peut pas etre l'adresse de la valeur - elle vit dans la meme case, et
    // serait la meme apres un remplacement -, elle est dans ce que ferait un
    // remplacement : redemarrer, c'est relire la configuration, et une
    // configuration disparue ferait echouer ce demarrage-la. Le lecteur reste
    // en marche : il n'a donc pas ete repris.
    let mis_de_cote = std::fs::read_to_string(&config).expect("configuration relisible");
    std::fs::remove_file(&config).expect("configuration mise de cote");
    assert_eq!(
        bureau::lecteur::relancer(&mut lecteur, &config, &verrou),
        EtatLecteur::EnMarche,
        "un lecteur qui tourne n'est pas relance"
    );
    std::fs::write(&config, mis_de_cote).expect("configuration remise en place");

    // L'arret, tel que l'application le subit : la valeur disparait, et le poste
    // redevient libre.
    lecteur.take();
    let repris = Verrou::prendre(&verrou, "vibemap").expect("le poste est rendu");
    drop(repris);

    assert_eq!(
        bureau::lecteur::relancer(&mut lecteur, &config, &verrou),
        EtatLecteur::EnMarche,
        "apres un arret, la relance remet le lecteur en marche"
    );
    Verrou::prendre(&verrou, "vibemap").expect_err("le lecteur relance a repris le poste");

    drop(lecteur);
    let _ = std::fs::remove_file(&config);
    let _ = std::fs::remove_file(&verrou);
}
