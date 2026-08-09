//! Ce que le daemon tire des journaux de Claude Code.
//!
//! Rien ici ne parle au reseau : la lecture des journaux est une fonction pure
//! sur du texte, et c'est exactement ce qui la rend verifiable.

use chrono::{TimeZone, Utc};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vibemap::journal::{
    depuis_hook, lire, lire_usage, localiser, racine_git, rattacher, rattacher_usage, Nature,
    Suivi,
};

/// Une ligne d'assistant telle qu'elle apparait dans un vrai journal.
fn ligne(tool: &str, input: serde_json::Value, id: &str, instant: &str) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant,
        "sessionId": "9adbb340-9563-4b1d-878e-4e2d5032ac67",
        "cwd": "/Users/moi/Developer/atelier",
        "gitBranch": "main",
        "message": {
            "role": "assistant",
            "content": [{ "type": "tool_use", "id": id, "name": tool, "input": input }],
            "usage": { "input_tokens": 2, "output_tokens": 18 }
        }
    })
    .to_string()
}

/// Une ligne d'assistant qui ne porte que sa consommation, sans outil.
fn ligne_usage(model: &str, usage: serde_json::Value, instant: &str) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant,
        "sessionId": "9adbb340-9563-4b1d-878e-4e2d5032ac67",
        "cwd": "/Users/moi/Developer/atelier",
        "gitBranch": "main",
        "message": {
            "role": "assistant",
            "model": model,
            "content": [{ "type": "text", "text": "bonjour" }],
            "usage": usage
        }
    })
    .to_string()
}

/// Une ligne de consommation pour une session et un dossier donnes.
fn ligne_usage_pour(
    session: &str,
    cwd: &str,
    model: &str,
    usage: serde_json::Value,
    instant: &str,
) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant,
        "sessionId": session,
        "cwd": cwd,
        "gitBranch": "main",
        "message": { "role": "assistant", "model": model, "usage": usage }
    })
    .to_string()
}

#[test]
fn les_usages_s_agregent_par_session_dans_leur_repo() {
    let racine = "/Users/moi/Developer/atelier";
    let a = ligne_usage_pour(
        "s1",
        racine,
        "claude-opus-4-8",
        serde_json::json!({
            "input_tokens": 10, "output_tokens": 100,
            "cache_read_input_tokens": 1000, "cache_creation_input_tokens": 50
        }),
        "2026-08-04T12:00:00.000Z",
    );
    let b = ligne_usage_pour(
        "s1",
        racine,
        "claude-sonnet-4-5",
        serde_json::json!({
            "input_tokens": 5, "output_tokens": 20,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0
        }),
        "2026-08-04T12:05:00.000Z",
    );
    let usages = lire_usage(&format!("{a}\n{b}"));

    let lots = rattacher_usage(&usages, &repos_connus(&[(racine, "repo-1")]));

    assert_eq!(lots.len(), 1);
    assert_eq!(lots[0].repo_id, "repo-1");
    assert_eq!(lots[0].branche.as_deref(), Some("main"));
    assert_eq!(lots[0].sessions.len(), 1);
    let s = &lots[0].sessions[0];
    assert_eq!(s.session_id, "s1");
    assert_eq!(s.input, 15);
    assert_eq!(s.output, 120);
    assert_eq!(s.cache_read, 1000);
    assert_eq!(s.cache_creation, 50);
    // Le dernier modele vu l'emporte : un agent peut changer en cours de route.
    assert_eq!(s.model, "claude-sonnet-4-5");
    assert_eq!(s.debut, Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap());
    assert_eq!(s.fin, Utc.with_ymd_and_hms(2026, 8, 4, 12, 5, 0).unwrap());
}

