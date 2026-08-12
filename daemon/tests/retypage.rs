//! Changer le type d'un travail a tout moment, sans effet sur son etat ni son
//! historique (issue #34, F10 du PRD, FR-032).
//!
//! Le retypage est un PATCH direct sur la seule colonne `type`, exactement
//! comme le bouton de sortie de Termine (#33) patche `statut` seul. Rien
//! cote base ne protege `type` : ni `statut_done_protege()` (qui ne regarde
//! que les ecritures qui CHANGENT `statut`), ni `bloc_statut_protege()` (qui
//! ne regarde, lui non plus, que `statut`). Ce fichier le reprouve nommement
//! plutot que de compter sur l'absence de trigger pour deviner que ça marche.
//!
//! Note de conception (#34) : seuls les BLOCS ont une colonne `type` -
//! `issues` n'en porte aucune (verifie dans les migrations,
//! `20260812000003_issues.sql`). Une issue n'apparait jamais comme une carte
//! autonome dans une colonne (F6, FR-021) ; le filtre et le retypage du
//! tableau ne portent donc que sur les blocs, ce que le schema impose deja,
//! pas seulement ce que l'UI choisit de faire.
//!
//! Comme partout ailleurs (ADR 0001), ça parle a la vraie pile Supabase
//! locale.

mod common;

use serde_json::json;

async fn repo_de_test(ctx: &common::TestContext) -> (vibemap::Supabase, String) {
    let machine_id = ctx.create_machine("MacBook Pro").await;
    let repo_id = ctx.creer_repo(&machine_id, &["web/app/checkout", "daemon/src"]).await;
    (vibemap::Supabase::new(&ctx.url, &ctx.user_token), repo_id)
}

fn commit_qui_nomme(sha: &str, ref_: i64) -> vibemap::CommitLocal {
    vibemap::CommitLocal {
        sha: sha.to_string(),
        message: format!("feat: cloture\n\nVM-{ref_}"),
        authored_at: chrono::Utc::now(),
    }
}

/// Critere d'acceptation central de #34 : retyper une carte « correction » en
/// « feature » ne bouge ni son statut ni son historique - ici, un bloc encore
/// a faire, le cas le plus simple.
#[tokio::test]
async fn retyper_un_bloc_a_faire_ne_touche_ni_statut_ni_version() {
    let ctx = common::TestContext::new().await;
    let (_client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Mauvais rangement", "correction", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let resultat = ctx
        .patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "type": "feature" }))
        .await;
    assert!(resultat.is_ok(), "le retypage doit etre accepte : {resultat:?}");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["type"], json!("feature"), "le nouveau type doit etre pose");
    assert_eq!(relu["statut"], json!("todo"), "le retypage ne doit pas toucher au statut");
    assert_eq!(relu["version"], json!(1), "le retypage ne doit pas toucher a la version");
}

/// Le coeur de FR-032 : « a tout moment » inclut une carte deja terminee -
/// permis cote base depuis #33 (statut_done_protege() ne protege que les
/// ecritures qui CHANGENT `statut`, jamais les autres colonnes d'une ligne
/// deja `done`). Le retypage doit reussir et laisser `statut`, `version` et
/// l'historique des fermetures parfaitement intacts.
#[tokio::test]
async fn retyper_un_bloc_termine_reussit_et_laisse_statut_version_et_fermetures_intacts() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Correction livree", "correction", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let ref_bloc = bloc["ref"].as_i64().unwrap();

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c1", ref_bloc)).await.unwrap();
    let avant = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(avant["statut"], json!("done"), "le commit doit fermer le bloc avant le retypage");
    assert_eq!(avant["version"], json!(1));
    let fermetures_avant = ctx.lire_fermetures_bloc(&bloc_id).await;
    assert_eq!(fermetures_avant.len(), 1, "une fermeture doit deja exister avant le retypage");

    let resultat = ctx
        .patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "type": "feature" }))
        .await;
    assert!(
        resultat.is_ok(),
        "le retypage d'une carte terminee doit etre accepte (FR-032, permis par #33) : {resultat:?}"
    );

    let apres = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres["type"], json!("feature"), "le nouveau type doit etre pose");
    assert_eq!(apres["statut"], json!("done"), "le retypage ne doit jamais rouvrir une carte terminee");
    assert_eq!(apres["version"], json!(1), "le retypage ne doit jamais faire avancer la version");

    let fermetures_apres = ctx.lire_fermetures_bloc(&bloc_id).await;
    assert_eq!(
        fermetures_apres, fermetures_avant,
        "l'historique des fermetures doit rester bit a bit identique apres un retypage"
    );
}

/// Un bloc terminee puis retype reste sortable de Termine exactement comme
/// avant (#33) : le retypage n'ajoute ni ne retire aucun geste, les deux
/// mecanismes vivent cote a cote sans interference.
#[tokio::test]
async fn un_bloc_termine_retype_reste_sortable_de_termine() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Correction livree", "correction", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let ref_bloc = bloc["ref"].as_i64().unwrap();

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c1", ref_bloc)).await.unwrap();

    ctx.patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "type": "feature" }))
        .await
        .expect("le retypage doit reussir");

    let resultat = ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "doing").await;
    assert!(resultat.is_ok(), "la sortie de Termine doit rester possible apres un retypage : {resultat:?}");
    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("doing"));
}

/// Le statut d'un bloc decoupe est derive de ses issues et refuse toute
/// ecriture directe (FR-018, #30) ; le retypage, lui, ne touche jamais
/// `statut` et doit donc passer sans encombre, meme sur un bloc decoupe dont
/// un PATCH de statut direct serait refuse.
#[tokio::test]
async fn retyper_un_bloc_decoupe_reussit_sans_toucher_a_son_statut_derive() {
    let ctx = common::TestContext::new().await;
    let (_client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du panier", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    ctx.creer_issue(&bloc_id, "Premiere issue", None).await;
    ctx.creer_issue(&bloc_id, "Seconde issue", Some("web/app/checkout/paiement")).await;

    let decoupe = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(decoupe["chemin"], serde_json::Value::Null, "le bloc doit bien etre decoupe");
    assert_eq!(decoupe["statut"], json!("todo"));

    // Un PATCH direct de statut, lui, doit rester refuse sur un bloc decoupe
    // (FR-018) - controle de non-regression avant de prouver que le
    // retypage, lui, passe.
    let statut_refuse = ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "doing").await;
    assert!(statut_refuse.is_err(), "un bloc decoupe ne doit pas se deplacer directement (FR-018)");

    let retypage = ctx
        .patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "type": "technique" }))
        .await;
    assert!(retypage.is_ok(), "le retypage d'un bloc decoupe doit reussir : {retypage:?}");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["type"], json!("technique"));
    assert_eq!(relu["statut"], json!("todo"), "le statut derive reste celui de ses issues, inchange");
}

/// La contrainte `check` du type refuse toujours une valeur hors des quatre
/// connues (§4.1 de la conception) : le retypage n'ouvre pas de nouvelle
/// porte, il emprunte la meme colonne que la creation, avec la meme regle.
#[tokio::test]
async fn retyper_vers_un_type_hors_de_la_contrainte_check_est_refuse() {
    let ctx = common::TestContext::new().await;
    let (_client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Mauvais rangement", "correction", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let resultat = ctx
        .patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "type": "chore" }))
        .await;
    assert!(resultat.is_err(), "un type hors des quatre connus doit etre refuse par la contrainte check");

    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["type"],
        json!("correction"),
        "un retypage refuse ne doit rien avoir change"
    );
}
