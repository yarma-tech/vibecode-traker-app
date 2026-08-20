//! L'heritage des deux dates de derniere touche (issue #76, PRD-002 FR-043).
//!
//! Une zone porte le plus recent de ce qui la vise ET de ce qui vise les zones
//! qu'elle contient. Tout se joue a la LECTURE, dans `touches_modules` : une
//! touche n'ecrit qu'une ligne, celle de la zone touchee, et aucune ligne
//! d'ancetre n'est tenue a jour. C'est ce qui interdit toute derive entre une
//! zone et ses parents - un ancetre ne peut pas mentir sur ses descendants
//! puisqu'il ne stocke rien d'eux.
//!
//! Ces tests parlent a la vraie pile Supabase locale : l'heritage vit dans une
//! expression SQL (`starts_with`, plus le cas particulier des parcelles en
//! « . »), et rien d'autre que Postgres ne peut dire si elle est juste.
//!
//! Fichier distinct de `derniere_touche.rs` (issue #75), qui couvre l'ECRITURE
//! monotone et le cycle de vie de l'agregat : ici, on n'eprouve que la lecture.

mod common;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use vibemap::DerniereTouche;

/// Appaire une machine et rend son identite.
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

fn touche(
    chemin: &str,
    ecrite: Option<DateTime<Utc>>,
    lue: Option<DateTime<Utc>>,
) -> DerniereTouche {
    DerniereTouche {
        chemin: chemin.to_string(),
        derniere_ecriture: ecrite,
        derniere_lecture: lue,
    }
}

fn ecriture(chemin: &str, quand: DateTime<Utc>) -> DerniereTouche {
    touche(chemin, Some(quand), None)
}

/// La ligne que la carte lira pour une zone, heritage compris. `None` quand la
/// zone n'a jamais rien recu, ni pour elle-meme ni par ses descendants.
async fn zone(ctx: &common::TestContext, repo_id: &str, module: &str) -> Option<Value> {
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

/// Le coeur de FR-043 : un ancetre porte, pour CHACUNE des deux dates
/// separement, la plus recente de celles de ses descendants.
///
/// Les deux dates du parent viennent ici de deux enfants DIFFERENTS : l'ecriture
/// de `src/rendu`, la lecture de `src/core`. Un heritage qui prendrait « la
/// ligne la plus recente » plutot que « le maximum de chaque date » rendrait
/// les deux dates du meme enfant, et ce test tomberait.
#[tokio::test]
async fn un_ancetre_herite_de_la_plus_recente_de_chaque_date() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(
            &machine.machine_id,
            &["src", "src/core", "src/rendu", "docs"],
        )
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let maintenant = Utc::now();
    let il_y_a_cinq_jours = maintenant - Duration::days(5);
    let il_y_a_trois_jours = maintenant - Duration::days(3);
    let il_y_a_deux_jours = maintenant - Duration::days(2);
    let il_y_a_un_jour = maintenant - Duration::days(1);

    client
        .pousser_dernieres_touches(
            &repo_id,
            &[
                // Ecriture la plus ancienne, lecture la plus recente.
                touche("src/core", Some(il_y_a_trois_jours), Some(il_y_a_un_jour)),
                // Ecriture la plus recente, lecture la plus ancienne.
                touche(
                    "src/rendu",
                    Some(il_y_a_deux_jours),
                    Some(il_y_a_cinq_jours),
                ),
            ],
        )
        .await
        .expect("les touches des deux sous-dossiers doivent passer");

    let parent = zone(&ctx, &repo_id, "src")
        .await
        .expect("le parent herite de ce qui se passe sous lui");
    assert_eq!(
        date(&parent, "derniere_ecriture"),
        Some(il_y_a_deux_jours),
        "l'ecriture du parent est la plus recente des ecritures de ses descendants"
    );
    assert_eq!(
        date(&parent, "derniere_lecture"),
        Some(il_y_a_un_jour),
        "sa lecture est la plus recente des lectures, meme si elle vient d'un autre enfant"
    );

    // Chaque descendant garde les siennes, inchangees : l'heritage remonte, il
    // ne redescend jamais.
    let core = zone(&ctx, &repo_id, "src/core")
        .await
        .expect("la zone core");
    assert_eq!(date(&core, "derniere_ecriture"), Some(il_y_a_trois_jours));
    assert_eq!(date(&core, "derniere_lecture"), Some(il_y_a_un_jour));

    // Une zone que rien n'a jamais touchee, ni elle ni ses descendants, ne rend
    // aucune ligne : l'ecran distingue « rien recu » de « recu il y a
    // longtemps » sans avoir a interpreter une date sentinelle.
    assert!(
        zone(&ctx, &repo_id, "docs").await.is_none(),
        "une zone jamais touchee ne rend rien"
    );

    // La preuve qu'aucune derive n'est possible : deux zones touchees, deux
    // lignes. Le parent herite sans qu'une seule ligne d'ancetre existe.
    assert_eq!(
        ctx.lire_dernieres_touches(&repo_id).await.len(),
        2,
        "l'agregat ne porte que les zones reellement touchees, jamais leurs ancetres"
    );
}

