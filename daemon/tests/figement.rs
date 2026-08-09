//! Machine injoignable, état gelé (issue #11).
//!
//! Au-delà de 90 s sans battement de cœur, la machine est déclarée injoignable.
//! L'écran garde le dernier état connu, mais désaturé et daté : il ne ment pas.
//!
//! La bascule « figé » se décide côté écran, à partir de l'âge du dernier
//! battement. La base, elle, doit porter cet âge jusqu'à l'écran : l'aperçu de
//! l'accueil rend, pour chaque repo, la dernière présence de sa machine. Sans
//! quoi le rail ne saurait pas signaler une machine muette. Ces tests éprouvent
//! ce canal de bout en bout : Rust, HTTP, PostgREST, la RLS et Postgres.

mod common;

async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

const ARBRE: &[&str] = &["src", "src/core", "docs"];

/// Avant le premier battement, une machine n'a pas de présence : l'aperçu rend
/// donc `null`, jamais une fausse date. « Jamais vue » n'est pas « vue à
/// l'instant ».
#[tokio::test]
async fn l_apercu_ignore_la_presence_avant_le_premier_battement() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo = ctx.creer_repo(&machine.machine_id, ARBRE).await;

    let apercu = ctx.apercu_repos(600).await;
    let ligne = apercu
        .iter()
        .find(|r| r["id"].as_str() == Some(repo.as_str()))
        .expect("le repo doit apparaître dans l'aperçu");

    assert!(
        ligne["derniere_presence"].is_null(),
        "sans battement, la présence est nulle, pas une date inventée : {ligne}"
    );
}

/// L'aperçu porte l'heure du dernier battement de la machine du repo. C'est
/// cette heure que le rail compare à maintenant pour déclarer, ou non, la
/// machine muette et dire depuis combien de temps.
#[tokio::test]
async fn l_apercu_porte_l_heure_du_dernier_battement() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo = ctx.creer_repo(&machine.machine_id, ARBRE).await;

    let battement = "2020-01-01T00:00:00Z";
    ctx.poser_derniere_presence(&machine.machine_id, battement).await;

    let apercu = ctx.apercu_repos(600).await;
    let ligne = apercu
        .iter()
        .find(|r| r["id"].as_str() == Some(repo.as_str()))
        .expect("le repo doit apparaître dans l'aperçu");

    let vue = ligne["derniere_presence"]
        .as_str()
        .expect("la dernière présence doit être renseignée après un battement")
        .parse::<chrono::DateTime<chrono::Utc>>()
        .expect("la dernière présence doit être une date valide");

    let attendu = battement.parse::<chrono::DateTime<chrono::Utc>>().unwrap();
    assert_eq!(
        vue, attendu,
        "l'aperçu doit rendre l'heure exacte du dernier battement"
    );
}

/// Une machine à jour et une machine muette coexistent sans confusion : chaque
/// repo porte la présence de SA machine, jamais celle d'une autre. C'est ce qui
/// permet au rail de peindre l'une vivante et l'autre figée côte à côte.
#[tokio::test]
async fn chaque_repo_porte_la_presence_de_sa_propre_machine() {
    let ctx = common::TestContext::new().await;
    let vivante = machine_reliee(&ctx).await;

    // Une seconde machine du même utilisateur, reliée par un second appairage.
    let code = ctx.creer_code().await;
    let muette = vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "Mac mini", Some("darwin"))
        .await
        .expect("appairage de la seconde machine");

    let repo_vivant = ctx.creer_repo(&vivante.machine_id, ARBRE).await;
    let repo_muet = ctx.creer_repo(&muette.machine_id, ARBRE).await;

    let recent = chrono::Utc::now().to_rfc3339();
    ctx.poser_derniere_presence(&vivante.machine_id, &recent).await;
    ctx.poser_derniere_presence(&muette.machine_id, "2020-01-01T00:00:00Z")
        .await;

    let apercu = ctx.apercu_repos(600).await;
    let presence = |repo_id: &str| -> Option<chrono::DateTime<chrono::Utc>> {
        apercu
            .iter()
            .find(|r| r["id"].as_str() == Some(repo_id))
            .and_then(|r| r["derniere_presence"].as_str())
            .map(|s| s.parse().expect("date valide"))
    };

    let vue_vivant = presence(&repo_vivant).expect("présence du repo vivant");
    let vue_muet = presence(&repo_muet).expect("présence du repo muet");

    assert!(
        vue_vivant > vue_muet,
        "chaque repo porte sa propre présence : la vivante est récente, la muette est ancienne"
    );
    assert_eq!(
        vue_muet,
        "2020-01-01T00:00:00Z"
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
        "la présence de la machine muette ne doit pas être contaminée par la vivante"
    );
}
