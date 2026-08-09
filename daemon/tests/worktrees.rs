//! Les worktrees arrivent en base et se reconcilient (issue #6).
//!
//! Comme l'activite et le cout, ces tests parlent a une vraie pile Supabase
//! locale : la reconciliation vit en SQL, et seul un vrai appel verifie qu'elle
//! tient. Ils prouvent les criteres d'acceptation cote donnees : un worktree
//! ouvert apparait, ferme il disparait, et c'est bien sa branche qui est notee.

mod common;

use vibemap::Worktree;

/// Appaire une machine et rend son identite.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

fn wt(path: &str, branch: &str) -> Worktree {
    Worktree { path: path.to_string(), branch: branch.to_string() }
}

/// Critere 1 (cote donnees) : pousser un worktree le rend visible, avec sa branche.
#[tokio::test]
async fn un_worktree_pousse_apparait_ouvert_avec_sa_branche() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    client
        .pousser_worktrees(&repo_id, &[wt("atelier-hotfix", "hotfix")])
        .await
        .expect("le worktree doit etre accepte");

    let ouverts = ctx.worktrees_ouverts(&repo_id).await;
    assert_eq!(ouverts.len(), 1, "un worktree ouvert");
    // Critere 4 : le badge porte la branche du worktree, pas la principale.
    assert_eq!(ouverts[0]["branch"].as_str(), Some("hotfix"));
    assert_eq!(ouverts[0]["path"].as_str(), Some("atelier-hotfix"));
}

/// Critere 2 (cote donnees) : un worktree absent du releve suivant disparait.
#[tokio::test]
async fn un_worktree_retire_du_releve_disparait() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    // Deux worktrees, puis un releve qui n'en garde qu'un.
    client
        .pousser_worktrees(&repo_id, &[wt("a", "feat-a"), wt("b", "feat-b")])
        .await
        .expect("premier releve accepte");
    assert_eq!(ctx.worktrees_ouverts(&repo_id).await.len(), 2);

    client
        .pousser_worktrees(&repo_id, &[wt("a", "feat-a")])
        .await
        .expect("second releve accepte");

    let ouverts = ctx.worktrees_ouverts(&repo_id).await;
    assert_eq!(ouverts.len(), 1, "b a disparu du releve, il se ferme");
    assert_eq!(ouverts[0]["branch"].as_str(), Some("feat-a"));
}

/// Un releve vide ferme le dernier worktree : le canal revient au repos.
#[tokio::test]
async fn un_releve_vide_ferme_tous_les_worktrees() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    client
        .pousser_worktrees(&repo_id, &[wt("a", "feat-a")])
        .await
        .expect("releve accepte");
    assert_eq!(ctx.worktrees_ouverts(&repo_id).await.len(), 1);

    client.pousser_worktrees(&repo_id, &[]).await.expect("releve vide accepte");
    assert!(
        ctx.worktrees_ouverts(&repo_id).await.is_empty(),
        "plus aucun worktree ouvert"
    );
}

/// Rouvrir un worktree ferme reutilise sa ligne : la branche peut avoir change.
#[tokio::test]
async fn rouvrir_un_worktree_ferme_le_rend_a_nouveau_visible() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    client.pousser_worktrees(&repo_id, &[wt("a", "feat-a")]).await.unwrap();
    client.pousser_worktrees(&repo_id, &[]).await.unwrap();
    assert!(ctx.worktrees_ouverts(&repo_id).await.is_empty());

    // Le meme dossier revient, sur une autre branche.
    client.pousser_worktrees(&repo_id, &[wt("a", "feat-a-suite")]).await.unwrap();

    let ouverts = ctx.worktrees_ouverts(&repo_id).await;
    assert_eq!(ouverts.len(), 1, "le worktree rouvert reapparait");
    assert_eq!(ouverts[0]["branch"].as_str(), Some("feat-a-suite"), "la branche est a jour");
}
