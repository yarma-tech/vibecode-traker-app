//! La consommation des sessions arrive en base, et le releve en tire le cout.
//!
//! Comme l'activite, ces tests parlent a une vraie pile Supabase locale : le
//! calcul du cout vit en SQL, et seul un vrai appel verifie qu'il tient.

mod common;

use chrono::{TimeZone, Utc};
use vibemap::SessionCout;

/// Appaire une machine et rend son jeton.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

fn session() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Une consommation de session, bornée dans le temps.
fn conso(
    session: &str,
    model: &str,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_creation: i64,
) -> SessionCout {
    SessionCout {
        session_id: session.to_string(),
        model: model.to_string(),
        input,
        output,
        cache_read,
        cache_creation,
        debut: Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap(),
        fin: Utc.with_ymd_and_hms(2026, 8, 4, 12, 30, 0).unwrap(),
    }
}

#[tokio::test]
async fn le_cout_suit_le_calcul_manuel_cache_compris() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_cout(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[conso(&s1, "claude-opus-4-8", 1000, 2000, 100_000, 5000)],
        )
        .await
        .expect("la consommation doit etre acceptee");

    let releve = ctx.releve_repo(&repo_id).await;
    assert_eq!(releve["input_tokens"].as_i64(), Some(1000));
    assert_eq!(releve["output_tokens"].as_i64(), Some(2000));
    assert_eq!(releve["cache_read_tokens"].as_i64(), Some(100_000));
    assert_eq!(releve["cache_creation_tokens"].as_i64(), Some(5000));

    // Prix Opus par million : entree 15, sortie 75, cache lu 1,50, cache ecrit 18,75.
    // (1000*15 + 2000*75 + 100000*1,50 + 5000*18,75) / 1e6 = 0,40875 $.
    let cout = releve["cout_usd"].as_f64().expect("un cout est attendu");
    assert!((cout - 0.408_75).abs() < 1e-9, "cout inattendu : {cout}");
    assert_eq!(releve["sessions"].as_i64(), Some(1));
    assert_eq!(releve["sessions_tarifees"].as_i64(), Some(1));
}

#[tokio::test]
async fn un_modele_inconnu_montre_les_jetons_sans_cout() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_cout(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[conso(&s1, "un-modele-hors-table", 500, 500, 0, 0)],
        )
        .await
        .expect("la consommation doit etre acceptee");

    let releve = ctx.releve_repo(&repo_id).await;
    assert_eq!(releve["input_tokens"].as_i64(), Some(500));
    assert_eq!(releve["output_tokens"].as_i64(), Some(500));
    // Un modele inconnu : les jetons, mais pas de cout faux.
    assert!(releve["cout_usd"].is_null(), "un modele inconnu ne coute rien de faux");
    assert_eq!(releve["sessions"].as_i64(), Some(1));
    assert_eq!(releve["sessions_tarifees"].as_i64(), Some(0));
}

#[tokio::test]
async fn deux_lots_additionnent_les_jetons_de_la_session() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    for _ in 0..2 {
        client
            .pousser_cout(
                &machine.machine_id,
                &repo_id,
                Some("main"),
                &[conso(&s1, "claude-sonnet-4-5", 10, 20, 30, 40)],
            )
            .await
            .expect("la consommation doit etre acceptee");
    }

    let s = ctx.lire_session(&s1).await;
    assert_eq!(s["input_tokens"].as_i64(), Some(20), "les jetons s'additionnent");
    assert_eq!(s["output_tokens"].as_i64(), Some(40));
    assert_eq!(s["cache_read_tokens"].as_i64(), Some(60));
    assert_eq!(s["cache_creation_tokens"].as_i64(), Some(80));
}

#[tokio::test]
async fn la_duree_vaut_le_dernier_evenement_moins_le_debut() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_cout(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[conso(&s1, "claude-opus-4-8", 1, 1, 0, 0)],
        )
        .await
        .expect("la consommation doit etre acceptee");

    let s = ctx.lire_session(&s1).await;
    let debut = s["started_at"].as_str().unwrap().parse::<chrono::DateTime<Utc>>().unwrap();
    let fin = s["last_event_at"].as_str().unwrap().parse::<chrono::DateTime<Utc>>().unwrap();
    assert_eq!((fin - debut).num_minutes(), 30, "la duree est fin moins debut");
}

#[tokio::test]
async fn les_agregats_survivent_a_la_purge_des_evenements() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_cout(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[conso(&s1, "claude-opus-4-8", 1000, 2000, 0, 0)],
        )
        .await
        .expect("la consommation doit etre acceptee");

    // Un evenement vieux de huit jours, que la purge doit emporter.
    ctx.poser_evenement_ancien(&repo_id, &s1, "2026-07-27T00:00:00Z").await;
    ctx.purger().await;

    // L'evenement est parti, mais les jetons de la session restent.
    assert!(ctx.lire_evenements(&repo_id).await.is_empty(), "la purge a emporte l'evenement");
    let s = ctx.lire_session(&s1).await;
    assert_eq!(s["input_tokens"].as_i64(), Some(1000), "les agregats survivent a la purge");
    assert_eq!(s["output_tokens"].as_i64(), Some(2000));
}
