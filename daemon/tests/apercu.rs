//! L'apercu de l'accueil : tous les repos d'un coup, tries par activite (issue #8).
//!
//! L'ecran d'accueil lit une seule fonction qui rend, pour chaque repo, de quoi
//! peindre sa bande d'etat et deduire son badge de compte. Le tri place en haut
//! ce qui demande une decision : un conflit passe devant une simple ecriture,
//! qui passe devant une lecture, qui passe devant le silence.
//!
//! Le badge @perso / @pro se deduit du proprietaire du remote git, croise a une
//! correspondance reglee une fois par l'utilisateur. Aucun appel a l'API GitHub :
//! ce test tourne hors reseau et n'interroge que la base.

mod common;

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

fn evenement(session: &str, id: &str, module: &str, fichier: &str, kind: &'static str) -> Activite {
    Activite {
        session_id: session.to_string(),
        tool_use_id: id.to_string(),
        module_path: module.to_string(),
        file_path: fichier.to_string(),
        kind,
        occurred_at: chrono::Utc::now(),
    }
}

const ARBRE: &[&str] = &["src", "src/core", "src/core/auth", "src/core/db", "docs"];

/// L'apercu place en tete ce qui demande une decision. Trois repos : un en
/// conflit, un en simple ecriture, un muet. L'ordre rendu doit etre exactement
/// celui-la, sans que l'ecran ait a re-trier.
#[tokio::test]
async fn le_tri_place_le_conflit_en_tete() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let muet = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let ecrit = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let decisif = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let (a, b) = (session(), session());

    client
        .pousser_activite(
            &machine.machine_id,
            &ecrit,
            Some("main"),
            &[evenement(&a, "toolu_1", "src/core", "src/core/a.ts", "write")],
        )
        .await
        .expect("envoi ecrit");

    client
        .pousser_activite(
            &machine.machine_id,
            &decisif,
            Some("main"),
            &[
                evenement(&a, "toolu_2", "src/core/auth", "src/core/auth/j.ts", "write"),
                evenement(&b, "toolu_3", "src/core/db", "src/core/db/p.ts", "write"),
            ],
        )
        .await
        .expect("envoi decisif");

    let apercu = ctx.apercu_repos(600).await;
    let ordre: Vec<&str> = apercu
        .iter()
        .map(|r| r["id"].as_str().unwrap_or_default())
        .collect();

    assert_eq!(
        ordre,
        vec![decisif.as_str(), ecrit.as_str(), muet.as_str()],
        "le conflit passe devant l'ecriture, qui passe devant le silence"
    );

    let tete = &apercu[0];
    assert_eq!(tete["etat"], "conflit", "la bande d'etat du repo en tete est rouge");
    assert_eq!(tete["conflits"], 1, "un seul point de rencontre");
}

/// Le badge de compte se deduit du proprietaire du remote, croise a la
/// correspondance reglee par l'utilisateur. Sans correspondance, pas de badge.
#[tokio::test]
async fn le_compte_se_deduit_du_proprietaire_du_remote() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let pro = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let perso = ctx.creer_repo(&machine.machine_id, ARBRE).await;
    let sans_compte = ctx.creer_repo(&machine.machine_id, ARBRE).await;

    ctx.poser_remote_owner(&pro, "acme-corp").await;
    ctx.poser_remote_owner(&perso, "moi-perso").await;
    ctx.poser_remote_owner(&sans_compte, "inconnu-au-bataillon").await;

    // La correspondance se regle une fois : owner -> perso / pro.
    ctx.regler_compte("acme-corp", "pro").await;
    ctx.regler_compte("moi-perso", "perso").await;

    let apercu = ctx.apercu_repos(600).await;
    let compte = |repo_id: &str| -> Option<String> {
        apercu
            .iter()
            .find(|r| r["id"].as_str() == Some(repo_id))
            .and_then(|r| r["compte"].as_str().map(str::to_string))
    };

    assert_eq!(compte(&pro).as_deref(), Some("pro"));
    assert_eq!(compte(&perso).as_deref(), Some("perso"));
    assert_eq!(
        compte(&sans_compte),
        None,
        "un proprietaire non classe ne porte aucun badge, il ne se devine pas"
    );
}

/// Deux utilisateurs peuvent classer le meme proprietaire differemment : la
/// correspondance appartient a chacun, jamais partagee. Ici on verifie qu'un
/// repo ne remonte que le badge de son propre proprietaire.
#[tokio::test]
async fn l_apercu_ne_montre_que_les_repos_de_l_appelant() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let mien = ctx.creer_repo(&machine.machine_id, ARBRE).await;

    let apercu = ctx.apercu_repos(600).await;

    assert!(
        apercu.iter().all(|r| r["id"].as_str() == Some(mien.as_str())),
        "l'apercu ne rend que les repos de l'utilisateur connecte, la RLS s'en charge"
    );
    assert_eq!(apercu.len(), 1);
}
