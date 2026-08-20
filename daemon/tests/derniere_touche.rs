//! Les deux dates de derniere touche d'une zone (issue #75, PRD-002 F8).
//!
//! Ces tests parlent a la vraie pile Supabase locale : c'est la seule facon de
//! verifier ce qui fait tout l'interet de la tranche - que l'agregat vit hors
//! de la table des zones, et que la monotonie tient cote base.

mod common;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use vibemap::{Activite, DerniereTouche};

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

fn session() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn evenement(session: &str, module: &str, fichier: &str, kind: &'static str) -> Activite {
    Activite {
        session_id: session.to_string(),
        tool_use_id: uuid::Uuid::new_v4().to_string(),
        module_path: module.to_string(),
        file_path: fichier.to_string(),
        kind,
        occurred_at: Utc::now(),
    }
}

fn ecriture(chemin: &str, quand: DateTime<Utc>) -> DerniereTouche {
    DerniereTouche {
        chemin: chemin.to_string(),
        derniere_ecriture: Some(quand),
        derniere_lecture: None,
    }
}

fn lecture(chemin: &str, quand: DateTime<Utc>) -> DerniereTouche {
    DerniereTouche {
        chemin: chemin.to_string(),
        derniere_ecriture: None,
        derniere_lecture: Some(quand),
    }
}

/// La ligne d'agregat d'une zone, telle que la carte la lira.
async fn touches(ctx: &common::TestContext, repo_id: &str, module: &str) -> Option<Value> {
    ctx.touches_modules(repo_id)
        .await
        .into_iter()
        .find(|ligne| ligne["module_path"] == module)
}

fn date(ligne: &Value, champ: &str) -> Option<DateTime<Utc>> {
    ligne[champ]
        .as_str()
        .map(|s| s.parse().expect("une date valide"))
}

/// Un depot minuscule, avec deux niveaux de dossiers.
fn petit_depot() -> PathBuf {
    let racine = std::env::temp_dir().join(format!("vibemap-touche-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(racine.join("src/core")).unwrap();

    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&racine)
        .status()
        .expect("git init");

    std::fs::write(racine.join("README.md"), "titre\n").unwrap();
    std::fs::write(racine.join("src/a.rs"), "une\ndeux\n").unwrap();
    std::fs::write(racine.join("src/core/auth.rs"), "une\ndeux\ntrois\n").unwrap();
    racine
}

/// Tracer bullet : les deux dates se posent et se relisent, chacune de son cote.
#[tokio::test]
async fn les_deux_dates_se_posent_et_se_relisent() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &["web", "web/lib"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let hier = Utc::now() - Duration::days(1);
    let maintenant = Utc::now();

    client
        .pousser_dernieres_touches(
            &repo_id,
            &[DerniereTouche {
                chemin: "web/lib".to_string(),
                derniere_ecriture: Some(maintenant),
                derniere_lecture: Some(hier),
            }],
        )
        .await
        .expect("les deux dates doivent etre acceptees");

    let ligne = touches(&ctx, &repo_id, "web/lib")
        .await
        .expect("la zone doit porter ses dates");

    assert_eq!(date(&ligne, "derniere_ecriture"), Some(maintenant));
    assert_eq!(date(&ligne, "derniere_lecture"), Some(hier));
}

/// FR-039 et FR-040 : une ecriture ne touche pas la date de lecture, et
/// reciproquement.
#[tokio::test]
async fn chaque_date_va_son_chemin() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let lue_le = Utc::now() - Duration::hours(3);
    client
        .pousser_dernieres_touches(&repo_id, &[lecture("src", lue_le)])
        .await
        .expect("la lecture doit passer");

    let ecrite_le = Utc::now();
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", ecrite_le)])
        .await
        .expect("l'ecriture doit passer");

    let ligne = touches(&ctx, &repo_id, "src").await.expect("la zone src");
    assert_eq!(date(&ligne, "derniere_ecriture"), Some(ecrite_le));
    assert_eq!(
        date(&ligne, "derniere_lecture"),
        Some(lue_le),
        "l'ecriture n'avait rien a dire de la lecture"
    );

    // Et l'inverse : une lecture plus recente ne rajeunit pas l'ecriture.
    let relue_le = Utc::now();
    client
        .pousser_dernieres_touches(&repo_id, &[lecture("src", relue_le)])
        .await
        .expect("la seconde lecture doit passer");

    let ligne = touches(&ctx, &repo_id, "src").await.expect("la zone src");
    assert_eq!(date(&ligne, "derniere_ecriture"), Some(ecrite_le));
    assert_eq!(date(&ligne, "derniere_lecture"), Some(relue_le));
}

