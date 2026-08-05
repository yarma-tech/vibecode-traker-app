//! Comportement 4 : la configuration se charge, et quand elle cloche elle dit quoi corriger.
//!
//! Un daemon n'a pas d'interface. Si sa configuration est mauvaise, le seul
//! endroit ou l'utilisateur peut comprendre pourquoi, c'est ce message d'erreur.

use std::path::PathBuf;

fn fichier_temporaire(contenu: &str) -> PathBuf {
    let chemin =
        std::env::temp_dir().join(format!("vibemap-{}.toml", uuid::Uuid::new_v4()));
    std::fs::write(&chemin, contenu).expect("ecriture du fichier de test");
    chemin
}

#[test]
fn charge_une_configuration_complete() {
    let chemin = fichier_temporaire(
        r#"
        supabase_url = "http://127.0.0.1:54321"
        machine_id = "3f2b1c00-0000-0000-0000-000000000000"
        label = "MacBook Pro"
        "#,
    );

    let config = vibemap::Config::load(&chemin).expect("une configuration complete doit charger");

    assert_eq!(config.supabase_url, "http://127.0.0.1:54321");
    assert_eq!(config.label, "MacBook Pro");
    assert_eq!(
        config.interval_seconds, 30,
        "sans valeur explicite, le battement est de 30 secondes"
    );
}

/// Le jeton ne vit plus dans le fichier : il est au trousseau (issue #9).
/// Un fichier qui en contient encore un vient d'avant l'appairage, et le dire
/// vaut mieux que de faire semblant de rien.
#[test]
fn refuse_un_jeton_ecrit_en_clair() {
    let chemin = fichier_temporaire(
        r#"
        supabase_url = "http://127.0.0.1:54321"
        machine_id = "3f2b1c00-0000-0000-0000-000000000000"
        label = "MacBook Pro"
        token = "un-jeton-en-clair"
        "#,
    );

    let erreur = vibemap::Config::load(&chemin)
        .expect_err("un jeton en clair dans le fichier doit etre signale");

    let message = erreur.to_string();
    assert!(
        message.contains("trousseau"),
        "le message doit renvoyer vers le trousseau, obtenu : {message}"
    );
    assert!(
        message.contains("vibemap pair"),
        "le message doit dire comment refaire l'appairage, obtenu : {message}"
    );
}

#[test]
fn le_battement_reste_reglable() {
    let chemin = fichier_temporaire(
        r#"
        supabase_url = "http://127.0.0.1:54321"
        machine_id = "3f2b1c00-0000-0000-0000-000000000000"
        label = "MacBook Pro"
        interval_seconds = 5
        "#,
    );

    let config = vibemap::Config::load(&chemin).expect("chargement");
    assert_eq!(config.interval_seconds, 5);
}

/// Cinq secondes est le budget du critere d'acceptation : « la parcelle passe
/// en ambre sur un autre appareil en moins de cinq secondes ». La lecture des
/// journaux doit donc tourner nettement plus vite que le battement.
#[test]
fn la_lecture_des_journaux_a_sa_propre_cadence() {
    let chemin = fichier_temporaire(
        r#"
        supabase_url = "http://127.0.0.1:54321"
        machine_id = "3f2b1c00-0000-0000-0000-000000000000"
        label = "MacBook Pro"
        "#,
    );

    let config = vibemap::Config::load(&chemin).expect("chargement");

    assert_eq!(config.journal_seconds, 2);
    assert!(
        config.journal_seconds < config.interval_seconds,
        "les journaux se lisent plus souvent que le battement"
    );
    assert_eq!(
        config.journal_lookback_seconds, 600,
        "au premier tour, on ne remonte pas plus loin que la fenetre d'activite"
    );
    assert!(config.journaux().ends_with(".claude/projects"));
}

#[test]
fn la_cadence_des_journaux_reste_reglable() {
    let chemin = fichier_temporaire(
        r#"
        supabase_url = "http://127.0.0.1:54321"
        machine_id = "3f2b1c00-0000-0000-0000-000000000000"
        label = "MacBook Pro"
        journal_seconds = 10
        journal_lookback_seconds = 60
        claude_projects = "~/ailleurs/projects"
        "#,
    );

    let config = vibemap::Config::load(&chemin).expect("chargement");

    assert_eq!(config.journal_seconds, 10);
    assert_eq!(config.journal_lookback_seconds, 60);
    assert!(
        config.journaux().ends_with("ailleurs/projects"),
        "le « ~ » doit etre remplace par le dossier personnel"
    );
    assert!(!config.journaux().to_string_lossy().contains('~'));
}

#[test]
fn nomme_le_champ_manquant() {
    let chemin = fichier_temporaire(
        r#"
        supabase_url = "http://127.0.0.1:54321"
        machine_id = "3f2b1c00-0000-0000-0000-000000000000"
        "#,
    );

    let erreur = vibemap::Config::load(&chemin)
        .expect_err("une configuration incomplete ne doit pas charger");

    let message = erreur.to_string();
    assert!(
        message.contains("label"),
        "le message doit nommer le champ manquant, obtenu : {message}"
    );
}

#[test]
fn dit_ou_il_a_cherche_quand_le_fichier_manque() {
    let chemin = std::env::temp_dir().join("vibemap-ce-fichier-n-existe-pas.toml");
    let _ = std::fs::remove_file(&chemin);

    let erreur =
        vibemap::Config::load(&chemin).expect_err("un fichier absent ne doit pas charger");

    let message = erreur.to_string();
    assert!(
        message.contains(chemin.to_str().unwrap()),
        "le message doit donner le chemin cherche, obtenu : {message}"
    );
    assert!(
        message.contains("vibemap pair"),
        "le message doit dire comment creer la configuration, obtenu : {message}"
    );
}
