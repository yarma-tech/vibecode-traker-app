//! L'activite arrive en base, et la vue en tire l'etat de chaque module.

mod common;

use chrono::{Duration, Utc};
use vibemap::Activite;

/// Appaire une machine et rend son jeton.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

/// Un identifiant de session unique par test.
///
/// `sessions.id` est une cle primaire globale : deux tests paralleles qui
/// reutiliseraient « s-1 » se disputeraient la meme ligne, et le second serait
/// refuse par la RLS puisque la ligne appartient a l'utilisateur du premier.
fn session() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn evenement(
    session: &str,
    id: &str,
    module: &str,
    fichier: &str,
    kind: &'static str,
) -> Activite {
    Activite {
        session_id: session.to_string(),
        tool_use_id: id.to_string(),
        module_path: module.to_string(),
        file_path: fichier.to_string(),
        kind,
        occurred_at: Utc::now(),
    }
}

/// L'etat d'un module tel que la base le calcule, ou « inactif » s'il n'y en a pas.
async fn etat(ctx: &common::TestContext, repo_id: &str, module: &str) -> String {
    ctx.etat_modules(repo_id, 600)
        .await
        .into_iter()
        .find(|ligne| ligne["module_path"] == module)
        .and_then(|ligne| ligne["etat"].as_str().map(str::to_string))
        .unwrap_or_else(|| "inactif".to_string())
}

#[tokio::test]
async fn une_ecriture_arrive_en_base() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src", "src/core"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    let poses = client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[evenement(&s1, "toolu_1", "src/core", "src/core/auth.ts", "write")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    assert_eq!(poses, 1);

    let evenements = ctx.lire_evenements(&repo_id).await;
    assert_eq!(evenements.len(), 1);
    assert_eq!(evenements[0]["file_path"], "src/core/auth.ts");
    assert_eq!(evenements[0]["kind"], "write");
}

#[tokio::test]
async fn poster_deux_fois_le_meme_appel_ne_cree_qu_une_ligne() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    // Le hook poste, puis le lecteur de journaux repasse sur la meme ligne.
    let meme = [evenement(&s1, "toolu_double", "src", "src/a.ts", "write")];

    let premier = client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &meme)
        .await
        .expect("premier envoi");
    let second = client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &meme)
        .await
        .expect("second envoi");

    assert_eq!(premier, 1);
    assert_eq!(second, 0, "le doublon est absorbe, pas refuse");
    assert_eq!(ctx.lire_evenements(&repo_id).await.len(), 1);
}

#[tokio::test]
async fn une_lecture_seule_donne_l_etat_lu() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&s1, "toolu_1", "src", "src/a.ts", "read"),
                evenement(&s1, "toolu_2", "src", "src/b.ts", "read"),
            ],
        )
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src").await, "lu");
}

#[tokio::test]
async fn une_ecriture_donne_l_etat_ecrit() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&s1, "toolu_1", "src", "src/a.ts", "read"),
                evenement(&s1, "toolu_2", "src", "src/a.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    assert_eq!(
        etat(&ctx, &repo_id, "src").await,
        "ecrit",
        "une ecriture l'emporte sur les lectures"
    );
}

#[tokio::test]
async fn un_evenement_hors_fenetre_laisse_le_module_inactif() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    let mut vieux = evenement(&s1, "toolu_1", "src", "src/a.ts", "write");
    vieux.occurred_at = Utc::now() - Duration::hours(2);

    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &[vieux])
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src").await, "inactif");
}

#[tokio::test]
async fn deux_sessions_qui_ecrivent_dans_le_meme_sous_arbre_donnent_un_conflit() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &["src", "src/core", "src/core/auth", "src/core/db"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s_a = session();
    let s_c = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&s_a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "write"),
                evenement(&s_c, "toolu_2", "src/core/db", "src/core/db/pool.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    assert_eq!(
        etat(&ctx, &repo_id, "src/core").await,
        "conflit",
        "deux agents dans le meme sous-arbre : le parent rougit"
    );
    assert_eq!(
        etat(&ctx, &repo_id, "src/core/auth").await,
        "ecrit",
        "chaque sous-dossier n'a qu'un seul agent : pas de conflit chez lui"
    );
}

