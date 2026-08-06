//! Appairage d'une machine (issue #9).
//!
//! Le web cree un code, le daemon l'echange contre un jeton qui ne vaut que
//! pour cette machine. Ces tests traversent la fonction SQL, la signature du
//! jeton et la RLS resserree.

mod common;

/// Tracer bullet : un code valide donne un jeton qui sait ecrire.
#[tokio::test]
async fn un_code_valide_donne_un_jeton_qui_ecrit() {
    let ctx = common::TestContext::new().await;
    let code = ctx.creer_code().await;

    let identite = vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("un code frais doit etre accepte");

    assert_eq!(identite.label, "MacBook Pro");

    // Le jeton recu doit suffire a battre, sans rien d'autre.
    let client = vibemap::Supabase::new(&ctx.url, &identite.token);
    client
        .announce(&identite.machine_id, chrono::Utc::now())
        .await
        .expect("le jeton d'appairage doit permettre d'ecrire");

    assert!(
        ctx.last_seen_at(&identite.machine_id).await.is_some(),
        "le battement doit avoir atteint la base"
    );
}

/// Un code inconnu est refuse, et le message dit quoi faire.
#[tokio::test]
async fn un_code_inconnu_est_refuse() {
    let ctx = common::TestContext::new().await;

    let erreur = vibemap::appairer(&ctx.url, &ctx.anon_key, "ZZZ-ZZZZ", "MacBook Pro", None)
        .await
        .expect_err("un code invente ne doit pas passer");

    let message = erreur.to_string();
    assert!(
        message.contains("inconnu"),
        "le message doit dire que le code est inconnu, obtenu : {message}"
    );
}

/// Un code perime est refuse.
#[tokio::test]
async fn un_code_expire_est_refuse() {
    let ctx = common::TestContext::new().await;
    let code = ctx.creer_code().await;
    ctx.perimer_code(&code).await;

    let erreur = vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", None)
        .await
        .expect_err("un code expire ne doit pas passer");

    let message = erreur.to_string();
    assert!(
        message.contains("expire"),
        "le message doit parler d'expiration, obtenu : {message}"
    );
}

/// Un code ne sert qu'une fois.
#[tokio::test]
async fn un_code_ne_sert_qu_une_fois() {
    let ctx = common::TestContext::new().await;
    let code = ctx.creer_code().await;

    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", None)
        .await
        .expect("premier echange accepte");

    let erreur = vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "Mac mini", None)
        .await
        .expect_err("le meme code ne doit pas resservir");

    let message = erreur.to_string();
    assert!(
        message.contains("deja servi"),
        "le message doit dire que le code a deja servi, obtenu : {message}"
    );
}

/// La propriete que l'ancien jeton colle a la main n'avait pas du tout :
/// un jeton de machine ne vaut que pour SA ligne, meme au sein du meme compte.
#[tokio::test]
async fn un_jeton_de_machine_ne_touche_pas_les_autres_machines() {
    let ctx = common::TestContext::new().await;

    let code_a = ctx.creer_code().await;
    let a = vibemap::appairer(&ctx.url, &ctx.anon_key, &code_a, "MacBook Pro", None)
        .await
        .expect("appairage de la premiere machine");

    let code_b = ctx.creer_code().await;
    let b = vibemap::appairer(&ctx.url, &ctx.anon_key, &code_b, "Mac mini", None)
        .await
        .expect("appairage de la seconde machine");

    // Le jeton de A tente de battre pour B.
    let client_a = vibemap::Supabase::new(&ctx.url, &a.token);
    let resultat = client_a.announce(&b.machine_id, chrono::Utc::now()).await;

    assert!(
        resultat.is_err(),
        "le jeton de la machine A ne doit pas pouvoir ecrire la ligne de la machine B"
    );
    assert!(
        ctx.last_seen_at(&b.machine_id).await.is_none(),
        "la machine B ne doit pas avoir bouge"
    );
}

/// Revoquer coupe les ecritures immediatement.
#[tokio::test]
async fn une_machine_revoquee_ne_peut_plus_ecrire() {
    let ctx = common::TestContext::new().await;
    let code = ctx.creer_code().await;
    let machine = vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", None)
        .await
        .expect("appairage");

    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    client
        .announce(&machine.machine_id, chrono::Utc::now())
        .await
        .expect("elle bat normalement avant revocation");

    ctx.revoquer(&machine.machine_id).await;

    let resultat = client.announce(&machine.machine_id, chrono::Utc::now()).await;
    assert!(
        resultat.is_err(),
        "une machine revoquee ne doit plus pouvoir ecrire, meme avec son ancien jeton"
    );
}

/// Revoquer une machine n'affecte pas les autres.
#[tokio::test]
async fn revoquer_une_machine_n_affecte_pas_les_autres() {
    let ctx = common::TestContext::new().await;

    let code_a = ctx.creer_code().await;
    let a = vibemap::appairer(&ctx.url, &ctx.anon_key, &code_a, "MacBook Pro", None)
        .await
        .expect("appairage de A");

    let code_b = ctx.creer_code().await;
    let b = vibemap::appairer(&ctx.url, &ctx.anon_key, &code_b, "Mac mini", None)
        .await
        .expect("appairage de B");

    ctx.revoquer(&a.machine_id).await;

    vibemap::Supabase::new(&ctx.url, &b.token)
        .announce(&b.machine_id, chrono::Utc::now())
        .await
        .expect("la machine B doit continuer a battre apres la revocation de A");
}
