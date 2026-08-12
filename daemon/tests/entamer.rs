//! Entamer un travail des qu'un agent ecrit a son emplacement (issue #31).
//!
//! Le signal existe deja dans `activity_events` (#4) : rien a ajouter cote
//! daemon, tout se joue dans un trigger SQL sur l'insertion `kind = 'write'`.
//! Ces tests parlent donc a la vraie pile Supabase locale par le chemin reel
//! du daemon (`pousser_activite`), pas par une insertion de service qui
//! contournerait la RLS que ce chemin traverse (ADR 0001).

mod common;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use vibemap::Activite;

/// Appaire une machine et rend son identite, comme le ferait le daemon.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

/// Un repo pret a porter des blocs et des issues, avec son client daemon.
async fn repo_de_test(ctx: &common::TestContext) -> (vibemap::Supabase, String, String) {
    let machine = machine_reliee(ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["web/app/checkout", "daemon/src"]).await;
    (vibemap::Supabase::new(&ctx.url, &machine.token), machine.machine_id, repo_id)
}

fn ecriture(fichier: &str) -> Activite {
    Activite {
        session_id: Uuid::new_v4().to_string(),
        tool_use_id: Uuid::new_v4().to_string(),
        module_path: fichier.rsplit_once('/').map(|(m, _)| m.to_string()).unwrap_or_default(),
        file_path: fichier.to_string(),
        kind: "write",
        occurred_at: Utc::now(),
    }
}

/// FR-009 : une ecriture a l'emplacement d'une issue vivante la passe en
/// « en cours » sans intervention.
#[tokio::test]
async fn une_ecriture_passe_lissue_en_doing() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Le tunnel de paiement", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue = ctx.creer_issue(&bloc_id, "Ecran de paiement", Some("web/app/checkout/paiement")).await;
    let issue_id = issue["id"].as_str().unwrap().to_string();

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("web/app/checkout/paiement/page.tsx")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let relue = ctx.lire_issue(&issue_id).await;
    assert_eq!(relue["statut"], json!("doing"), "l'ecriture doit entamer l'issue");
}

/// FR-010 : ecrire dans un dossier n'est pas reprendre un travail livre. Une
/// issue terminee reste terminee, quelle que soit l'activite a son
/// emplacement.
#[tokio::test]
async fn elle_ne_rouvre_pas_une_issue_terminee() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Deja livre", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue = ctx.creer_issue(&bloc_id, "Ecran termine", Some("web/app/checkout/confirmation")).await;
    let issue_id = issue["id"].as_str().unwrap().to_string();

    ctx.poser_statut_issue(&issue_id, "done").await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("web/app/checkout/confirmation/page.tsx")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let relue = ctx.lire_issue(&issue_id).await;
    assert_eq!(relue["statut"], json!("done"), "une ecriture ne rouvre jamais un travail termine");
}

/// FR-011 : quand plusieurs travaux vivants partagent un emplacement, tous
/// passent en cours - pas seulement le premier trouve.
#[tokio::test]
async fn six_issues_du_meme_dossier_passent_toutes_en_doing() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Six chantiers", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let mut issue_ids = Vec::new();
    for i in 0..6 {
        let issue = ctx
            .creer_issue(&bloc_id, &format!("Morceau {i}"), Some("web/app/checkout"))
            .await;
        issue_ids.push(issue["id"].as_str().unwrap().to_string());
    }

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("web/app/checkout/panier.tsx")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    for id in &issue_ids {
        let relue = ctx.lire_issue(id).await;
        assert_eq!(relue["statut"], json!("doing"), "toutes les six doivent passer en cours");
    }
}

/// Critere central : entre une issue ancree a `web/` et une autre a
/// `web/app/hero`, une ecriture dans `web/app/hero/page.tsx` route la plus
/// profonde, pas la plus large.
#[tokio::test]
async fn le_plus_profond_gagne() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc_large = ctx.creer_bloc(&repo_id, "Tout le web", "technique", "web").await;
    let bloc_large_id = bloc_large["id"].as_str().unwrap().to_string();
    let issue_large = ctx.creer_issue(&bloc_large_id, "Large", Some("web")).await;
    let issue_large_id = issue_large["id"].as_str().unwrap().to_string();

    let bloc_profond = ctx.creer_bloc(&repo_id, "Le hero", "feature", "web/app/hero").await;
    let bloc_profond_id = bloc_profond["id"].as_str().unwrap().to_string();
    let issue_profonde = ctx.creer_issue(&bloc_profond_id, "Profonde", Some("web/app/hero")).await;
    let issue_profonde_id = issue_profonde["id"].as_str().unwrap().to_string();

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("web/app/hero/page.tsx")])
        .await
        .expect("l'evenement doit etre accepte");

    assert_eq!(
        ctx.lire_issue(&issue_profonde_id).await["statut"],
        json!("doing"),
        "la plus profonde doit etre entamee"
    );
    assert_eq!(
        ctx.lire_issue(&issue_large_id).await["statut"],
        json!("todo"),
        "la plus large ne doit pas bouger quand une plus profonde existe"
    );
}

