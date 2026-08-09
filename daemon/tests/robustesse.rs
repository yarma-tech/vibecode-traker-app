//! Le suivi survit aux coupures et aux redemarrages.
//!
//! Rien ici ne parle au reseau : la position de lecture se persiste sur disque,
//! et c'est une propriete du systeme de fichiers, verifiable a sec. Le defaut que
//! ces tests ferment : jusqu'ici les decalages vivaient en memoire, donc un
//! redemarrage recomptait la fenetre de rattrapage — et les jetons, additifs,
//! doublaient.

use chrono::{DateTime, TimeZone, Utc};
use std::path::{Path, PathBuf};
use vibemap::journal::Suivi;

fn evenement(id: &str, fichier: &str, instant: &str) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant,
        "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier",
        "gitBranch": "main",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": id, "name": "Edit",
              "input": { "file_path": format!("/Users/moi/Developer/atelier/{fichier}") } }
        ] }
    })
    .to_string()
}

fn ligne_usage(model: &str, input: i64, instant: &str) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant,
        "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier",
        "gitBranch": "main",
        "uuid": format!("u-{instant}"),
        "message": { "role": "assistant", "model": model,
            "usage": { "input_tokens": input, "output_tokens": 1 } }
    })
    .to_string()
}

fn poser_journal(projets: &Path, nom: &str, contenu: &str) -> PathBuf {
    let projet = projets.join("-Users-moi-Developer-atelier");
    std::fs::create_dir_all(&projet).expect("creation du dossier de projet");
    let chemin = projet.join(nom);
    std::fs::write(&chemin, contenu).expect("ecriture du journal");
    chemin
}