/// FR-088, le test qui tombe sur le scenario du premier depouillement : une
/// date ne recule jamais, et l'ecriture ancienne ne produit aucune erreur.
#[tokio::test]
async fn une_date_ne_recule_jamais() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let ce_matin = Utc::now() - Duration::hours(6);
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", ce_matin)])
        .await
        .expect("la date fraiche doit passer");

    let il_y_a_un_mois = Utc::now() - Duration::days(30);
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", il_y_a_un_mois)])
        .await
        .expect("une ecriture ancienne est ignoree, jamais refusee");

    let ligne = touches(&ctx, &repo_id, "src").await.expect("la zone src");
    assert_eq!(
        date(&ligne, "derniere_ecriture"),
        Some(ce_matin),
        "la date de ce matin tient face a un depouillement du mois passe"
    );

    // La monotonie n'est pas un refus d'ecrire : une date vraiment plus
    // recente, elle, avance.
    let a_l_instant = Utc::now();
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", a_l_instant)])
        .await
        .expect("une date plus recente doit passer");

    let ligne = touches(&ctx, &repo_id, "src").await.expect("la zone src");
    assert_eq!(date(&ligne, "derniere_ecriture"), Some(a_l_instant));
}

/// Le test le plus important du lot : les dates survivent a une cartographie
/// complete, celle qui efface toutes les zones du depot avant de les repousser.
/// Il echoue si l'agregat vit dans la table des zones (FR-062).
#[tokio::test]
async fn les_dates_survivent_a_une_cartographie() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let racine = petit_depot();
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let plan = vibemap::scanner(Path::new(&racine)).expect("premier scan");
    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("premier envoi du plan");

    let il_y_a_deux_jours = Utc::now() - Duration::days(2);
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src/core", il_y_a_deux_jours)])
        .await
        .expect("la date doit passer");

    // La cartographie repasse cinq minutes plus tard, sur un depot qui a bouge.
    std::fs::write(racine.join("src/core/jeton.rs"), "une\n").unwrap();
    let plan = vibemap::scanner(Path::new(&racine)).expect("second scan");
    client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("second envoi du plan");

    let ligne = touches(&ctx, &repo_id, "src/core")
        .await
        .expect("la zone doit encore porter ses dates apres la cartographie");
    assert_eq!(
        date(&ligne, "derniere_ecriture"),
        Some(il_y_a_deux_jours),
        "la cartographie n'a rien a dire des dates"
    );
}

/// FR-064 : une zone disparue du depot cesse d'etre dessinee, garde ses dates,
/// et les retrouve si elle reapparait.
#[tokio::test]
async fn une_zone_disparue_garde_ses_dates() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let racine = petit_depot();
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let plan = vibemap::scanner(Path::new(&racine)).expect("premier scan");
    let repo_id = client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("premier envoi du plan");

    let ecrite_le = Utc::now() - Duration::days(2);
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src/core", ecrite_le)])
        .await
        .expect("la date doit passer");

    // Le dossier est renomme sur le disque, puis la cartographie repasse.
    std::fs::rename(racine.join("src/core"), racine.join("src/coeur")).unwrap();
    let plan = vibemap::scanner(Path::new(&racine)).expect("scan apres renommage");
    client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("envoi du plan apres renommage");

    assert!(
        touches(&ctx, &repo_id, "src/core").await.is_none(),
        "la zone disparue n'est plus dessinee"
    );
    let survivante = ctx
        .lire_dernieres_touches(&repo_id)
        .await
        .into_iter()
        .find(|ligne| ligne["chemin"] == "src/core");
    assert!(
        survivante.is_some(),
        "sa ligne d'agregat, elle, n'est pas effacee"
    );

    // On remet le dossier sous son ancien nom : la parcelle revient avec ses
    // deux dates d'origine.
    std::fs::rename(racine.join("src/coeur"), racine.join("src/core")).unwrap();
    let plan = vibemap::scanner(Path::new(&racine)).expect("scan apres retour");
    client
        .pousser_plan(&machine.machine_id, &plan)
        .await
        .expect("envoi du plan apres retour");

    let ligne = touches(&ctx, &repo_id, "src/core")
        .await
        .expect("la zone revenue est de nouveau dessinee");
    assert_eq!(date(&ligne, "derniere_ecriture"), Some(ecrite_le));
}