#[test]
fn un_lot_de_consommation_porte_une_cle_stable_et_sensible_au_contenu() {
    let racine = "/Users/moi/Developer/atelier";
    let a = ligne_usage_pour(
        "s1",
        racine,
        "claude-opus-4-8",
        serde_json::json!({ "input_tokens": 10, "output_tokens": 1 }),
        "2026-08-04T12:00:00.000Z",
    );
    let b = ligne_usage_pour(
        "s1",
        racine,
        "claude-opus-4-8",
        serde_json::json!({ "input_tokens": 5, "output_tokens": 2 }),
        "2026-08-04T12:05:00.000Z",
    );

    let cle = |contenu: &str| {
        rattacher_usage(&lire_usage(contenu), &repos_connus(&[(racine, "repo-1")]))[0].sessions[0]
            .cle_usage
            .clone()
    };

    let deux_lignes = cle(&format!("{a}\n{b}"));
    assert!(deux_lignes.is_some(), "un lot lu porte une cle");
    // Meme contenu, meme cle : c'est ce qui laisse la base reconnaitre un rejeu.
    assert_eq!(deux_lignes, cle(&format!("{a}\n{b}")), "meme contenu, meme cle");
    // Insensible a l'ordre d'arrivee : les identifiants sont tries avant hachage.
    assert_eq!(deux_lignes, cle(&format!("{b}\n{a}")), "l'ordre n'a pas d'importance");

    // Une ligne de plus : le lot est different, la cle change, les jetons
    // nouveaux seront bien comptes.
    let c = ligne_usage_pour(
        "s1",
        racine,
        "claude-opus-4-8",
        serde_json::json!({ "input_tokens": 7, "output_tokens": 3 }),
        "2026-08-04T12:10:00.000Z",
    );
    assert_ne!(deux_lignes, cle(&format!("{a}\n{b}\n{c}")), "un contenu different, une cle differente");
}

#[test]
fn un_usage_hors_de_tout_repo_connu_est_ignore() {
    let usages = lire_usage(&ligne_usage_pour(
        "s1",
        "/ailleurs/projet",
        "claude-opus-4-8",
        serde_json::json!({ "input_tokens": 10, "output_tokens": 100 }),
        "2026-08-04T12:00:00.000Z",
    ));

    let lots = rattacher_usage(
        &usages,
        &repos_connus(&[("/Users/moi/Developer/atelier", "repo-1")]),
    );

    assert!(lots.is_empty(), "un usage hors repo cartographie ne sort pas");
}

#[test]
fn une_ligne_assistant_donne_sa_consommation() {
    let contenu = ligne_usage(
        "claude-opus-4-8-20260101",
        serde_json::json!({
            "input_tokens": 12,
            "output_tokens": 340,
            "cache_read_input_tokens": 8000,
            "cache_creation_input_tokens": 1500
        }),
        "2026-08-04T12:00:00.000Z",
    );

    let usages = lire_usage(&contenu);

    assert_eq!(usages.len(), 1);
    let u = &usages[0];
    assert_eq!(u.session_id, "9adbb340-9563-4b1d-878e-4e2d5032ac67");
    assert_eq!(u.model, "claude-opus-4-8-20260101");
    assert_eq!(u.input, 12);
    assert_eq!(u.output, 340);
    assert_eq!(u.cache_read, 8000);
    assert_eq!(u.cache_creation, 1500);
    assert_eq!(u.cwd, "/Users/moi/Developer/atelier");
    assert_eq!(u.branche.as_deref(), Some("main"));
    assert_eq!(
        u.instant,
        Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
    );
}

#[test]
fn un_appel_read_donne_une_lecture() {
    let contenu = ligne(
        "Read",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/src/a.ts" }),
        "toolu_01",
        "2026-08-04T12:00:00.000Z",
    );

    let evenements = lire(&contenu);

    assert_eq!(evenements.len(), 1);
    let e = &evenements[0];
    assert_eq!(e.nature, Nature::Lecture);
    assert_eq!(e.tool_use_id, "toolu_01");
    assert_eq!(e.session_id, "9adbb340-9563-4b1d-878e-4e2d5032ac67");
    assert_eq!(e.chemin, "/Users/moi/Developer/atelier/src/a.ts");
    assert!(!e.dossier);
    assert_eq!(e.branche.as_deref(), Some("main"));
    assert_eq!(
        e.instant,
        Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap()
    );
}

#[test]
fn un_appel_edit_donne_une_ecriture() {
    let contenu = ligne(
        "Edit",
        serde_json::json!({
            "file_path": "/Users/moi/Developer/atelier/src/a.ts",
            "old_string": "secret", "new_string": "toujours secret"
        }),
        "toolu_02",
        "2026-08-04T12:00:01.000Z",
    );

    let evenements = lire(&contenu);

    assert_eq!(evenements.len(), 1);
    assert_eq!(evenements[0].nature, Nature::Ecriture);
    // Le contenu edite ne doit exister nulle part dans ce qu'on retient.
    let trace = format!("{:?}", evenements[0]);
    assert!(!trace.contains("secret"), "le contenu edite a fuite : {trace}");
}

