//! Decouper un bloc en issues, et deriver son etat depuis elles (issue #30).
//!
//! Ces tests parlent a une vraie pile Supabase locale, pas a un simulacre :
//! la descente du chemin, la derivation de l'etat et le refus d'un
//! deplacement direct vivent tous en SQL, seul un vrai appel les prouve
//! (ADR 0001).

mod common;

use serde_json::json;

/// Appaire une machine et pose un repo a son nom, pour porter des blocs.
async fn repo_de_test(ctx: &common::TestContext) -> String {
    let machine_id = ctx.create_machine("MacBook Pro").await;
    ctx.creer_repo(&machine_id, &["web/app/hero", "web/app/checkout"]).await
}

/// Le critere d'acceptation central de l'issue (FR-006, FR-007) : un bloc
/// simple ancre a `web/app/hero` recoit sa premiere issue, elle porte
/// `web/app/hero` et le bloc n'a plus d'emplacement. Le chemin fourni a la
/// creation ("peu importe") est ignore : la descente est un invariant tenu
/// par la base, pas une simple valeur par defaut cote application.
#[tokio::test]
async fn la_premiere_issue_fait_descendre_le_chemin_du_bloc_et_le_vide() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    let issue = ctx.creer_issue(&bloc_id, "Nouveau visuel", Some("peu importe")).await;

    assert_eq!(
        issue["chemin"],
        json!("web/app/hero"),
        "la premiere issue doit porter l'emplacement du bloc, pas celui fourni a la creation"
    );
    assert_eq!(issue["statut"], json!("todo"));
    assert!(issue["ref"].as_i64().is_some(), "l'issue doit porter une reference entiere");

    let bloc_relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        bloc_relu["chemin"],
        json!(null),
        "un bloc ne porte jamais a la fois un emplacement et des issues (FR-007)"
    );
}

/// Une fois le bloc decoupe (chemin videe par la premiere issue), toute
/// issue suivante doit porter son propre emplacement : il n'y a plus rien a
/// descendre.
#[tokio::test]
async fn une_issue_suivante_doit_porter_son_propre_emplacement() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    ctx.creer_issue(&bloc_id, "Premiere issue", None).await;

    let resultat = ctx.creer_issue_avec_jeton(&ctx.user_token, &bloc_id, "Seconde issue", None).await;
    assert!(
        resultat.is_err(),
        "une issue sans emplacement doit etre refusee une fois le bloc deja decoupe"
    );

    let issue = ctx.creer_issue(&bloc_id, "Seconde issue", Some("web/app/hero/bandeau")).await;
    assert_eq!(issue["chemin"], json!("web/app/hero/bandeau"));
}

/// Une issue ne peut pas se substituer a un bloc parent (FR-008) : le modele
/// n'a pas de colonne pour une sous-issue, et `bloc_id` ne peut viser que la
/// table `blocs`. Le detour consistant a passer l'id d'une issue existante en
/// guise de bloc doit donc echouer, pas creer une hierarchie a trois niveaux.
#[tokio::test]
async fn une_issue_ne_peut_pas_servir_de_bloc_a_une_autre_issue() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();
    let issue = ctx.creer_issue(&bloc_id, "Premiere issue", None).await;
    let issue_id = issue["id"].as_str().expect("id de l'issue").to_string();

    let resultat = ctx
        .creer_issue_avec_jeton(&ctx.user_token, &issue_id, "Sous-issue interdite", Some("web/app/hero"))
        .await;

    assert!(
        resultat.is_err(),
        "une issue ne doit jamais pouvoir servir de bloc a une autre issue"
    );
}

/// Le meme detour, hors RPC cette fois : une insertion brute qui vise l'id
/// d'une issue en guise de bloc doit echouer sur la contrainte de cle
/// etrangere elle-meme, pas seulement sur la verification que fait deja
/// `creer_issue` plus haut dans la pile (FR-008 : "verifie qu'aucun chemin
/// detourne ne le permet").
#[tokio::test]
async fn une_insertion_brute_ne_permet_pas_non_plus_une_sous_issue() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();
    let issue = ctx.creer_issue(&bloc_id, "Premiere issue", None).await;
    let issue_id = issue["id"].as_str().expect("id de l'issue").to_string();

    let resultat = ctx.tenter_inserer_issue_brute(&repo_id, &issue_id, 9999, "web/app/hero").await;

    assert!(
        resultat.is_err(),
        "le schema doit refuser bloc_id pointant vers une issue, meme par une insertion directe"
    );
}