/// Effacer une zone de `modules` ne touche pas l'agregat ; effacer le depot
/// l'emporte, et c'est la seule facon de l'emporter.
#[tokio::test]
async fn l_agregat_ne_part_qu_avec_son_depot() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", Utc::now())])
        .await
        .expect("la date doit passer");

    ctx.supprimer_modules(&repo_id).await;
    assert_eq!(
        ctx.lire_dernieres_touches(&repo_id).await.len(),
        1,
        "la disparition des zones ne touche pas l'agregat"
    );

    // Aucun droit de suppression n'a ete accorde : meme la machine
    // proprietaire ne peut pas effacer une ligne a la main.
    let refus = ctx
        .tenter_supprimer_touche_avec_jeton(&machine.token, &repo_id, "src")
        .await;
    assert!(
        refus.is_err(),
        "une ligne d'agregat ne s'efface pas a la main"
    );
    assert_eq!(ctx.lire_dernieres_touches(&repo_id).await.len(), 1);

    ctx.supprimer_repo(&repo_id).await;
    assert!(
        ctx.lire_dernieres_touches(&repo_id).await.is_empty(),
        "le depot emporte son agregat en cascade"
    );
}

/// FR-043 : l'heritage se calcule a la lecture, par prefixe, sans qu'aucune
/// ligne d'ancetre soit tenue a jour.
#[tokio::test]
async fn une_zone_herite_de_ce_qui_se_passe_sous_elle() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &["web", "web/lib", "web_a"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let ecrite_le = Utc::now();
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("web/lib", ecrite_le)])
        .await
        .expect("la date doit passer");

    let parent = touches(&ctx, &repo_id, "web")
        .await
        .expect("le parent herite de son sous-dossier");
    assert_eq!(date(&parent, "derniere_ecriture"), Some(ecrite_le));

    assert!(
        touches(&ctx, &repo_id, "web_a").await.is_none(),
        "un voisin dont le nom commence pareil n'herite de rien"
    );

    assert_eq!(
        ctx.lire_dernieres_touches(&repo_id).await.len(),
        1,
        "une touche n'ecrit qu'une ligne, celle de la zone touchee"
    );
}

/// Le chemin du direct : pousser des appels d'outils tient les deux dates,
/// sans que l'appelant ait rien a faire de plus.
#[tokio::test]
async fn le_direct_tient_les_deux_dates() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src", "docs"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let s1 = session();

    let evenements = [
        evenement(&s1, "src", "src/auth.rs", "write"),
        evenement(&s1, "docs", "docs/guide.md", "read"),
    ];

    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &evenements)
        .await
        .expect("les evenements doivent passer");

    let src = touches(&ctx, &repo_id, "src").await.expect("la zone src");
    assert!(date(&src, "derniere_ecriture").is_some());
    assert!(
        date(&src, "derniere_lecture").is_none(),
        "personne n'a lu dans src"
    );

    let docs = touches(&ctx, &repo_id, "docs").await.expect("la zone docs");
    assert!(date(&docs, "derniere_lecture").is_some());
    assert!(
        date(&docs, "derniere_ecriture").is_none(),
        "personne n'a ecrit dans docs"
    );

    // Le meme lot rejoue par la file d'attente ne fait rien reculer.
    let avant = date(&src, "derniere_ecriture");
    client
        .pousser_activite(&machine.machine_id, &repo_id, Some("main"), &evenements)
        .await
        .expect("le rejeu doit passer");
    let src = touches(&ctx, &repo_id, "src").await.expect("la zone src");
    assert_eq!(date(&src, "derniere_ecriture"), avant);
}