#[test]
fn bash_ne_donne_aucun_evenement() {
    let contenu = ligne(
        "Bash",
        serde_json::json!({ "command": "rm -rf ./tmp" }),
        "toolu_03",
        "2026-08-04T12:00:02.000Z",
    );

    assert!(lire(&contenu).is_empty());
}

#[test]
fn un_outil_hors_correspondance_ne_donne_rien() {
    let contenu = ligne(
        "WebFetch",
        serde_json::json!({ "url": "https://example.com" }),
        "toolu_04",
        "2026-08-04T12:00:03.000Z",
    );

    assert!(lire(&contenu).is_empty());
}

#[test]
fn notebookedit_vise_le_carnet() {
    let contenu = ligne(
        "NotebookEdit",
        serde_json::json!({
            "notebook_path": "/Users/moi/Developer/atelier/etudes/mesure.ipynb",
            "new_source": "print(1)"
        }),
        "toolu_05",
        "2026-08-04T12:00:04.000Z",
    );

    let evenements = lire(&contenu);

    assert_eq!(evenements[0].nature, Nature::Ecriture);
    assert_eq!(
        evenements[0].chemin,
        "/Users/moi/Developer/atelier/etudes/mesure.ipynb"
    );
}

#[test]
fn grep_sans_chemin_vise_le_dossier_courant() {
    let contenu = ligne(
        "Grep",
        serde_json::json!({ "pattern": "TODO" }),
        "toolu_06",
        "2026-08-04T12:00:05.000Z",
    );

    let evenements = lire(&contenu);

    assert_eq!(evenements[0].nature, Nature::Lecture);
    assert_eq!(evenements[0].chemin, "/Users/moi/Developer/atelier");
    assert!(evenements[0].dossier, "un grep vise un dossier, pas un fichier");
}

#[test]
fn un_chemin_relatif_se_lit_depuis_le_dossier_courant() {
    let contenu = ligne(
        "Write",
        serde_json::json!({ "file_path": "src/b.ts", "content": "x" }),
        "toolu_07",
        "2026-08-04T12:00:06.000Z",
    );

    assert_eq!(lire(&contenu)[0].chemin, "/Users/moi/Developer/atelier/src/b.ts");
}

#[test]
fn une_ligne_illisible_n_arrete_pas_la_lecture() {
    let bonne = ligne(
        "Read",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/src/a.ts" }),
        "toolu_08",
        "2026-08-04T12:00:07.000Z",
    );
    let contenu = format!("{{ceci n'est pas du json\n{bonne}\n");

    assert_eq!(lire(&contenu).len(), 1);
}

#[test]
fn plusieurs_outils_dans_un_meme_message_donnent_plusieurs_evenements() {
    let contenu = serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-08-04T12:00:08.000Z",
        "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier",
        "message": { "role": "assistant", "content": [
            { "type": "text", "text": "je regarde" },
            { "type": "tool_use", "id": "a", "name": "Read",
              "input": { "file_path": "/Users/moi/Developer/atelier/x.ts" } },
            { "type": "tool_use", "id": "b", "name": "Bash",
              "input": { "command": "ls" } },
            { "type": "tool_use", "id": "c", "name": "Write",
              "input": { "file_path": "/Users/moi/Developer/atelier/y.ts", "content": "" } }
        ] }
    })
    .to_string();

    let evenements = lire(&contenu);

    assert_eq!(evenements.len(), 2);
    assert_eq!(evenements[0].tool_use_id, "a");
    assert_eq!(evenements[1].tool_use_id, "c");
}

// ------------------------------------------------------------- localisation

#[test]
fn un_fichier_donne_son_dossier_pour_module() {
    let place = localiser(
        "/Users/moi/Developer/atelier/src/core/auth.ts",
        false,
        Path::new("/Users/moi/Developer/atelier"),
    );

    assert_eq!(
        place,
        Some(("src/core".to_string(), "src/core/auth.ts".to_string()))
    );
}

