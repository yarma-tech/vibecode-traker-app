//! Sortir de « Termine », sans jamais y entrer a la main (issue #33, F8 du
//! PRD, §10 de la conception).
//!
//! Deux verrous distincts a eprouver ici : le serveur accepte le seul geste
//! de sortie (FR-025), et refuse toute ecriture directe de `statut = 'done'`
//! venue d'un client (FR-026) - sur `blocs` comme sur `issues`, en blocs
//! simples comme en blocs decoupes, meme quand la requete tente de se
//! confondre avec d'autres colonnes. `fermer_par_reference()` (#32) et
//! `etat_bloc()` (#30) continuent, eux, de poser `done` normalement : ce
//! fichier le reprouve nommement plutot que de compter sur le seul non-echec
//! de `commits.rs` et `issues.rs`.
//!
//! Comme partout ailleurs (ADR 0001), ça parle a la vraie pile Supabase
//! locale.

mod common;

use serde_json::json;

/// Appaire une machine et pose un repo a son nom, avec son client daemon.
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

// --------------------------------------------------------------------------
// FR-025 : la seule sortie de « Termine », vers « En cours ».
// --------------------------------------------------------------------------

/// Critere d'acceptation central : une carte (bloc simple) terminee par un
/// vrai commit peut etre ramenee en cours depuis le tableau - un PATCH direct
/// avec le jeton de l'utilisateur, comme le fera le bouton de sortie.
#[tokio::test]
async fn un_bloc_simple_termine_peut_etre_ramene_en_cours() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Correction a tort", "correction", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let ref_bloc = bloc["ref"].as_i64().unwrap();

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c1", ref_bloc)).await.unwrap();
    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("done"), "le commit doit fermer le bloc");

    let resultat = ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "doing").await;
    assert!(resultat.is_ok(), "la sortie de Termine doit etre acceptee : {resultat:?}");
    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("doing"));
}

/// Meme geste sur une issue d'un bloc decoupe : la sortie ramene l'issue en
/// cours, et le bloc parent la suit par derivation (#30), sans qu'aucun code
/// de ce chantier n'ait besoin de le reecrire.
#[tokio::test]
async fn une_issue_terminee_peut_etre_ramenee_en_cours_et_derive_son_bloc() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du panier", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue = ctx.creer_issue(&bloc_id, "Seule issue", None).await;
    let issue_id = issue["id"].as_str().unwrap().to_string();
    let ref_issue = issue["ref"].as_i64().unwrap();

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c1", ref_issue)).await.unwrap();
    assert_eq!(ctx.lire_issue(&issue_id).await["statut"], json!("done"));
    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("done"), "toutes les issues terminees ferment le bloc");

    let resultat = ctx.deplacer_issue_avec_jeton(&ctx.user_token, &issue_id, "doing").await;
    assert!(resultat.is_ok(), "la sortie de Termine doit etre acceptee sur une issue : {resultat:?}");
    assert_eq!(ctx.lire_issue(&issue_id).await["statut"], json!("doing"));
    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("doing"),
        "le bloc decoupe doit deriver en cours des qu'une issue en repart"
    );
}

/// Le scenario nomme du prompt : une carte ramenee en cours, puis nommee a
/// nouveau par un commit, se referme - la sortie n'use pas le mecanisme de
/// fermeture, elle ne fait que le reculer d'un pas.
///
/// La sortie est un PATCH direct, hors de `fermer_par_reference()` : elle ne
/// touche jamais `version` (seul le toggle de #32, sur un travail deja
/// `done`, l'incremente - §5.3 de la conception). Le second commit ferme
/// donc depuis `doing`, exactement comme le premier, avec la meme version.
#[tokio::test]
async fn une_carte_ramenee_en_cours_puis_nommee_par_un_commit_se_referme() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Ferme puis rouvert puis referme", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let ref_bloc = bloc["ref"].as_i64().unwrap();

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c1", ref_bloc)).await.unwrap();
    let apres_premiere_fermeture = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres_premiere_fermeture["statut"], json!("done"));
    assert_eq!(apres_premiere_fermeture["version"], json!(1));

    ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "doing")
        .await
        .expect("la sortie doit reussir");
    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("doing"));

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c2", ref_bloc)).await.unwrap();
    let apres_seconde_fermeture = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres_seconde_fermeture["statut"], json!("done"), "le second commit doit refermer la carte");
    assert_eq!(
        apres_seconde_fermeture["version"], json!(1),
        "la sortie ne passe pas par le toggle de #32 : la version ne bouge pas"
    );

    let fermetures = ctx.lire_fermetures_bloc(&bloc_id).await;
    assert_eq!(fermetures.len(), 2, "les deux fermetures doivent etre tracees, pas la sortie entre les deux");
}

