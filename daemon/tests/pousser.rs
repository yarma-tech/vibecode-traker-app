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

/// Ajoute un remote `origin` au depot, sans jamais toucher au reseau.
fn poser_remote(racine: &Path, url: &str) {
    Command::new("git")
        .args(["remote", "add", "origin", url])
        .current_dir(racine)
        .status()
        .expect("git remote add");
}

/// Le meme distant sur deux clones distincts : c'est ce qu'on obtient en
/// clonant le meme depot sur deux machines. L'identite doit les faire tomber
/// sur la meme ligne (conception, section 3 ; PRD FR-030).
#[tokio::test]
async fn le_meme_depot_clone_sur_deux_machines_produit_une_seule_ligne() {
    let ctx = common::TestContext::new().await;
    let machine_a = machine_reliee(&ctx).await;
    let machine_b = machine_reliee(&ctx).await;

    let clone_a = petit_depot();
    poser_remote(&clone_a, "git@github.com:yarma-tech/atelier.git");
    let clone_b = petit_depot();
    poser_remote(&clone_b, "https://github.com/yarma-tech/atelier.git");

    let client_a = vibemap::Supabase::new(&ctx.url, &machine_a.token);
    let client_b = vibemap::Supabase::new(&ctx.url, &machine_b.token);

    let plan_a = vibemap::scanner(Path::new(&clone_a)).expect("scan machine A");
    let repo_id_a = client_a
        .pousser_plan(&machine_a.machine_id, &plan_a)
        .await
        .expect("la machine A doit pouvoir poser son plan");

    let plan_b = vibemap::scanner(Path::new(&clone_b)).expect("scan machine B");
    let repo_id_b = client_b
        .pousser_plan(&machine_b.machine_id, &plan_b)
        .await
        .expect("la machine B doit pouvoir ecrire sur la ligne posee par A");

    assert_eq!(
        repo_id_a, repo_id_b,
        "les deux clones du meme distant doivent partager une seule ligne repos"
    );

    let repo = ctx.lire_repo(&repo_id_a).await;
    assert_eq!(
        repo["identity"], "github.com/yarma-tech/atelier",
        "l'identite est celle du distant normalise, peu importe la forme d'URL de chaque clone"
    );
}

/// Un dossier de depot renomme garde sa ligne (PRD FR-030) : le chemin
/// absolu change, donc l'empreinte aussi, mais l'identite ne bouge pas tant
/// que le distant reste le meme.
#[tokio::test]
async fn un_depot_renomme_garde_sa_ligne() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let avant = petit_depot();
    poser_remote(&avant, "git@github.com:yarma-tech/renomme.git");

    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let plan = vibemap::scanner(Path::new(&avant)).expect("premier scan");
    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("premier envoi");

    let apres = avant
        .parent()
        .unwrap()
        .join(format!("renomme-{}", uuid::Uuid::new_v4()));
    std::fs::rename(&avant, &apres).expect("renommage du dossier");

    let plan = vibemap::scanner(Path::new(&apres)).expect("second scan, apres renommage");
    let second_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("second envoi");

    assert_eq!(
        second_id, repo_id,
        "le renommage ne doit pas creer une seconde ligne"
    );
}

/// Un depot sans distant recoit un distant plus tard (par exemple un
/// `git remote add`) : la ligne existante doit etre reprise, pas dupliquee
/// (conception, section 3 : « le daemon rattache la ligne existante »).
#[tokio::test]
async fn un_distant_ajoute_plus_tard_rattache_la_ligne_existante() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let racine = petit_depot();
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let plan_local = vibemap::scanner(Path::new(&racine)).expect("premier scan, sans distant");
    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan_local)
        .await
        .expect("premier envoi");

    let avant = ctx.lire_repo(&repo_id).await;
    assert!(
        avant["identity"].as_str().unwrap().starts_with("local:"),
        "sans distant, l'identite doit etre locale : {avant}"
    );

    poser_remote(&racine, "git@github.com:yarma-tech/rattachement.git");
    let plan_distant = vibemap::scanner(Path::new(&racine)).expect("second scan, avec distant");
    let second_id = client
        .pousser_plan(&machine.machine_id, &plan_distant)
        .await
        .expect("second envoi");

    assert_eq!(
        second_id, repo_id,
        "la ligne existante doit etre reprise, pas dupliquee"
    );

    let apres = ctx.lire_repo(&repo_id).await;
    assert_eq!(apres["identity"], "github.com/yarma-tech/rattachement");
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
