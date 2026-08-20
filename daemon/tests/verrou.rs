//! Comportement : un seul lecteur par machine, et il le dit quand il refuse.
//!
//! Le verrou s'eprouve seul, sur un fichier temporaire : jamais sur
//! l'emplacement reel du poste, qu'un test ne doit pas pouvoir condamner - ni
//! prendre au lecteur de l'utilisateur pendant qu'il tourne.

use std::path::PathBuf;
use vibemap::Verrou;

fn chemin_temporaire() -> PathBuf {
    std::env::temp_dir().join(format!("vibemap-verrou-{}.lock", uuid::Uuid::new_v4()))
}

#[test]
fn un_verrou_relache_peut_etre_repris() {
    let chemin = chemin_temporaire();

    let premier = Verrou::prendre(&chemin, "vibemap").expect("le poste est libre");
    drop(premier);

    Verrou::prendre(&chemin, "vibemap")
        .expect("un verrou relache doit pouvoir etre repris");

    let _ = std::fs::remove_file(&chemin);
}

/// Le second lecteur ne demarre pas, et le message nomme ce qui tient le
/// poste : sans cela, l'utilisateur n'a aucun moyen de savoir quoi arreter.
#[test]
fn un_verrou_tenu_est_refuse_en_nommant_ce_qui_le_tient() {
    let chemin = chemin_temporaire();

    let _premier = Verrou::prendre(&chemin, "vibemap").expect("le poste est libre");

    let erreur = Verrou::prendre(&chemin, "l'application de bureau")
        .expect_err("un second lecteur ne doit pas demarrer");

    let message = erreur.to_string();
    assert!(
        message.contains("vibemap"),
        "le message doit nommer le vehicule qui tient le poste, obtenu : {message}"
    );
    assert!(
        message.contains(&std::process::id().to_string()),
        "le message doit donner le processus qui tient le poste, obtenu : {message}"
    );
    assert!(
        message.contains(chemin.to_str().unwrap()),
        "le message doit dire ou est le verrou, obtenu : {message}"
    );

    let _ = std::fs::remove_file(&chemin);
}

/// Une marque laissee par un processus disparu ne condamne pas le poste : un
/// lecteur tue brutalement ne doit pas empecher le lancement suivant.
#[test]
fn une_marque_orpheline_ne_bloque_pas_le_demarrage() {
    let chemin = chemin_temporaire();

    // Un vrai processus, mene jusqu'a sa fin : son identifiant ne designe plus
    // rien, exactement comme celui d'un lecteur tue.
    let mut disparu = std::process::Command::new("true")
        .spawn()
        .expect("lancement du processus temoin");
    let pid = disparu.id();
    disparu.wait().expect("attente du processus temoin");

    std::fs::write(
        &chemin,
        format!(
            "pid = {pid}\n\
             vehicule = \"vibemap\"\n\
             depuis = \"2020-01-01T00:00:00Z\"\n"
        ),
    )
    .expect("ecriture de la marque orpheline");

    let verrou = Verrou::prendre(&chemin, "vibemap")
        .expect("une marque orpheline ne doit pas condamner le poste");
    assert_eq!(verrou.chemin(), chemin);

    let _ = std::fs::remove_file(&chemin);
}

/// Le verrou vit a cote de la configuration, a un seul endroit par poste : deux
/// configurations sur la meme machine ne doivent pas ouvrir deux postes.
#[test]
fn le_verrou_a_un_seul_emplacement_par_poste() {
    let chemin = Verrou::chemin_par_defaut();

    assert!(chemin.ends_with("vibemap/lecteur.lock"), "obtenu : {}", chemin.display());
    assert!(chemin.is_absolute(), "obtenu : {}", chemin.display());
}