// --------------------------------------------------------------------------
// FR-026 : aucune ecriture directe de `done`, ni sur blocs ni sur issues.
// --------------------------------------------------------------------------

#[tokio::test]
async fn un_client_ne_peut_pas_ecrire_done_directement_sur_un_bloc_simple() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Jamais ferme a la main", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let resultat = ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "done").await;
    assert!(resultat.is_err(), "un PATCH direct vers done doit etre refuse : {resultat:?}");
    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("todo"),
        "le refus ne doit laisser aucune trace de la tentative"
    );
}

#[tokio::test]
async fn un_client_ne_peut_pas_ecrire_done_directement_sur_une_issue() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Bloc decoupe", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue = ctx.creer_issue(&bloc_id, "Jamais fermee a la main", None).await;
    let issue_id = issue["id"].as_str().unwrap().to_string();

    let resultat = ctx.deplacer_issue_avec_jeton(&ctx.user_token, &issue_id, "done").await;
    assert!(resultat.is_err(), "un PATCH direct vers done doit etre refuse sur une issue : {resultat:?}");
    assert_eq!(ctx.lire_issue(&issue_id).await["statut"], json!("todo"));
    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("todo"),
        "le refus de l'issue ne doit pas non plus deriver le bloc parent"
    );
}

/// Tentative de contournement demandee par le prompt : glisser `statut =
/// 'done'` au milieu d'autres colonnes dans le meme PATCH. La policy regarde
/// la ligne resultante, pas la forme de la requete - le titre doit rester
/// intact lui aussi, la RLS annule tout le PATCH, jamais une partie.
#[tokio::test]
async fn un_update_qui_touche_dautres_colonnes_ne_fait_pas_passer_done() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Titre original", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let resultat = ctx
        .patch_bloc_avec_jeton(
            &ctx.user_token,
            &bloc_id,
            json!({ "titre": "Titre truque", "statut": "done" }),
        )
        .await;

    assert!(resultat.is_err(), "le detour par une autre colonne ne doit pas suffire : {resultat:?}");
    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["statut"], json!("todo"));
    assert_eq!(relu["titre"], json!("Titre original"), "le refus doit etre atomique, le titre n'a pas bouge");
}

/// Meme detour, sur une issue.
#[tokio::test]
async fn un_update_qui_touche_dautres_colonnes_ne_fait_pas_passer_done_sur_une_issue() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Bloc decoupe", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue = ctx.creer_issue(&bloc_id, "Titre original", None).await;
    let issue_id = issue["id"].as_str().unwrap().to_string();

    let resultat = ctx
        .patch_issue_avec_jeton(
            &ctx.user_token,
            &issue_id,
            json!({ "titre": "Titre truque", "statut": "done" }),
        )
        .await;

    assert!(resultat.is_err(), "le detour par une autre colonne ne doit pas suffire sur une issue : {resultat:?}");
    let relue = ctx.lire_issue(&issue_id).await;
    assert_eq!(relue["statut"], json!("todo"));
    assert_eq!(relue["titre"], json!("Titre original"));
}

/// Un jeton d'un autre compte ne peut ni voir ni fermer le bloc d'autrui : la
/// RLS de #29 continue de filtrer la ligne avant meme que la regle `done` de
/// #33 n'ait a s'appliquer.
#[tokio::test]
async fn un_jeton_dun_autre_compte_ne_peut_pas_fermer_le_bloc_dun_autre() {
    let proprietaire = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&proprietaire).await;

    let bloc = proprietaire.creer_bloc(&repo_id, "Prive", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let resultat = intrus.deplacer_bloc_avec_jeton(&intrus.user_token, &bloc_id, "done").await;
    assert!(resultat.is_err(), "un compte etranger ne doit pas pouvoir fermer ce bloc : {resultat:?}");
    assert_eq!(proprietaire.lire_bloc(&bloc_id).await["statut"], json!("todo"));
}