/// FR-045 : ce qui part sur le reseau est une liste fermee. Le test echoue si
/// un champ s'ajoute a la charge utile.
#[test]
fn la_charge_ne_porte_que_des_chemins_et_des_horodatages() {
    let quand = Utc::now();
    let evenements = [
        Activite {
            session_id: "session-secrete".to_string(),
            tool_use_id: "toolu_secret".to_string(),
            module_path: "web/lib".to_string(),
            file_path: "web/lib/auth.ts".to_string(),
            kind: "write",
            occurred_at: quand,
        },
        Activite {
            session_id: "session-secrete".to_string(),
            tool_use_id: "toolu_autre".to_string(),
            module_path: "web/lib".to_string(),
            file_path: "web/lib/jeton.ts".to_string(),
            kind: "read",
            occurred_at: quand,
        },
    ];

    let touches = DerniereTouche::depuis_activites(&evenements);
    assert_eq!(touches.len(), 1, "une seule zone touchee");

    let charge = touches[0].charge();
    let objet = charge.as_object().expect("un objet JSON");
    let mut cles: Vec<&str> = objet.keys().map(String::as_str).collect();
    cles.sort_unstable();
    assert_eq!(
        cles,
        ["chemin", "ecrit_a", "lu_a"],
        "trois champs, jamais un quatrieme"
    );
    assert_eq!(objet["chemin"], "web/lib");

    // Ni nom de fichier, ni session, ni identifiant d'appel d'outil : rien de
    // tout cela n'a de champ ou se glisser.
    let texte = charge.to_string();
    for interdit in [
        "auth.ts",
        "jeton.ts",
        "session-secrete",
        "toolu_secret",
        "write",
        "/Users/",
    ] {
        assert!(
            !texte.contains(interdit),
            "« {interdit} » ne doit pas sortir de la machine, obtenu : {texte}"
        );
    }
}

/// Une touche sans date de lecture ne porte pas de lecture, et l'agregation
/// garde la plus recente de chaque nature.
#[test]
fn l_agregation_garde_la_plus_recente_de_chaque_nature() {
    let vieux = Utc::now() - Duration::hours(2);
    let recent = Utc::now();

    let touches = DerniereTouche::depuis_activites(&[
        Activite {
            session_id: "s".to_string(),
            tool_use_id: "1".to_string(),
            module_path: "src".to_string(),
            file_path: "src/a.rs".to_string(),
            kind: "write",
            occurred_at: recent,
        },
        Activite {
            session_id: "s".to_string(),
            tool_use_id: "2".to_string(),
            module_path: "src".to_string(),
            file_path: "src/b.rs".to_string(),
            kind: "write",
            occurred_at: vieux,
        },
    ]);

    assert_eq!(
        touches,
        vec![DerniereTouche {
            chemin: "src".to_string(),
            derniere_ecriture: Some(recent),
            derniere_lecture: None,
        }]
    );
}

/// RLS : une machine revoquee n'ecrit plus dans l'agregat.
#[tokio::test]
async fn une_machine_revoquee_n_ecrit_plus_dans_l_agregat() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["src"]).await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", Utc::now())])
        .await
        .expect("avant la revocation, la machine ecrit");

    ctx.revoquer(&machine.machine_id).await;

    let erreur = client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", Utc::now())])
        .await
        .expect_err("une machine revoquee ne doit plus ecrire");
    let message = erreur.to_string();
    assert!(
        message.contains("refuse"),
        "le refus doit venir de la base, obtenu : {message}"
    );
}

/// RLS : une machine n'ecrit que dans l'agregat de ses propres depots.
#[tokio::test]
async fn une_machine_n_ecrit_pas_dans_l_agregat_d_un_autre() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;

    let autre_ctx = common::TestContext::new().await;
    let autre_machine = machine_reliee(&autre_ctx).await;
    let repo_etranger = autre_ctx
        .creer_repo(&autre_machine.machine_id, &["src"])
        .await;

    let client = vibemap::Supabase::new(&ctx.url, &machine.token);
    let erreur = client
        .pousser_dernieres_touches(&repo_etranger, &[ecriture("src", Utc::now())])
        .await
        .expect_err("le depot d'un autre compte doit rester hors d'atteinte");

    assert!(
        autre_ctx
            .lire_dernieres_touches(&repo_etranger)
            .await
            .is_empty(),
        "rien n'a ete pose, obtenu apres : {erreur}"
    );
}