#[test]
fn un_fichier_a_la_racine_appartient_au_module_vide() {
    let place = localiser(
        "/Users/moi/Developer/atelier/README.md",
        false,
        Path::new("/Users/moi/Developer/atelier"),
    );

    assert_eq!(place, Some((String::new(), "README.md".to_string())));
}

#[test]
fn un_dossier_est_son_propre_module() {
    let place = localiser(
        "/Users/moi/Developer/atelier/src",
        true,
        Path::new("/Users/moi/Developer/atelier"),
    );

    assert_eq!(place, Some(("src".to_string(), "src".to_string())));
}

#[test]
fn un_chemin_hors_du_repo_n_appartient_a_aucun_module() {
    let place = localiser(
        "/etc/hosts",
        false,
        Path::new("/Users/moi/Developer/atelier"),
    );

    assert_eq!(place, None);
}

// --------------------------------------------------------------------- hook

/// La charge utile que Claude Code passe a un hook `PostToolUse`.
fn charge_hook(tool: &str, input: serde_json::Value) -> String {
    serde_json::json!({
        "session_id": "9adbb340-9563-4b1d-878e-4e2d5032ac67",
        "transcript_path": "/Users/moi/.claude/projects/x/9adbb340.jsonl",
        "cwd": "/Users/moi/Developer/atelier",
        "hook_event_name": "PostToolUse",
        "tool_name": tool,
        "tool_input": input,
        "tool_response": { "filePath": "/Users/moi/Developer/atelier/src/a.ts" },
        "tool_use_id": "toolu_hook"
    })
    .to_string()
}

#[test]
fn un_hook_donne_le_meme_evenement_que_le_journal() {
    let evenement = depuis_hook(&charge_hook(
        "Edit",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/src/a.ts" }),
    ))
    .expect("le hook doit produire un evenement");

    assert_eq!(evenement.nature, Nature::Ecriture);
    assert_eq!(evenement.session_id, "9adbb340-9563-4b1d-878e-4e2d5032ac67");
    // Le meme identifiant que la ligne du journal : c'est ce qui fait que la
    // contrainte d'unicite absorbe le doublon au lieu de creer deux lignes.
    assert_eq!(evenement.tool_use_id, "toolu_hook");
    assert_eq!(evenement.chemin, "/Users/moi/Developer/atelier/src/a.ts");
}

#[test]
fn un_hook_sur_bash_ne_donne_rien() {
    assert!(depuis_hook(&charge_hook("Bash", serde_json::json!({ "command": "ls" }))).is_none());
}

#[test]
fn une_charge_de_hook_illisible_ne_donne_rien() {
    assert!(depuis_hook("pas du json").is_none());
}

// -------------------------------------------------------------- racine git

#[test]
fn la_racine_git_se_trouve_en_remontant() {
    let dossier = dossier_neuf("racine");
    let profond = dossier.join("web").join("app");
    std::fs::create_dir_all(&profond).expect("creation des sous-dossiers");
    std::fs::create_dir_all(dossier.join(".git")).expect("creation du .git");

    assert_eq!(
        racine_git(&profond).as_deref(),
        Some(dossier.as_path()),
        "un sous-dossier appartient au depot qui le contient"
    );
}

#[test]
fn sans_depot_il_n_y_a_pas_de_racine() {
    let dossier = dossier_neuf("sans-depot");
    assert_eq!(racine_git(&dossier), None);
}

// -------------------------------------------------------------- rattachement

/// Les repos que le daemon a deja cartographies, par racine.
fn repos_connus(couples: &[(&str, &str)]) -> BTreeMap<PathBuf, String> {
    couples
        .iter()
        .map(|(racine, id)| (PathBuf::from(racine), id.to_string()))
        .collect()
}

#[test]
fn un_evenement_rejoint_le_repo_de_son_dossier_courant() {
    let evenements = lire(&ligne(
        "Edit",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/src/core/a.ts" }),
        "toolu_01",
        "2026-08-04T12:00:00.000Z",
    ));

    let lots = rattacher(
        &evenements,
        &repos_connus(&[("/Users/moi/Developer/atelier", "repo-1")]),
    );

    assert_eq!(lots.len(), 1);
    assert_eq!(lots[0].repo_id, "repo-1");
    assert_eq!(lots[0].branche.as_deref(), Some("main"));
    assert_eq!(lots[0].activites.len(), 1);
    assert_eq!(lots[0].activites[0].module_path, "src/core");
    assert_eq!(lots[0].activites[0].file_path, "src/core/a.ts");
    assert_eq!(lots[0].activites[0].kind, "write");
}

