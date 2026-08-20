//! Un mois de passe depouille, et pas une couleur allumee (issue #79, F9).
//!
//! Ces tests parlent a la vraie pile Supabase locale. C'est la seule facon
//! d'eprouver la propriete centrale de la tranche, qui est une absence : apres
//! un depouillement, `etat_modules` ne rend rien, aucun evenement n'est en base,
//! aucune session n'est apparue - et les deux dates, elles, sont bien la.
//! Un simulacre ne prouverait rien de tout cela.

mod common;

use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vibemap::depouillement::{depouiller, horizon, Marque};

/// Appaire une machine et rend son jeton.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(
        &ctx.url,
        &ctx.anon_key,
        &code,
        "MacBook Pro",
        Some("darwin"),
    )
    .await
    .expect("appairage")
}

/// Un bac de travail jetable : la racine d'un depot et un dossier de journaux.
struct Bac {
    racine: PathBuf,
    depot: PathBuf,
    journaux: PathBuf,
}

impl Bac {
    fn new() -> Self {
        let racine = std::env::temp_dir().join(format!("vibemap-passe-{}", uuid::Uuid::new_v4()));
        let depot = racine.join("atelier");
        let journaux = racine.join("projects");
        std::fs::create_dir_all(&depot).unwrap();
        std::fs::create_dir_all(&journaux).unwrap();
        Bac {
            racine,
            depot,
            journaux,
        }
    }

    /// Ecrit un journal de session, tel que Claude Code en laisse un.
    fn journal(&self, nom: &str, lignes: &[String]) {
        std::fs::write(self.journaux.join(nom), format!("{}\n", lignes.join("\n"))).unwrap();
    }

    /// La carte des depots connus du lecteur.
    fn carte(&self, repo_id: &str) -> BTreeMap<PathBuf, String> {
        BTreeMap::from([(self.depot.clone(), repo_id.to_string())])
    }
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.racine);
    }
}

/// Une ligne d'assistant, avec tout ce qui allume la carte en temps normal.
fn ligne(cwd: &Path, tool: &str, chemin: &str, id: &str, instant: DateTime<Utc>) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant.to_rfc3339(),
        "sessionId": uuid::Uuid::new_v4().to_string(),
        "cwd": cwd.to_string_lossy(),
        "gitBranch": "main",
        "message": {
            "role": "assistant",
            "model": "claude-opus-4-8",
            "content": [{
                "type": "tool_use",
                "id": id,
                "name": tool,
                "input": { "file_path": cwd.join(chemin).to_string_lossy() }
            }],
            "usage": { "input_tokens": 2000, "output_tokens": 180 }
        }
    })
    .to_string()
}

/// Les deux dates d'une zone, telles que la carte les lira.
async fn dates(
    ctx: &common::TestContext,
    repo_id: &str,
    module: &str,
) -> Option<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
    let ligne = ctx
        .touches_modules(repo_id)
        .await
        .into_iter()
        .find(|ligne| ligne["module_path"] == module)?;

    let date = |champ: &str| {
        ligne[champ]
            .as_str()
            .map(|s| s.parse::<DateTime<Utc>>().expect("une date valide"))
    };

    Some((date("derniere_ecriture"), date("derniere_lecture")))
}

/// La propriete centrale : le passe donne ses dates, et n'allume rien.
///
/// C'est le critere d'acceptation de l'issue, verifie de l'exterieur : rien
/// n'est deduit de la forme du code, tout est relu dans la base.
#[tokio::test]
async fn un_depouillement_n_allume_aucune_couleur() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &["src", "src/core"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let bac = Bac::new();
    let maintenant = Utc::now();
    let ecrit = maintenant - Duration::days(12);
    let lu = maintenant - Duration::days(4);

    bac.journal(
        "vieille-session.jsonl",
        &[
            ligne(&bac.depot, "Write", "src/core/auth.rs", "toolu_1", ecrit),
            ligne(&bac.depot, "Read", "src/core/auth.rs", "toolu_2", lu),
        ],
    );

    let resume = depouiller(
        &client,
        &bac.journaux,
        &bac.carte(&repo_id),
        horizon(maintenant),
        &mut Marque::default(),
    )
    .await;

    assert_eq!(resume.journaux_total, 1);
    assert_eq!(resume.journaux_depouilles, 1);
    assert_eq!(resume.en_defaut, 0);

    // Les deux dates sont la.
    let (ecriture, lecture) = dates(&ctx, &repo_id, "src/core")
        .await
        .expect("la zone doit porter ses deux dates");
    assert_eq!(ecriture, Some(ecrit));
    assert_eq!(lecture, Some(lu));

    // Et rien d'autre. Aucune zone n'est bleue, ambre ni rouge.
    assert!(
        ctx.etat_modules(&repo_id, 600).await.is_empty(),
        "le depouillement a allume la carte"
    );
    // La meme chose sur une fenetre qui couvre tout le passe depouille : c'est
    // ce qui distingue « aucune couleur » de « des couleurs deja eteintes ».
    assert!(
        ctx.etat_modules(&repo_id, 60 * 60 * 24 * 60)
            .await
            .is_empty(),
        "le depouillement a pose de l'activite, seulement trop vieille pour se voir"
    );
    assert!(
        ctx.lire_evenements(&repo_id).await.is_empty(),
        "le journal direct montre des lignes venues du passe"
    );
    assert!(
        ctx.lire_sessions(&repo_id).await.is_empty(),
        "le depouillement a fait apparaitre une session"
    );
}