#[tokio::test]
async fn une_seule_session_dans_deux_sous_dossiers_ne_fait_pas_conflit() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &["src", "src/core", "src/core/auth", "src/core/db"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s_a = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&s_a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "write"),
                evenement(&s_a, "toolu_2", "src/core/db", "src/core/db/pool.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src/core").await, "ecrit");
}

#[tokio::test]
async fn deux_lectures_de_sessions_differentes_ne_font_jamais_conflit() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src", "src/core"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s_a = session();
    let s_c = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&s_a, "toolu_1", "src/core", "src/core/a.ts", "read"),
                evenement(&s_c, "toolu_2", "src/core", "src/core/b.ts", "read"),
            ],
        )
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src/core").await, "lu");
}

#[tokio::test]
async fn l_activite_remonte_au_dossier_parent() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src", "src/core"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[evenement(&s1, "toolu_1", "src/core", "src/core/a.ts", "write")],
        )
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src").await, "ecrit");
}

/// Le voisin ne s'allume pas : sans ce test, un prefixe mal ecrit passerait.
#[tokio::test]
async fn un_dossier_voisin_reste_inactif() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src", "src2"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[evenement(&s1, "toolu_1", "src", "src/a.ts", "write")],
        )
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src2").await, "inactif");
}

#[tokio::test]
async fn une_machine_revoquee_ne_peut_plus_ecrire_d_activite() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    ctx.revoquer(&machine.machine_id).await;

    let issue = client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[evenement(&s1, "toolu_1", "src", "src/a.ts", "write")],
        )
        .await;

    assert!(issue.is_err(), "une machine revoquee n'ecrit plus rien");
    assert!(ctx.lire_evenements(&repo_id).await.is_empty());
}

/// Le hook n'a pas la carte du daemon en memoire : il doit retrouver le repo
/// tout seul, a partir de l'empreinte de sa racine.
#[tokio::test]
async fn le_repo_se_retrouve_par_l_empreinte_de_sa_racine() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let empreinte = vibemap::empreinte(std::path::Path::new("/Users/moi/Developer/atelier"));
    let repo_id = ctx
        .creer_repo_avec_empreinte(&machine.machine_id, &empreinte, &["src"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let trouve = client
        .repo_par_empreinte(&machine.machine_id, &empreinte)
        .await
        .expect("la lecture doit aboutir");

    assert_eq!(trouve.as_deref(), Some(repo_id.as_str()));
}

#[tokio::test]
async fn une_racine_jamais_cartographiee_ne_rend_aucun_repo() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let trouve = client
        .repo_par_empreinte(&machine.machine_id, "une-empreinte-inconnue")
        .await
        .expect("la lecture doit aboutir");

    assert_eq!(trouve, None);
}

#[tokio::test]
async fn la_session_retient_son_debut_et_son_dernier_evenement() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    let mut tot = evenement(&s1, "toolu_1", "src", "src/a.ts", "read");
    tot.occurred_at = Utc::now() - Duration::minutes(5);
    let tard = evenement(&s1, "toolu_2", "src", "src/a.ts", "write");

    // Deux envois separes : le second ne doit pas ecraser le debut du premier.
    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), std::slice::from_ref(&tot))
        .await
        .expect("premier envoi");
    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), std::slice::from_ref(&tard))
        .await
        .expect("second envoi");

    let session = ctx.lire_session(&s1).await;
    assert_eq!(session["branch"], "main");
    let debut = session["started_at"]
        .as_str()
        .expect("started_at")
        .parse::<chrono::DateTime<Utc>>()
        .expect("date de debut");
    let dernier = session["last_event_at"]
        .as_str()
        .expect("last_event_at")
        .parse::<chrono::DateTime<Utc>>()
        .expect("date du dernier evenement");

    assert!(
        (debut - tot.occurred_at).num_seconds().abs() < 2,
        "le debut reste celui du premier evenement, pas du dernier envoi"
    );
    assert!((dernier - tard.occurred_at).num_seconds().abs() < 2);
}