fn dossier_neuf(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir().join(format!(
        "vibemap-robustesse-{nom}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).expect("creation du dossier de test");
    dossier
}

fn tres_ancien() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()
}

// -------------------------------------------------- persistance des offsets

#[test]
fn apres_un_redemarrage_rien_n_est_rejoue() {
    let projets = dossier_neuf("rejoue-projets");
    let offsets = dossier_neuf("rejoue-etat").join("offsets.json");
    poser_journal(
        &projets,
        "session.jsonl",
        &format!(
            "{}\n{}\n",
            ligne_usage("claude-opus-4-8", 100, "2026-08-04T12:00:00.000Z"),
            evenement("toolu_a", "a.ts", "2026-08-04T12:00:01.000Z"),
        ),
    );

    // Premiere vie : on lit, puis on enregistre la position sur disque.
    let mut premiere = Suivi::charger(&offsets);
    let lecture = premiere.nouveaux(&projets, tres_ancien());
    assert_eq!(lecture.evenements.len(), 1, "la premiere vie voit l'evenement");
    assert_eq!(lecture.usages.len(), 1, "la premiere vie voit la consommation");
    premiere.enregistrer().expect("la position doit s'ecrire");

    // Seconde vie : un Suivi tout neuf, recree depuis le disque. C'est un
    // redemarrage du daemon.
    let mut seconde = Suivi::charger(&offsets);
    let apres = seconde.nouveaux(&projets, tres_ancien());
    assert!(apres.evenements.is_empty(), "aucun evenement rejoue apres redemarrage");
    assert!(
        apres.usages.is_empty(),
        "aucun jeton recompte apres redemarrage : c'est le defaut que #10 ferme"
    );
}

#[test]
fn seules_les_lignes_ajoutees_depuis_le_redemarrage_reviennent() {
    let projets = dossier_neuf("ajout-projets");
    let offsets = dossier_neuf("ajout-etat").join("offsets.json");
    let chemin = poser_journal(
        &projets,
        "session.jsonl",
        &format!("{}\n", evenement("toolu_a", "a.ts", "2026-08-04T12:00:00.000Z")),
    );

    let mut premiere = Suivi::charger(&offsets);
    premiere.nouveaux(&projets, tres_ancien());
    premiere.enregistrer().expect("ecriture de la position");

    // Le journal grandit pendant que le daemon etait arrete.
    let suite = format!("{}\n", evenement("toolu_b", "b.ts", "2026-08-04T12:05:00.000Z"));
    let mut fichier = std::fs::OpenOptions::new().append(true).open(&chemin).unwrap();
    std::io::Write::write_all(&mut fichier, suite.as_bytes()).unwrap();

    let mut seconde = Suivi::charger(&offsets);
    let apres = seconde.nouveaux(&projets, tres_ancien());
    assert_eq!(apres.evenements.len(), 1, "seule la ligne ajoutee revient");
    assert_eq!(apres.evenements[0].tool_use_id, "toolu_b");
}

// ------------------------------------------------ corruption du fichier d'etat

#[test]
fn un_fichier_d_offsets_illisible_ne_fait_pas_paniquer() {
    let projets = dossier_neuf("corrompu-projets");
    let offsets = dossier_neuf("corrompu-etat").join("offsets.json");
    std::fs::create_dir_all(offsets.parent().unwrap()).unwrap();
    // Une ecriture interrompue laisserait ce genre de moitie de fichier.
    std::fs::write(&offsets, b"\x00\x01{ ceci n'est pas fini").unwrap();
    poser_journal(
        &projets,
        "session.jsonl",
        &format!("{}\n", evenement("toolu_a", "a.ts", "2026-08-04T12:00:00.000Z")),
    );

    // On ne plante pas : faute de position sure, on relit dans la fenetre, ce qui
    // est borne et rattrape par l'idempotence cote base.
    let mut suivi = Suivi::charger(&offsets);
    let lecture = suivi.nouveaux(&projets, tres_ancien());
    assert_eq!(lecture.evenements.len(), 1, "offsets illisibles : on relit sans planter");
}

#[test]
fn l_ecriture_des_offsets_est_atomique() {
    let projets = dossier_neuf("atomique-projets");
    let offsets = dossier_neuf("atomique-etat").join("offsets.json");
    poser_journal(
        &projets,
        "session.jsonl",
        &format!("{}\n", evenement("toolu_a", "a.ts", "2026-08-04T12:00:00.000Z")),
    );

    let mut suivi = Suivi::charger(&offsets);
    suivi.nouveaux(&projets, tres_ancien());
    suivi.enregistrer().expect("ecriture de la position");

    assert!(offsets.exists(), "le fichier d'offsets existe apres ecriture");
    let temporaire = offsets.with_extension("json.tmp");
    assert!(
        !temporaire.exists(),
        "l'ecriture atomique (temp puis rename) ne laisse aucun fichier temporaire"
    );

    // Rechargeable a l'identique : la seconde vie ne rejoue rien.
    let mut relu = Suivi::charger(&offsets);
    assert!(
        relu.nouveaux(&projets, tres_ancien()).evenements.is_empty(),
        "la position relue reprend exactement ou la premiere s'est arretee"
    );
}

// --------------------------------------- journaux tronques ou mal formes

#[test]
fn un_journal_de_pur_binaire_n_arrete_pas_le_suivi() {
    let projets = dossier_neuf("binaire-projets");
    let offsets = dossier_neuf("binaire-etat").join("offsets.json");
    let projet = projets.join("-Users-moi-Developer-atelier");
    std::fs::create_dir_all(&projet).unwrap();
    std::fs::write(projet.join("brouille.jsonl"), [0u8, 159, 146, 150, 255, 254]).unwrap();

    let mut suivi = Suivi::charger(&offsets);
    let lecture = suivi.nouveaux(&projets, tres_ancien());
    assert!(lecture.evenements.is_empty(), "un binaire ne donne rien, mais ne plante pas");
    assert!(lecture.usages.is_empty());
}

#[test]
fn une_derniere_ligne_tronquee_est_gardee_pour_plus_tard() {
    let projets = dossier_neuf("tronque-projets");
    let offsets = dossier_neuf("tronque-etat").join("offsets.json");
    // Une ligne complete, puis une ligne en cours d'ecriture, sans retour a la
    // ligne final : le journal est en train d'etre ecrit.
    let complete = evenement("toolu_a", "a.ts", "2026-08-04T12:00:00.000Z");
    let tronquee = "{\"type\":\"assistant\",\"timestamp\":\"2026-08-04T12:0";
    poser_journal(&projets, "session.jsonl", &format!("{complete}\n{tronquee}"));

    let mut suivi = Suivi::charger(&offsets);
    let lecture = suivi.nouveaux(&projets, tres_ancien());
    assert_eq!(lecture.evenements.len(), 1, "seule la ligne complete sort");
    assert_eq!(lecture.evenements[0].tool_use_id, "toolu_a");
}
