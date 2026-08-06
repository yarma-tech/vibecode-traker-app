//! Le conflit : deux sessions qui ecrivent dans le meme sous-arbre.
//!
//! C'est la seule alarme du produit. Elle doit sonner quand il faut, et se
//! taire le reste du temps : un rouge qui ment est pire qu'un rouge absent.

mod common;

use chrono::{Duration, Utc};
use vibemap::Activite;

async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

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

/// Les quatre dossiers du cas de la spec : deux agents sous `src/core`.
const ARBRE: &[&str] = &["src", "src/core", "src/core/auth", "src/core/db", "docs"];

async fn etat(ctx: &common::TestContext, repo_id: &str, module: &str) -> String {
    ctx.etat_modules(repo_id, 600)
        .await
        .into_iter()
        .find(|ligne| ligne["module_path"] == module)
        .and_then(|ligne| ligne["etat"].as_str().map(str::to_string))
        .unwrap_or_else(|| "inactif".to_string())
}

#[tokio::test]
async fn le_conflit_se_signale_au_dossier_le_plus_profond_qui_le_contient() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "write"),
                evenement(&c, "toolu_2", "src/core/db", "src/core/db/pool.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    // `src` herite lui aussi du conflit : la propagation monte sur deux niveaux.
    assert_eq!(etat(&ctx, &repo_id, "src/core").await, "conflit");
    assert_eq!(etat(&ctx, &repo_id, "src").await, "conflit");

    // Mais on ne compte qu'un seul conflit, au point ou les deux agents se
    // rencontrent. Sinon un conflit unique en vaudrait autant que sa profondeur.
    let conflits = ctx.conflits(&repo_id, 600).await;
    assert_eq!(conflits.len(), 1);
    assert_eq!(conflits[0]["module_path"], "src/core");
    assert_eq!(conflits[0]["sessions_ecrivant"], 2);
}

#[tokio::test]
async fn le_conflit_nomme_les_sessions_en_cause() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "write"),
                evenement(&c, "toolu_2", "src/core/db", "src/core/db/pool.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    let conflits = ctx.conflits(&repo_id, 600).await;
    let nommees: Vec<String> = conflits[0]["sessions"]
        .as_array()
        .expect("la liste des sessions en cause")
        .iter()
        .map(|s| s.as_str().unwrap_or_default().to_string())
        .collect();

    assert_eq!(nommees.len(), 2, "le journal doit pouvoir nommer les deux agents");
    assert!(nommees.contains(&a));
    assert!(nommees.contains(&c));
}

#[tokio::test]
async fn deux_sessions_dans_deux_sous_arbres_differents_ne_declenchent_rien() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&a, "toolu_1", "src/core", "src/core/a.ts", "write"),
                evenement(&c, "toolu_2", "docs", "docs/b.md", "write"),
            ],
        )
        .await
        .expect("envoi");

    assert_eq!(etat(&ctx, &repo_id, "src/core").await, "ecrit");
    assert_eq!(etat(&ctx, &repo_id, "docs").await, "ecrit");
    assert!(
        ctx.conflits(&repo_id, 600).await.is_empty(),
        "deux agents qui ne se croisent pas ne sont pas un conflit"
    );
}

#[tokio::test]
async fn une_seule_session_ne_declenche_jamais_de_conflit() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let a = session();

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "write"),
                evenement(&a, "toolu_2", "src/core/db", "src/core/db/pool.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    assert!(ctx.conflits(&repo_id, 600).await.is_empty());
}

#[tokio::test]
async fn deux_lectures_ne_declenchent_jamais_de_conflit() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[
                evenement(&a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "read"),
                evenement(&c, "toolu_2", "src/core/db", "src/core/db/pool.ts", "read"),
                // Une ecriture d'un seul des deux ne suffit pas non plus.
                evenement(&a, "toolu_3", "src/core/auth", "src/core/auth/jeton.ts", "write"),
            ],
        )
        .await
        .expect("envoi");

    assert!(ctx.conflits(&repo_id, 600).await.is_empty());
}

#[tokio::test]
async fn le_rouge_disparait_quand_la_fenetre_s_ecoule() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    let mut premier = evenement(&a, "toolu_1", "src/core/auth", "src/core/auth/jeton.ts", "write");
    premier.occurred_at = Utc::now() - Duration::minutes(9);
    let mut second = evenement(&c, "toolu_2", "src/core/db", "src/core/db/pool.ts", "write");
    second.occurred_at = Utc::now() - Duration::minutes(9);

    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &[premier, second])
        .await
        .expect("envoi");

    assert_eq!(
        ctx.conflits(&repo_id, 600).await.len(),
        1,
        "dans la fenetre de dix minutes, le conflit tient"
    );
    assert!(
        ctx.conflits(&repo_id, 480).await.is_empty(),
        "passe la fenetre sans nouvelle ecriture, le rouge s'eteint"
    );
    assert_eq!(etat(&ctx, &repo_id, "src/core").await, "conflit");
}

/// Le bandeau compte les agents presents. Comme la fenetre, ce compte se fait
/// dans la base : l'ecran n'a pas a savoir manier une horloge.
#[tokio::test]
async fn les_agents_presents_se_comptent_sur_la_fenetre() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    let mut vieux = evenement(&a, "toolu_1", "src", "src/a.ts", "write");
    vieux.occurred_at = Utc::now() - Duration::minutes(30);

    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &[vieux])
        .await
        .expect("envoi ancien");
    client
        .pousser_activite(
            &machine.machine_id,
            &repo_id,
            Some("main"),
            &[evenement(&c, "toolu_2", "docs", "docs/b.md", "read")],
        )
        .await
        .expect("envoi recent");

    assert_eq!(
        ctx.agents_actifs(&repo_id, 600).await,
        1,
        "l'agent parti il y a une demi-heure n'est plus la"
    );
    assert_eq!(ctx.agents_actifs(&repo_id, 3600).await, 2);
}

/// L'accueil liste les conflits de tous les repos d'un coup : sans quoi il
/// faudrait une requete par repo, et le compte du rail pourrait diverger de
/// celui du plan.
#[tokio::test]
async fn les_conflits_de_tous_les_repos_se_lisent_ensemble() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let paisible = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let agite = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, c) = (session(), session());

    client
        .pousser_activite(
            &machine.machine_id,
            &paisible,
            Some("main"),
            &[evenement(&a, "toolu_1", "src/core", "src/core/a.ts", "write")],
        )
        .await
        .expect("envoi paisible");

    client
        .pousser_activite(
            &machine.machine_id,
            &agite,
            Some("main"),
            &[
                evenement(&a, "toolu_2", "src/core/auth", "src/core/auth/jeton.ts", "write"),
                evenement(&c, "toolu_3", "src/core/db", "src/core/db/pool.ts", "write"),
            ],
        )
        .await
        .expect("envoi agite");

    let tous = ctx.conflits_de_la_machine(&machine.machine_id, 600).await;

    assert_eq!(tous.len(), 1, "un seul repo est en conflit");
    assert_eq!(tous[0]["repo_id"], agite);
    assert_eq!(tous[0]["module_path"], "src/core");
}