/// Un journal de quarante-cinq jours ne pose aucune date (FR-051).
#[tokio::test]
async fn un_journal_de_quarante_cinq_jours_ne_pose_rien() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let bac = Bac::new();
    let maintenant = Utc::now();

    bac.journal(
        "vieux.jsonl",
        &[ligne(
            &bac.depot,
            "Write",
            "src/a.rs",
            "toolu_1",
            maintenant - Duration::days(45),
        )],
    );

    let resume = depouiller(
        &client,
        &bac.journaux,
        &bac.carte(&repo_id),
        horizon(maintenant),
        &mut Marque::default(),
    )
    .await;

    assert_eq!(resume.zones_notees, 0);
    assert_eq!(resume.en_defaut, 0);
    assert!(
        ctx.touches_modules(&repo_id).await.is_empty(),
        "une date anterieure a trente jours a ete ecrite"
    );
}

/// Un depot absent du catalogue n'arrete pas le depouillement (FR-066).
#[tokio::test]
async fn un_depot_inconnu_n_arrete_pas_le_depouillement() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let bac = Bac::new();
    let maintenant = Utc::now();
    let ecrit = maintenant - Duration::days(2);

    // Un depot efface du disque, dont les journaux parlent encore.
    let disparu = bac.racine.join("efface");
    bac.journal(
        "a-disparu.jsonl",
        &[ligne(
            &disparu,
            "Write",
            "src/parti.rs",
            "toolu_1",
            maintenant - Duration::days(1),
        )],
    );
    bac.journal(
        "b-encore-la.jsonl",
        &[ligne(&bac.depot, "Write", "src/reste.rs", "toolu_2", ecrit)],
    );

    let resume = depouiller(
        &client,
        &bac.journaux,
        &bac.carte(&repo_id),
        horizon(maintenant),
        &mut Marque::default(),
    )
    .await;

    assert_eq!(resume.journaux_depouilles, 2, "les deux ont ete traites");
    assert_eq!(resume.en_defaut, 0, "le depot disparu n'est pas une erreur");

    let (ecriture, _) = dates(&ctx, &repo_id, "src")
        .await
        .expect("le depot encore la doit porter sa date");
    assert_eq!(ecriture, Some(ecrit));
}

/// Sans aucun journal, le depouillement se termine tout de suite (FR-065).
#[tokio::test]
async fn sans_journal_le_depouillement_se_termine_aussitot() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let bac = Bac::new();
    let resume = depouiller(
        &client,
        &bac.journaux,
        &bac.carte(&repo_id),
        horizon(Utc::now()),
        &mut Marque::default(),
    )
    .await;

    assert_eq!(resume.journaux_total, 0);
    assert_eq!(resume.journaux_depouilles, 0);
    assert_eq!(resume.en_defaut, 0);
}

/// Une date fraiche ne recule pas quand le passe repasse dessus (FR-088).
///
/// C'est le scenario exact du premier depouillement : une zone ecrite ce matin,
/// puis un journal d'il y a un mois ou la meme zone etait ecrite.
#[tokio::test]
async fn le_passe_ne_fait_pas_reculer_une_date_fraiche() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let bac = Bac::new();
    let maintenant = Utc::now();
    let ce_matin = maintenant - Duration::hours(6);

    // Le direct a pose la date de ce matin.
    client
        .pousser_dernieres_touches(
            &repo_id,
            &[vibemap::DerniereTouche {
                chemin: "src".to_string(),
                derniere_ecriture: Some(ce_matin),
                derniere_lecture: None,
            }],
        )
        .await
        .expect("la date fraiche doit etre acceptee");

    // Puis le depouillement passe sur un journal d'il y a un mois.
    bac.journal(
        "il-y-a-un-mois.jsonl",
        &[ligne(
            &bac.depot,
            "Write",
            "src/a.rs",
            "toolu_1",
            maintenant - Duration::days(28),
        )],
    );

    depouiller(
        &client,
        &bac.journaux,
        &bac.carte(&repo_id),
        horizon(maintenant),
        &mut Marque::default(),
    )
    .await;

    let (ecriture, _) = dates(&ctx, &repo_id, "src")
        .await
        .expect("la zone doit porter sa date");
    assert_eq!(
        ecriture,
        Some(ce_matin),
        "le depouillement a fait reculer une date fraiche d'un mois"
    );
}