#[test]
fn un_dossier_courant_plus_profond_trouve_quand_meme_son_repo() {
    let contenu = serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-08-04T12:00:00.000Z",
        "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier/web",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "a", "name": "Read",
              "input": { "file_path": "/Users/moi/Developer/atelier/web/app/page.tsx" } }
        ] }
    })
    .to_string();

    let lots = rattacher(
        &lire(&contenu),
        &repos_connus(&[("/Users/moi/Developer/atelier", "repo-1")]),
    );

    assert_eq!(lots[0].activites[0].module_path, "web/app");
}

/// Un repo imbrique dans un autre gagne : c'est le plus proche du travail.
#[test]
fn le_repo_le_plus_profond_l_emporte() {
    let contenu = serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-08-04T12:00:00.000Z",
        "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier/web",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "a", "name": "Read",
              "input": { "file_path": "/Users/moi/Developer/atelier/web/app/page.tsx" } }
        ] }
    })
    .to_string();

    let lots = rattacher(
        &lire(&contenu),
        &repos_connus(&[
            ("/Users/moi/Developer/atelier", "repo-1"),
            ("/Users/moi/Developer/atelier/web", "repo-web"),
        ]),
    );

    assert_eq!(lots.len(), 1);
    assert_eq!(lots[0].repo_id, "repo-web");
    assert_eq!(lots[0].activites[0].module_path, "app");
}

#[test]
fn un_travail_hors_de_tout_repo_connu_est_ignore() {
    let contenu = serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-08-04T12:00:00.000Z",
        "sessionId": "s1",
        "cwd": "/Users/moi/Documents",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "a", "name": "Write",
              "input": { "file_path": "/Users/moi/Documents/notes.md" } }
        ] }
    })
    .to_string();

    let lots = rattacher(
        &lire(&contenu),
        &repos_connus(&[("/Users/moi/Developer/atelier", "repo-1")]),
    );

    assert!(lots.is_empty());
}

#[test]
fn un_fichier_hors_du_repo_courant_est_ignore() {
    let evenements = lire(&ligne(
        "Read",
        serde_json::json!({ "file_path": "/etc/hosts" }),
        "toolu_01",
        "2026-08-04T12:00:00.000Z",
    ));

    let lots = rattacher(
        &evenements,
        &repos_connus(&[("/Users/moi/Developer/atelier", "repo-1")]),
    );

    assert!(lots.is_empty(), "un secret du systeme n'entre pas dans la carte");
}

#[test]
fn un_voisin_de_meme_prefixe_ne_capte_pas_le_travail() {
    let contenu = serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-08-04T12:00:00.000Z",
        "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier-bis",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "a", "name": "Write",
              "input": { "file_path": "/Users/moi/Developer/atelier-bis/a.ts" } }
        ] }
    })
    .to_string();

    let lots = rattacher(
        &lire(&contenu),
        &repos_connus(&[("/Users/moi/Developer/atelier", "repo-1")]),
    );

    assert!(lots.is_empty());
}

#[test]
fn deux_repos_donnent_deux_lots() {
    let un = serde_json::json!({
        "type": "assistant", "timestamp": "2026-08-04T12:00:00.000Z", "sessionId": "s1",
        "cwd": "/Users/moi/Developer/atelier",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "a", "name": "Write",
              "input": { "file_path": "/Users/moi/Developer/atelier/a.ts" } }
        ] }
    })
    .to_string();
    let deux = serde_json::json!({
        "type": "assistant", "timestamp": "2026-08-04T12:00:01.000Z", "sessionId": "s2",
        "cwd": "/Users/moi/Developer/autre",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "b", "name": "Read",
              "input": { "file_path": "/Users/moi/Developer/autre/b.ts" } }
        ] }
    })
    .to_string();

    let lots = rattacher(
        &lire(&format!("{un}\n{deux}")),
        &repos_connus(&[
            ("/Users/moi/Developer/atelier", "repo-1"),
            ("/Users/moi/Developer/autre", "repo-2"),
        ]),
    );

    assert_eq!(lots.len(), 2);
    assert_eq!(lots.iter().map(|l| l.activites.len()).sum::<usize>(), 2);
}

