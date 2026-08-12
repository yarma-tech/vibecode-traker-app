//! Le tableau manuel : creer un bloc, le lire en trois colonnes (issue #29).
//!
//! Ces tests parlent a une vraie pile Supabase locale, pas a un simulacre :
//! l'attribution de reference et la RLS vivent en SQL, seul un vrai appel les
//! prouve (ADR 0001).

mod common;

use serde_json::json;

/// Appaire une machine et pose un repo a son nom, pour porter des blocs.
async fn repo_de_test(ctx: &common::TestContext) -> String {
    let machine_id = ctx.create_machine("MacBook Pro").await;
    ctx.creer_repo(&machine_id, &["web/app/checkout", "daemon/src"]).await
}

/// Tracer bullet : un bloc cree avec un titre et un emplacement arrive en
/// « A faire », porte une reference, et rien d'autre ne le distingue.
#[tokio::test]
async fn creer_un_bloc_larrivee_en_a_faire_avec_une_reference() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx
        .creer_bloc(&repo_id, "Refaire la landing", "feature", "web/app/landing")
        .await;

    assert_eq!(bloc["statut"], json!("todo"));
    assert_eq!(bloc["titre"], json!("Refaire la landing"));
    assert_eq!(bloc["type"], json!("feature"));
    assert_eq!(bloc["chemin"], json!("web/app/landing"));
    assert_eq!(bloc["version"], json!(1));
    assert!(bloc["ref"].as_i64().is_some(), "le bloc doit porter une reference entiere");
}

/// La saisie libre d'un chemin de fichier est acceptee, pas seulement un
/// dossier connu de `modules` (FR-002) : l'autocompletion ne fait que
/// suggerer, elle ne restreint jamais la saisie.
#[tokio::test]
async fn un_chemin_de_fichier_saisi_librement_est_accepte() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx
        .creer_bloc(&repo_id, "Corriger le calcul de la TVA", "correction", "web/lib/tva.ts")
        .await;

    assert_eq!(bloc["chemin"], json!("web/lib/tva.ts"));
}

/// Deux blocs du meme depot ne partagent jamais de reference : les
/// references puisent dans un compteur, pas dans un `max()`.
#[tokio::test]
async fn deux_blocs_du_meme_depot_ont_des_references_distinctes() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let a = ctx.creer_bloc(&repo_id, "Premier travail", "feature", "web/app/a").await;
    let b = ctx.creer_bloc(&repo_id, "Second travail", "feature", "web/app/b").await;

    assert_ne!(a["ref"], b["ref"]);
}

/// Le critere d'acceptation central de l'issue : une reference liberee par une
/// suppression n'est JAMAIS reattribuee. Avec un `max(ref) + 1`, supprimer le
/// bloc le plus recent aurait fait retomber sur la meme reference ; ce test
/// echoue exactement dans ce cas-la.
#[tokio::test]
async fn une_reference_supprimee_nest_jamais_reattribuee() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let premier = ctx.creer_bloc(&repo_id, "VM-1", "feature", "web/app/a").await;
    let second = ctx.creer_bloc(&repo_id, "VM-2, bientot supprime", "feature", "web/app/b").await;
    let ref_supprimee = second["ref"].as_i64().expect("le second bloc a une reference");

    ctx.supprimer_bloc(second["id"].as_str().expect("id du second bloc")).await;

    let troisieme = ctx.creer_bloc(&repo_id, "VM-3, apres suppression", "feature", "web/app/c").await;
    let nouvelle_ref = troisieme["ref"].as_i64().expect("le troisieme bloc a une reference");

    assert_ne!(
        nouvelle_ref, ref_supprimee,
        "la reference du bloc supprime {ref_supprimee} n'a pas ete reattribuee"
    );
    assert!(
        nouvelle_ref > second["ref"].as_i64().unwrap_or(0).max(premier["ref"].as_i64().unwrap_or(0)),
        "le compteur doit continuer d'avancer, meme apres une suppression"
    );
}

/// Un compte ne voit rien du tableau d'un autre : la selection filtre les
/// lignes plutot que de renvoyer une erreur, comme le reste de la RLS du depot.
#[tokio::test]
async fn un_compte_ne_voit_rien_du_tableau_dun_autre_compte() {
    let proprietaire = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;

    let repo_id = repo_de_test(&proprietaire).await;
    proprietaire
        .creer_bloc(&repo_id, "Travail prive", "feature", "web/app/prive")
        .await;

    let vu_par_lintrus = intrus.lire_blocs_avec_jeton(&intrus.user_token, &repo_id).await;
    assert!(
        vu_par_lintrus.is_empty(),
        "un autre compte ne doit voir aucun bloc de ce depot, recu : {vu_par_lintrus:?}"
    );
}

/// Un compte qui n'est proprietaire d'aucun repo ne peut pas non plus en
/// creer un bloc : l'ecriture est refusee, pas seulement la lecture.
#[tokio::test]
async fn un_compte_ne_peut_pas_creer_un_bloc_dans_le_depot_dun_autre() {
    let proprietaire = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;

    let repo_id = repo_de_test(&proprietaire).await;

    let resultat = intrus
        .creer_bloc_avec_jeton(&intrus.user_token, &repo_id, "Travail vole", "feature", "web/app/vole")
        .await;

    assert!(
        resultat.is_err(),
        "un compte etranger ne doit pas pouvoir creer un bloc dans un depot qui n'est pas le sien"
    );

    let blocs = proprietaire.lire_blocs(&repo_id).await;
    assert!(blocs.is_empty(), "aucun bloc ne doit avoir ete cree malgre la tentative");
}