/// Une issue declaree dans un depot ne peut pas s'accrocher au bloc d'un
/// autre depot : rien dans le schema ne reliait `issues.bloc_id` a
/// `issues.repo_id`, seule la cle etrangere simple sur `blocs.id` etait
/// verifiee. Deux consequences a prouver mortes : l'insertion doit echouer,
/// et le bloc etranger doit ressortir INCHANGE - `bloc_coherent()` est
/// `security definer` et viderait sinon l'emplacement d'un bloc qui n'a rien
/// a voir avec le depot de l'issue.
#[tokio::test]
async fn une_issue_ne_peut_pas_sattacher_a_un_bloc_dun_autre_depot() {
    let ctx = common::TestContext::new().await;
    let machine_id = ctx.create_machine("MacBook Pro").await;
    let repo_a = ctx.creer_repo(&machine_id, &["web/a"]).await;
    let repo_b = ctx.creer_repo(&machine_id, &["web/b"]).await;

    let bloc = ctx.creer_bloc(&repo_a, "Bloc du depot A", "feature", "web/a").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    let resultat = ctx.tenter_inserer_issue_brute(&repo_b, &bloc_id, 999, "web/a").await;
    assert!(
        resultat.is_err(),
        "une issue du depot B ne doit pas pouvoir s'accrocher a un bloc du depot A"
    );

    let bloc_relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        bloc_relu["chemin"],
        json!("web/a"),
        "le bloc etranger ne doit pas avoir ete modifie par la tentative refusee"
    );
}

/// Un bloc decoupe suit ses issues (§4.4) : sa case n'est jamais ecrite a la
/// main, elle est deduite a chaque changement d'une de ses issues.
#[tokio::test]
async fn un_bloc_decoupe_suit_ses_issues() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    let issue_1 = ctx.creer_issue(&bloc_id, "Premiere issue", None).await;
    let issue_2 = ctx.creer_issue(&bloc_id, "Seconde issue", Some("web/app/hero/bandeau")).await;
    let issue_1_id = issue_1["id"].as_str().expect("id issue 1").to_string();
    let issue_2_id = issue_2["id"].as_str().expect("id issue 2").to_string();

    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("todo"),
        "deux issues a faire : le bloc reste a faire"
    );

    ctx.poser_statut_issue(&issue_1_id, "doing").await;
    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("doing"),
        "une issue entamee suffit a passer le bloc en cours"
    );

    ctx.poser_statut_issue(&issue_1_id, "done").await;
    ctx.poser_statut_issue(&issue_2_id, "done").await;
    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("done"),
        "toutes les issues terminees : le bloc est termine"
    );
}

/// Le scenario nomme du PRD (F5, critere d'acceptation) : neuf issues dont
/// une seule entamee suffisent a faire passer le bloc en « En cours ».
#[tokio::test]
async fn neuf_issues_dont_une_entamee_le_bloc_est_en_cours() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Chantier a neuf issues", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    let mut ids = Vec::new();
    for n in 0..9 {
        let chemin = if n == 0 { None } else { Some("web/app/hero/partie") };
        let issue = ctx.creer_issue(&bloc_id, &format!("Issue {n}"), chemin).await;
        ids.push(issue["id"].as_str().expect("id issue").to_string());
    }

    ctx.poser_statut_issue(&ids[0], "doing").await;

    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("doing"));
}

/// Un bloc simple - sans aucune issue - garde son statut propre : c'est le
/// cas `v_total = 0` de `etat_bloc`, qui ne touche a rien. Le PATCH direct
/// que le web fera plus tard (#31, #32) doit donc rester possible ici.
#[tokio::test]
async fn un_bloc_simple_garde_son_statut_propre() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Correction rapide", "correction", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    let resultat = ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "doing").await;
    assert!(resultat.is_ok(), "un bloc simple doit rester deplacable directement : {resultat:?}");
    assert_eq!(ctx.lire_bloc(&bloc_id).await["statut"], json!("doing"));
}

