//! Comportement 1 : un battement de coeur arrive jusqu'en base.
//!
//! Ce test est le tracer bullet de l'issue #2 : il traverse Rust, HTTP,
//! PostgREST, la RLS et Postgres d'un seul coup.

mod common;

#[tokio::test]
async fn un_battement_ecrit_last_seen_at() {
    let ctx = common::TestContext::new().await;
    let machine = ctx.create_machine("MacBook Pro").await;

    assert!(
        ctx.last_seen_at(&machine).await.is_none(),
        "une machine fraichement inseree n'a pas encore de dernier signe de vie"
    );

    let client = vibemap::Supabase::new(&ctx.url, &ctx.user_token);
    let instant = chrono::Utc::now();

    client
        .announce(&machine, instant)
        .await
        .expect("le battement doit etre accepte");

    let vu = ctx
        .last_seen_at(&machine)
        .await
        .expect("last_seen_at doit etre renseigne apres un battement");

    assert!(
        (vu - instant).num_milliseconds().abs() < 1000,
        "last_seen_at doit valoir l'instant du battement, ecart constate : {} ms",
        (vu - instant).num_milliseconds()
    );
}

/// Comportement 2 : un jeton invalide echoue franchement.
///
/// Le pire scenario serait un echec silencieux : le daemon croit battre,
/// l'ecran affiche une machine vivante, et personne ne sait que c'est faux.
#[tokio::test]
async fn un_jeton_invalide_donne_une_erreur_explicite() {
    let ctx = common::TestContext::new().await;
    let machine = ctx.create_machine("MacBook Pro").await;

    let client = vibemap::Supabase::new(&ctx.url, "jeton-qui-ne-vaut-rien");
    let erreur = client
        .announce(&machine, chrono::Utc::now())
        .await
        .expect_err("un jeton invalide ne doit pas passer");

    match erreur {
        vibemap::ApiError::Refuse { code, .. } => {
            assert_eq!(code, 401, "un jeton invalide doit donner un 401")
        }
        autre => panic!("erreur attendue de type Refuse, obtenue : {autre}"),
    }

    assert!(
        ctx.last_seen_at(&machine).await.is_none(),
        "un battement refuse ne doit rien avoir ecrit"
    );
}

/// Comportement 3 : la RLS tient entre deux comptes.
///
/// Sans cette garantie, l'isolement des donnees repose sur l'espoir que
/// personne ne devine l'identifiant d'une machine.
#[tokio::test]
async fn le_jeton_d_un_autre_compte_ne_peut_pas_battre() {
    let proprietaire = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;

    let machine = proprietaire.create_machine("MacBook Pro").await;

    let client = vibemap::Supabase::new(&intrus.url, &intrus.user_token);
    let resultat = client.announce(&machine, chrono::Utc::now()).await;

    assert!(
        resultat.is_err(),
        "battre pour la machine d'un autre compte doit echouer, pas reussir en silence"
    );

    assert!(
        proprietaire.last_seen_at(&machine).await.is_none(),
        "la machine du proprietaire ne doit pas avoir bouge"
    );
}