/// Contournement trouve en essayant de casser cette meme migration : la
/// policy `update` ne protege rien contre une ligne qui NAIT deja `done`. Un
/// POST direct sur `/rest/v1/blocs`, hors de `creer_bloc()`, doit donc etre
/// refuse lui aussi des la creation.
#[tokio::test]
async fn un_client_ne_peut_pas_creer_un_bloc_deja_termine() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let resultat = ctx
        .inserer_bloc_brut_avec_jeton(
            &ctx.user_token,
            json!({
                "user_id": ctx.user_id,
                "repo_id": repo_id,
                "ref": 999_001,
                "type": "feature",
                "titre": "Ne devrait jamais exister",
                "statut": "done",
                "chemin": "web/x",
            }),
        )
        .await;

    assert!(resultat.is_err(), "une ligne ne doit jamais pouvoir naitre deja terminee : {resultat:?}");
    assert!(
        ctx.lire_blocs(&repo_id).await.is_empty(),
        "l'insertion refusee ne doit laisser aucune ligne, meme partielle"
    );
}

/// Meme detour, sur `issues`.
#[tokio::test]
async fn un_client_ne_peut_pas_creer_une_issue_deja_terminee() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Bloc porteur", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let resultat = ctx
        .inserer_issue_brute_avec_jeton(
            &ctx.user_token,
            json!({
                "user_id": ctx.user_id,
                "repo_id": repo_id,
                "bloc_id": bloc_id,
                "ref": 999_002,
                "titre": "Ne devrait jamais exister",
                "chemin": "web/app/checkout",
                "statut": "done",
            }),
        )
        .await;

    assert!(resultat.is_err(), "une issue ne doit jamais pouvoir naitre deja terminee : {resultat:?}");
}

// --------------------------------------------------------------------------
// Ce qui doit continuer d'ecrire `done`, malgre la nouvelle regle.
// --------------------------------------------------------------------------

/// `fermer_par_reference()` (#32) est `security definer`, executee sous le
/// role proprietaire de la fonction (`postgres`) : la nouvelle regle ne doit
/// pas la genner, elle ferme toujours.
#[tokio::test]
async fn fermer_par_reference_ferme_toujours() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Ferme par un vrai commit", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let ref_bloc = bloc["ref"].as_i64().unwrap();

    client.ingerer_commit(&repo_id, Some("main"), &commit_qui_nomme("c1", ref_bloc)).await.unwrap();

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["statut"], json!("done"));
    assert_eq!(ctx.lire_fermetures_bloc(&bloc_id).await.len(), 1);
}

/// `etat_bloc()` (#30) est elle aussi `security definer` : un bloc decoupe
/// dont toutes les issues terminent doit toujours deriver en `done`.
#[tokio::test]
async fn un_bloc_decoupe_dont_toutes_les_issues_terminent_reste_derive_en_done() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Deux issues", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue_1 = ctx.creer_issue(&bloc_id, "Premiere", None).await;
    let issue_2 = ctx.creer_issue(&bloc_id, "Seconde", Some("web/app/checkout/autre")).await;

    ctx.poser_statut_issue(issue_1["id"].as_str().unwrap(), "done").await;
    ctx.poser_statut_issue(issue_2["id"].as_str().unwrap(), "done").await;

    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("done"),
        "toutes les issues terminees doivent toujours deriver le bloc en done"
    );
}

/// Decision documentee (voir le rapport de l'issue #33) : `service_role`
/// n'est jamais transmise au navigateur, elle vit dans le daemon et les
/// tests. Elle continue donc de pouvoir poser `done` directement - c'est deja
/// ce que `poser_statut_issue`/`poser_statut_bloc` font pour construire des
/// fixtures ailleurs dans cette suite.
#[tokio::test]
async fn la_cle_service_role_peut_toujours_ecrire_done_directement() {
    let ctx = common::TestContext::new().await;
    let (_, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Pose par la cle de service", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    ctx.poser_statut_bloc(&bloc_id, "done").await;

    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("done"));
}
