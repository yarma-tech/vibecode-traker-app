//! Rejouer sans dupliquer, et une purge qui tourne toute seule.
//!
//! Ces tests parlent a une vraie pile Supabase locale : l'idempotence des jetons
//! et la planification de la purge vivent en SQL, et seul un vrai appel le
//! prouve. Ils ne tournent donc pas en CI (voir le README).

mod common;

use chrono::{TimeZone, Utc};
use vibemap::{Activite, SessionCout};

async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

fn session() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Une consommation de session portant une cle d'idempotence.
fn conso_avec_cle(session: &str, cle: &str, input: i64, output: i64) -> SessionCout {
    SessionCout {
        session_id: session.to_string(),
        model: "claude-opus-4-8".to_string(),
        input,
        output,
        cache_read: 0,
        cache_creation: 0,
        debut: Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap(),
        fin: Utc.with_ymd_and_hms(2026, 8, 4, 12, 30, 0).unwrap(),
        cle_usage: Some(cle.to_string()),
    }
}

fn activite(session: &str, tool_use_id: &str) -> Activite {
    Activite {
        session_id: session.to_string(),
        tool_use_id: tool_use_id.to_string(),
        module_path: "src".to_string(),
        file_path: "src/a.ts".to_string(),
        kind: "write",
        occurred_at: Utc.with_ymd_and_hms(2026, 8, 4, 12, 0, 0).unwrap(),
    }
}

#[tokio::test]
async fn rejouer_le_meme_lot_de_cout_ne_double_pas_les_jetons() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    // Le meme lot, envoye trois fois : un redemarrage avant persistance de la
    // position de lecture, ou un fichier de journal rejoue entierement.
    for _ in 0..3 {
        client
            .pousser_cout(
                &machine.machine_id,
                &repo_id,
                Some("main"),
                &[conso_avec_cle(&s1, "cle-lot-1", 1000, 2000)],
            )
            .await
            .expect("la consommation doit etre acceptee");
    }

    let s = ctx.lire_session(&s1).await;
    assert_eq!(s["input_tokens"].as_i64(), Some(1000), "le rejeu ne double pas les jetons");
    assert_eq!(s["output_tokens"].as_i64(), Some(2000));

    // Un lot vraiment nouveau (autre cle) s'ajoute bien : l'idempotence ne gele
    // pas la session, elle ne fait que refuser les doublons.
    client
        .pousser_cout(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[conso_avec_cle(&s1, "cle-lot-2", 30, 40)],
        )
        .await
        .expect("un nouveau lot doit etre accepte");

    let s = ctx.lire_session(&s1).await;
    assert_eq!(s["input_tokens"].as_i64(), Some(1030), "un lot neuf s'ajoute");
    assert_eq!(s["output_tokens"].as_i64(), Some(2040));
}

#[tokio::test]
async fn rejouer_entierement_les_evenements_ne_cree_aucun_doublon() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    let evenements = [activite(&s1, "toolu_a"), activite(&s1, "toolu_b")];

    // Deux passes completes sur le meme fichier de journal.
    for _ in 0..2 {
        client
            .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &evenements)
            .await
            .expect("les evenements doivent etre acceptes");
    }

    let poses = ctx.lire_evenements(&repo_id).await;
    assert_eq!(poses.len(), 2, "la contrainte d'unicite absorbe le rejeu : pas de doublon");
}

#[tokio::test]
async fn la_purge_des_evenements_est_planifiee() {
    let ctx = common::TestContext::new().await;
    assert!(
        ctx.purge_planifiee().await,
        "la purge des evenements a plus de sept jours doit tourner toute seule (pg_cron)"
    );
}
