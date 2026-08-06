//! Le plan arrive en base, et un second scan ne cree pas de doublon.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn petit_depot() -> PathBuf {
    let racine = std::env::temp_dir().join(format!("vibemap-push-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(racine.join("src")).unwrap();

    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&racine)
        .status()
        .expect("git init");

    std::fs::write(racine.join("README.md"), "titre\n").unwrap();
    std::fs::write(racine.join("src/a.rs"), "une\ndeux\ntrois\n").unwrap();
    racine
}

/// Appaire une machine et rend son jeton.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

#[tokio::test]
async fn le_plan_arrive_en_base() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let racine = petit_depot();

    let plan = vibemap::scanner(Path::new(&racine)).expect("scan");
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("le plan doit etre accepte");

    let repo = ctx.lire_repo(&repo_id).await;
    assert_eq!(repo["loc_total"], 4, "README (1) et src/a.rs (3)");
    assert_eq!(repo["file_count"], 2);

    let modules = ctx.lire_modules(&repo_id).await;
    let src = modules
        .iter()
        .find(|m| m["path"] == "src")
        .expect("le module src doit exister");
    assert_eq!(src["loc"], 3);
}

#[tokio::test]
async fn un_second_scan_met_a_jour_sans_dupliquer() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let racine = petit_depot();

    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let plan = vibemap::scanner(Path::new(&racine)).expect("premier scan");
    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("premier envoi");

    // Le repo grossit entre deux scans.
    std::fs::write(racine.join("src/b.rs"), "une\ndeux\n").unwrap();

    let plan = vibemap::scanner(Path::new(&racine)).expect("second scan");
    let second_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("second envoi");

    assert_eq!(second_id, repo_id, "le meme repo, pas un nouveau");

    let repo = ctx.lire_repo(&repo_id).await;
    assert_eq!(repo["loc_total"], 6, "les deux lignes ajoutees sont comptees");

    let modules = ctx.lire_modules(&repo_id).await;
    let src: Vec<_> = modules.iter().filter(|m| m["path"] == "src").collect();
    assert_eq!(src.len(), 1, "un seul module src, pas deux");
    assert_eq!(src[0]["loc"], 5);
}

/// Un dossier disparu du disque disparait de la carte.
#[tokio::test]
async fn un_module_supprime_disparait_de_la_carte() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let racine = petit_depot();
    std::fs::create_dir_all(racine.join("docs")).unwrap();
    std::fs::write(racine.join("docs/x.md"), "doc\n").unwrap();

    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let plan = vibemap::scanner(Path::new(&racine)).expect("scan");
    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("envoi");

    assert!(
        ctx.lire_modules(&repo_id).await.iter().any(|m| m["path"] == "docs"),
        "docs doit d'abord exister"
    );

    std::fs::remove_dir_all(racine.join("docs")).unwrap();
    let plan = vibemap::scanner(Path::new(&racine)).expect("second scan");
    client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("second envoi");

    assert!(
        !ctx.lire_modules(&repo_id).await.iter().any(|m| m["path"] == "docs"),
        "docs a disparu du disque, il doit disparaitre de la carte"
    );
}