/// Le piege classique du prefixe de segment : `web/app` ne doit jamais
/// preffixer `web/application/x.ts`, meme si la chaine `web/app` est bien un
/// prefixe de caracteres de `web/application`. Sans frontiere de segment
/// (`/` ou fin de chaine), cette issue s'entamerait a tort.
#[tokio::test]
async fn un_faux_prefixe_de_segment_ne_route_rien() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "L'app web", "feature", "web/app").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue = ctx.creer_issue(&bloc_id, "Le dossier app", Some("web/app")).await;
    let issue_id = issue["id"].as_str().unwrap().to_string();

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("web/application/x.ts")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let relue = ctx.lire_issue(&issue_id).await;
    assert_eq!(
        relue["statut"],
        json!("todo"),
        "« web/app » ne prefixe pas « web/application/x.ts », rien ne doit bouger"
    );
}

/// Un chemin vide designe la racine du depot : il prefixe tout fichier,
/// meme en l'absence de tout autre candidat plus profond.
///
/// Ni `creer_bloc` ni `creer_issue` ne laissent jamais poser un chemin vide
/// en pratique (un bloc simple exige un emplacement non vide, et la
/// premiere issue d'un bloc herite toujours du chemin - non vide lui aussi -
/// de son bloc, §4.3 de #30) : le cas n'est donc atteignable, aujourd'hui,
/// que par une insertion directe. Le trigger doit neanmoins s'en accommoder,
/// puisque la regle est du PRD, pas une simple consequence du chemin normal
/// d'ecriture.
#[tokio::test]
async fn un_chemin_vide_prefixe_tout() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Chantier racine", "technique", "daemon/src").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    // Decoupe le bloc une premiere fois : la seconde issue, elle, n'est plus
    // soumise a l'heritage force du chemin du bloc par `bloc_coherent()`.
    ctx.creer_issue(&bloc_id, "Premiere, pour decouper", Some("daemon/src/journal.rs")).await;
    let racine = ctx
        .tenter_inserer_issue_brute(&repo_id, &bloc_id, 424_242, "")
        .await
        .expect("une insertion brute avec un chemin vide doit passer le schema");
    let issue_id = racine[0]["id"].as_str().unwrap().to_string();

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("README.md")])
        .await
        .expect("l'evenement doit etre accepte");

    let relue = ctx.lire_issue(&issue_id).await;
    assert_eq!(relue["statut"], json!("doing"), "la racine doit couvrir n'importe quel fichier");
}

/// Les blocs simples sont routables comme les issues : ils portent un
/// emplacement (§5.4 de la conception, prompt de la tache).
#[tokio::test]
async fn un_bloc_simple_est_route_comme_une_issue() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Correction ponctuelle", "correction", "daemon/src").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("daemon/src/journal.rs")])
        .await
        .expect("l'evenement doit etre accepte");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["statut"], json!("doing"), "un bloc simple doit s'entamer comme une issue");
}

/// La derivation de l'etat du bloc parent doit suivre : entamer une issue
/// unique d'un bloc decoupe met le bloc en « en cours » via le trigger
/// existant (#30) - sans que ce chantier n'ait besoin de le reecrire.
#[tokio::test]
async fn entamer_une_issue_derive_son_bloc_parent_en_cours() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Bloc a deux issues", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    ctx.creer_issue(&bloc_id, "Premiere", None).await;
    ctx.creer_issue(&bloc_id, "Seconde", Some("web/app/checkout/autre")).await;

    let avant = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(avant["statut"], json!("todo"));

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("web/app/checkout/panier.tsx")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let apres = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres["statut"], json!("doing"), "le bloc decoupe doit deriver en cours");
}

/// Etancheite entre depots : une ecriture dans le depot A ne doit jamais
/// entamer un travail du depot B, meme quand les deux partagent exactement
/// le meme emplacement en texte. Le routage se scope par `repo_id`, jamais
/// par la seule egalite de chemin.
#[tokio::test]
async fn une_ecriture_dans_un_depot_nentame_rien_dans_un_autre() {
    let ctx = common::TestContext::new().await;
    let (client_a, machine_a, repo_a) = repo_de_test(&ctx).await;
    let machine_b = ctx.create_machine("Autre machine").await;
    let repo_b = ctx.creer_repo(&machine_b, &["web/app/checkout"]).await;

    let bloc_b = ctx.creer_bloc(&repo_b, "Meme chemin, autre depot", "feature", "web/app/checkout").await;
    let bloc_b_id = bloc_b["id"].as_str().unwrap().to_string();

    client_a
        .pousser_activite(
            &machine_a,
            &repo_a,
            Some("main"),
            &[ecriture("web/app/checkout/panier.tsx")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let relu = ctx.lire_bloc(&bloc_b_id).await;
    assert_eq!(relu["statut"], json!("todo"), "un autre depot ne doit jamais bouger");
}

/// Une lecture ne route rien : seule une ecriture entame (FR-009 parle
/// explicitement d'ecriture, la table distingue `read` et `write`).
#[tokio::test]
async fn une_lecture_nentame_rien() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Pas touche par une lecture", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    let mut lecture = ecriture("web/app/checkout/panier.tsx");
    lecture.kind = "read";

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[lecture])
        .await
        .expect("l'evenement doit etre accepte");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["statut"], json!("todo"), "une lecture ne doit jamais entamer un travail");
}