/// FR-018 : un bloc decoupe ne peut pas etre deplace directement, ni par le
/// web ni par quiconque - seule `etat_bloc()`, appelee depuis le trigger des
/// issues, a le droit d'ecrire son statut.
#[tokio::test]
async fn un_bloc_decoupe_refuse_un_deplacement_direct() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();
    ctx.creer_issue(&bloc_id, "Premiere issue", None).await;

    let resultat = ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &bloc_id, "doing").await;
    assert!(
        resultat.is_err(),
        "un bloc decoupe ne doit jamais accepter un deplacement direct de son statut"
    );
    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("todo"),
        "le refus ne doit laisser aucune trace du deplacement tente"
    );
}

/// Le mecanisme de reference de #29 se prolonge, il ne se duplique pas : un
/// bloc et une issue du meme depot ne partagent jamais de reference, qu'ils
/// puisent dans le meme compteur `compteur_ref`.
#[tokio::test]
async fn blocs_et_issues_puisent_au_meme_compteur() {
    let ctx = common::TestContext::new().await;
    let repo_id = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Refonte du hero", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();
    let bloc_ref = bloc["ref"].as_i64().expect("reference du bloc");

    let issue = ctx.creer_issue(&bloc_id, "Premiere issue", None).await;
    let issue_ref = issue["ref"].as_i64().expect("reference de l'issue");

    let second_bloc = ctx.creer_bloc(&repo_id, "Autre travail", "technique", "web/app/checkout").await;
    let second_bloc_ref = second_bloc["ref"].as_i64().expect("reference du second bloc");

    let references = [bloc_ref, issue_ref, second_bloc_ref];
    let distinctes: std::collections::HashSet<_> = references.iter().collect();
    assert_eq!(
        distinctes.len(),
        references.len(),
        "blocs et issues ne doivent jamais partager de reference : {references:?}"
    );

    assert!(
        issue_ref > bloc_ref && second_bloc_ref > issue_ref,
        "le compteur doit avancer dans l'ordre des creations, references obtenues : {references:?}"
    );
}

/// Un compte ne voit rien des issues d'un autre : la RLS suit son depot,
/// comme celle des blocs (#29).
#[tokio::test]
async fn un_compte_ne_voit_rien_des_issues_dun_autre_compte() {
    let proprietaire = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;

    let repo_id = repo_de_test(&proprietaire).await;
    let bloc = proprietaire.creer_bloc(&repo_id, "Travail prive", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();
    proprietaire.creer_issue(&bloc_id, "Issue privee", None).await;

    let vu_par_lintrus = intrus.lire_issues_avec_jeton(&intrus.user_token, &repo_id).await;
    assert!(
        vu_par_lintrus.is_empty(),
        "un autre compte ne doit voir aucune issue de ce depot, recu : {vu_par_lintrus:?}"
    );
}

/// Un compte etranger ne peut pas non plus decouper le bloc d'un autre : la
/// RLS refuse l'ecriture, pas seulement la lecture.
#[tokio::test]
async fn un_compte_ne_peut_pas_creer_une_issue_dans_le_bloc_dun_autre() {
    let proprietaire = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;

    let repo_id = repo_de_test(&proprietaire).await;
    let bloc = proprietaire.creer_bloc(&repo_id, "Travail prive", "feature", "web/app/hero").await;
    let bloc_id = bloc["id"].as_str().expect("id du bloc").to_string();

    let resultat = intrus
        .creer_issue_avec_jeton(&intrus.user_token, &bloc_id, "Issue volee", Some("web/app/hero"))
        .await;

    assert!(
        resultat.is_err(),
        "un compte etranger ne doit pas pouvoir decouper le bloc d'un autre"
    );

    let issues = proprietaire.lire_issues(&repo_id).await;
    assert!(issues.is_empty(), "aucune issue ne doit avoir ete creee malgre la tentative");
}
