//! Comportement : le lecteur embarque en bibliotheque tient le poste, et le
//! rend quand il s'arrete.
//!
//! C'est la promesse sur laquelle repose l'application de bureau : elle demarre
//! le meme lecteur que le binaire, par le meme module, et rien ne doit survivre
//! a son arret - pas meme le verrou. Le verrou lui-meme est eprouve dans
//! `verrou.rs` ; ce qui se verifie ici, c'est que le lecteur le prend et le
//! relache aux bons moments.
//!
//! Tout se joue sur des fichiers temporaires : jamais sur l'emplacement reel du
//! poste, qu'un test ne doit pas pouvoir condamner - ni prendre au lecteur de
//! l'utilisateur pendant qu'il tourne.

use std::path::PathBuf;
use vibemap::lecteur::Lecteur;
use vibemap::Verrou;

/// Le jeton vient de l'environnement : le trousseau du systeme n'a rien a faire
/// dans un test, et il n'existe pas sur la machine d'integration continue.
fn sans_trousseau() {
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");
}

fn chemin_temporaire(quoi: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vibemap-lecteur-{quoi}-{}", uuid::Uuid::new_v4()))
}

/// Une configuration minimale, juste assez pour que le lecteur se prepare.
fn config_temporaire() -> PathBuf {
    let chemin = chemin_temporaire("config.toml");
    std::fs::write(
        &chemin,
        "supabase_url = \"http://127.0.0.1:54321\"\n\
         machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
         label = \"machine de test\"\n",
    )
    .expect("ecriture de la configuration de test");
    chemin
}

#[test]
fn le_lecteur_prend_le_poste_et_le_rend_a_son_arret() {
    sans_trousseau();
    let config = config_temporaire();
    let verrou = chemin_temporaire("lecteur.lock");

    let lecteur = Lecteur::preparer(&config, &verrou, "l'application de bureau")
        .expect("le poste est libre, le lecteur doit se preparer");
    assert_eq!(lecteur.label(), "machine de test");

    Verrou::prendre(&verrou, "vibemap")
        .expect_err("le lecteur prepare tient le poste : personne d'autre ne doit le prendre");

    // L'arret, c'est la disparition du lecteur : rien d'autre n'est a faire, et
    // c'est exactement ce que l'application rejoue en se fermant.
    drop(lecteur);

    Verrou::prendre(&verrou, "vibemap")
        .expect("le poste doit etre libre des que le lecteur a disparu");

    let _ = std::fs::remove_file(&config);
    let _ = std::fs::remove_file(&verrou);
}

/// Le second lecteur ne demarre pas, et l'echec se distingue d'une panne : il
/// n'y a rien a reparer, il y a un lecteur a arreter.
#[test]
fn un_poste_deja_tenu_empeche_le_lecteur_de_se_preparer() {
    sans_trousseau();
    let config = config_temporaire();
    let verrou = chemin_temporaire("lecteur.lock");

    let _tenant = Verrou::prendre(&verrou, "vibemap").expect("le poste est libre");

    let erreur = Lecteur::preparer(&config, &verrou, "l'application de bureau")
        .expect_err("un second lecteur ne doit pas se preparer");

    assert!(
        erreur.poste_tenu(),
        "le cas doit se reconnaitre pour lui-meme, obtenu : {erreur}"
    );
    assert!(
        erreur.to_string().contains("vibemap"),
        "l'echec doit nommer ce qui tient le poste, obtenu : {erreur}"
    );

    let _ = std::fs::remove_file(&config);
    let _ = std::fs::remove_file(&verrou);
}

/// Une configuration absente se dit avant tout le reste : sans elle, le lecteur
/// n'a ni machine ni adresse, et prendre le poste ne servirait a rien.
#[test]
fn une_configuration_absente_se_dit_sans_prendre_le_poste() {
    sans_trousseau();
    let config = chemin_temporaire("absente.toml");
    let verrou = chemin_temporaire("lecteur.lock");

    let erreur = Lecteur::preparer(&config, &verrou, "l'application de bureau")
        .expect_err("sans configuration, le lecteur ne demarre pas");

    assert!(
        !erreur.poste_tenu(),
        "une configuration absente n'est pas un poste tenu, obtenu : {erreur}"
    );
    Verrou::prendre(&verrou, "vibemap").expect("le poste ne doit pas avoir ete pris pour rien");

    let _ = std::fs::remove_file(&verrou);
}