/// L'heritage traverse tous les etages, pas seulement le parent immediat, et il
/// se cumule avec la touche propre de l'ancetre.
#[tokio::test]
async fn l_heritage_remonte_jusqu_a_la_racine() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &["web", "web/lib", "web/lib/supabase"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let il_y_a_quatre_jours = Utc::now() - Duration::days(4);
    let a_l_instant = Utc::now();

    client
        .pousser_dernieres_touches(
            &repo_id,
            &[
                // Le grand-parent a ete touche il y a longtemps, pour lui-meme.
                ecriture("web", il_y_a_quatre_jours),
                // L'arriere-petit-fils vient de l'etre.
                ecriture("web/lib/supabase", a_l_instant),
            ],
        )
        .await
        .expect("les deux touches doivent passer");

    for ancetre in ["web", "web/lib"] {
        let ligne = zone(&ctx, &repo_id, ancetre)
            .await
            .unwrap_or_else(|| panic!("la zone {ancetre} doit porter une date"));
        assert_eq!(
            date(&ligne, "derniere_ecriture"),
            Some(a_l_instant),
            "{ancetre} herite du plus profond, pas seulement de son enfant direct"
        );
    }
}

/// Une parcelle en « . » porte les fichiers poses DIRECTEMENT dans un dossier,
/// a cote de ses sous-dossiers : elle ne recoit rien de ceux-ci, exactement
/// comme `etat_modules` refuse deja de leur donner sa couleur.
#[tokio::test]
async fn une_parcelle_en_point_n_herite_pas_de_ses_sous_dossiers() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(&machine.machine_id, &[".", "src", "src/.", "src/core"])
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let a_l_instant = Utc::now();
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src/core", a_l_instant)])
        .await
        .expect("la touche du sous-dossier doit passer");

    assert!(
        zone(&ctx, &repo_id, "src/.").await.is_none(),
        "« src/. » ne parle que des fichiers a plat de src, jamais de ses sous-dossiers"
    );
    assert!(
        zone(&ctx, &repo_id, ".").await.is_none(),
        "« . » ne parle que des fichiers a la racine du depot"
    );
    assert!(
        zone(&ctx, &repo_id, "src").await.is_some(),
        "le dossier src, lui, herite bien"
    );

    // Et reciproquement : une touche a plat dans src va a « src/. » et au
    // dossier src, jamais a son sous-dossier.
    let plus_tard = Utc::now();
    client
        .pousser_dernieres_touches(&repo_id, &[ecriture("src", plus_tard)])
        .await
        .expect("la touche a plat doit passer");

    let plat = zone(&ctx, &repo_id, "src/.")
        .await
        .expect("« src/. » porte ce qui est pose a plat dans src");
    assert_eq!(date(&plat, "derniere_ecriture"), Some(plus_tard));

    let core = zone(&ctx, &repo_id, "src/core")
        .await
        .expect("la zone core");
    assert_eq!(
        date(&core, "derniere_ecriture"),
        Some(a_l_instant),
        "un descendant n'herite jamais de son parent"
    );
}

/// Les deux pieges de prefixe, celui du bon sens et celui du joker SQL.
///
/// 1. « src_a » n'est pas sous « src » : un simple `t.chemin like m.path || '%'`
///    l'y mettrait. `src` est touche AVANT, et de longue date, pour que sa ligne
///    existe de toute facon : si l'heritage debordait, sa date avancerait - une
///    absence de ligne n'aurait rien prouve.
/// 2. Le piege que le commentaire de `20260804000001_activite.sql` documente
///    deja : dans un motif `like`, le tiret bas de « src_a » est un JOKER, si
///    bien que « src_a » heriterait de son voisin « srcxa ». C'est toute la
///    raison de preferer `starts_with`, et il faut un vrai « srcxa » a cote pour
///    que le test puisse le voir.
#[tokio::test]
async fn un_voisin_de_meme_prefixe_ne_deteint_pas() {
    let ctx = common::TestContext::new().await;
    let machine = machine_reliee(&ctx).await;
    let repo_id = ctx
        .creer_repo(
            &machine.machine_id,
            &["src", "src_a", "src_a/interne", "srcxa", "srcxa/interne"],
        )
        .await;
    let client = vibemap::Supabase::new(&ctx.url, &machine.token);

    let il_y_a_six_jours = Utc::now() - Duration::days(6);
    let il_y_a_deux_heures = Utc::now() - Duration::hours(2);
    let a_l_instant = Utc::now();

    client
        .pousser_dernieres_touches(
            &repo_id,
            &[
                ecriture("src", il_y_a_six_jours),
                ecriture("src_a/interne", il_y_a_deux_heures),
                ecriture("srcxa/interne", a_l_instant),
            ],
        )
        .await
        .expect("les trois touches doivent passer");

    let src = zone(&ctx, &repo_id, "src").await.expect("la zone src");
    assert_eq!(
        date(&src, "derniere_ecriture"),
        Some(il_y_a_six_jours),
        "« src » garde sa propre date : « src_a » n'est pas sous lui"
    );

    let voisin = zone(&ctx, &repo_id, "src_a").await.expect("la zone src_a");
    assert_eq!(
        date(&voisin, "derniere_ecriture"),
        Some(il_y_a_deux_heures),
        "« src_a » n'herite que de SON sous-dossier : son tiret bas n'est pas un joker"
    );

    let sosie = zone(&ctx, &repo_id, "srcxa").await.expect("la zone srcxa");
    assert_eq!(date(&sosie, "derniere_ecriture"), Some(a_l_instant));
}
