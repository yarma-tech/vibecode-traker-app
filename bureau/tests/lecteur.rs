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

use bureau::lecteur::{EchecLecteur, EtatLecteur, TenantDuPoste};
use std::path::PathBuf;
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

    assert!(
        echec.poste_tenu,
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

    assert!(!echec.poste_tenu, "obtenu : {echec:?}");
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

/// Les trois etats, tels que la fenetre les recoit.
///
/// La forme compte autant que le fond : c'est le contrat que lit la page, et un
/// nom de champ qui bouge la laisserait muette sans que rien ne casse ici.
#[test]
fn les_trois_etats_du_lecteur_se_lisent_dans_la_fenetre() {
    let en_marche = serde_json::to_value(EtatLecteur::EnMarche).expect("etat serialisable");
    assert_eq!(en_marche, serde_json::json!({ "etat": "en_marche" }));

    let arrete = serde_json::to_value(EtatLecteur::Arrete).expect("etat serialisable");
    assert_eq!(arrete, serde_json::json!({ "etat": "arrete" }));

    let en_echec = serde_json::to_value(EtatLecteur::EnEchec(EchecLecteur {
        raison: "un lecteur tourne deja sur cette machine : « vibemap »".to_string(),
        poste_tenu: true,
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
            "raison": "un lecteur tourne deja sur cette machine : « vibemap »",
            "poste_tenu": true,
            "tenant": {
                "vehicule": "vibemap",
                "pid": 4242,
                "depuis": "2026-08-20T05:00:00+00:00",
            },
        })
    );
}