// ------------------------------------------------------------------- suivi

/// Ecrit un journal jsonl dans un dossier de projet, comme Claude Code le fait.
fn poser_journal(dossier: &Path, nom: &str, contenu: &str) -> std::path::PathBuf {
    let projet = dossier.join("-Users-moi-Developer-atelier");
    std::fs::create_dir_all(&projet).expect("creation du dossier de projet");
    let chemin = projet.join(nom);
    std::fs::write(&chemin, contenu).expect("ecriture du journal");
    chemin
}

fn dossier_neuf(nom: &str) -> std::path::PathBuf {
    let dossier = std::env::temp_dir().join(format!("vibemap-journal-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).expect("creation du dossier de test");
    dossier
}

#[test]
fn la_relecture_ne_rend_que_les_lignes_ajoutees() {
    let dossier = dossier_neuf("relecture");
    let premiere = format!(
        "{}\n",
        ligne(
            "Read",
            serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/a.ts" }),
            "toolu_a",
            "2026-08-04T12:00:00.000Z",
        )
    );
    let chemin = poser_journal(&dossier, "session.jsonl", &premiere);

    let mut suivi = Suivi::new();
    let debut = Utc.with_ymd_and_hms(2026, 8, 4, 11, 0, 0).unwrap();

    let premier_tour = suivi.nouveaux(&dossier, debut).evenements;
    assert_eq!(premier_tour.len(), 1, "le premier tour rend la ligne posee");

    assert!(
        suivi.nouveaux(&dossier, debut).evenements.is_empty(),
        "sans ecriture nouvelle, un second tour ne rend rien"
    );

    let suite = ligne(
        "Edit",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/a.ts" }),
        "toolu_b",
        "2026-08-04T12:00:10.000Z",
    );
    let mut fichier = std::fs::OpenOptions::new()
        .append(true)
        .open(&chemin)
        .expect("ouverture en ajout");
    std::io::Write::write_all(&mut fichier, format!("{suite}\n").as_bytes())
        .expect("ajout au journal");

    let second_tour = suivi.nouveaux(&dossier, debut).evenements;
    assert_eq!(second_tour.len(), 1);
    assert_eq!(second_tour[0].tool_use_id, "toolu_b");
}

#[test]
fn un_tour_rapporte_aussi_la_consommation() {
    let dossier = dossier_neuf("conso");
    let contenu = format!(
        "{}\n",
        ligne_usage(
            "claude-sonnet-4-5",
            serde_json::json!({
                "input_tokens": 5,
                "output_tokens": 50,
                "cache_read_input_tokens": 100,
                "cache_creation_input_tokens": 10
            }),
            "2026-08-04T12:00:00.000Z",
        )
    );
    poser_journal(&dossier, "session.jsonl", &contenu);

    let horizon = Utc.with_ymd_and_hms(2026, 8, 4, 11, 0, 0).unwrap();
    let lecture = Suivi::new().nouveaux(&dossier, horizon);

    assert_eq!(lecture.usages.len(), 1, "le tour rapporte la consommation");
    assert_eq!(lecture.usages[0].model, "claude-sonnet-4-5");
    assert_eq!(lecture.usages[0].output, 50);
}

#[test]
fn le_passe_lointain_ne_remonte_pas_au_premier_tour() {
    let dossier = dossier_neuf("horizon");
    let vieux = ligne(
        "Edit",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/a.ts" }),
        "toolu_vieux",
        "2020-01-01T00:00:00.000Z",
    );
    let recent = ligne(
        "Edit",
        serde_json::json!({ "file_path": "/Users/moi/Developer/atelier/b.ts" }),
        "toolu_recent",
        "2026-08-04T12:00:00.000Z",
    );
    poser_journal(&dossier, "session.jsonl", &format!("{vieux}\n{recent}\n"));

    let horizon = Utc.with_ymd_and_hms(2026, 8, 4, 11, 50, 0).unwrap();
    let evenements = Suivi::new().nouveaux(&dossier, horizon).evenements;

    assert_eq!(evenements.len(), 1);
    assert_eq!(evenements[0].tool_use_id, "toolu_recent");
}
