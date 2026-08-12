//! Constater une exploration ecrite par un agent (issue #38, F13 du PRD -
//! FR-045 a FR-048).
//!
//! Meme famille que `entamer.rs` (#31) : le signal existe deja dans
//! `activity_events` (#4), tout se joue dans un second trigger SQL sur
//! l'insertion `kind = 'write'`. Rien a ajouter cote daemon - ces tests
//! parlent donc au vrai chemin du daemon (`pousser_activite`), jamais a une
//! insertion de service qui contournerait la RLS que ce chemin traverse
//! (ADR 0001).

mod common;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use vibemap::Activite;

/// Appaire une machine et rend son identite, comme le ferait le daemon.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

/// Un repo pret a porter des blocs, avec son client daemon.
async fn repo_de_test(ctx: &common::TestContext) -> (vibemap::Supabase, String, String) {
    let machine = machine_reliee(ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &[]).await;
    (vibemap::Supabase::new(&ctx.url, &machine.token), machine.machine_id, repo_id)
}

fn ecriture(fichier: &str) -> Activite {
    Activite {
        session_id: Uuid::new_v4().to_string(),
        tool_use_id: Uuid::new_v4().to_string(),
        module_path: fichier.rsplit_once('/').map(|(m, _)| m.to_string()).unwrap_or_default(),
        file_path: fichier.to_string(),
        kind: "write",
        occurred_at: Utc::now(),
    }
}

/// FR-045, le critere d'acceptation central de l'issue : un agent qui ecrit
/// `docs/adr/0012-file-attente.md`, jamais couvert par aucun bloc, fait
/// apparaitre une carte d'exploration directement en « En cours ».
#[tokio::test]
async fn une_ecriture_sous_docs_adr_non_couverte_cree_une_exploration_en_cours() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("docs/adr/0012-file-attente.md")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "une seule carte doit apparaitre");
    assert_eq!(blocs[0]["type"], json!("exploration"));
    assert_eq!(blocs[0]["statut"], json!("doing"), "directement en cours, jamais a faire (FR-045)");
    assert_eq!(blocs[0]["chemin"], json!("docs/adr/0012-file-attente.md"));
    assert!(blocs[0]["ref"].as_i64().is_some(), "elle porte une reference VM-n comme tout travail suivi");
}

/// FR-046 : le titre vient du NOM du fichier, jamais de son contenu - le
/// fichier n'est meme jamais ouvert par ce mecanisme (il vit entierement en
/// SQL, sur la seule colonne `file_path` de l'evenement).
#[tokio::test]
async fn le_titre_vient_du_nom_de_fichier_seul() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("docs/superpowers/plans/sous-dossier/2026-08-12-plan-du-jour.md")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1);
    assert_eq!(
        blocs[0]["titre"],
        json!("2026-08-12-plan-du-jour.md"),
        "le titre est le seul nom de fichier, pas le chemin complet ni un titre lu dans le document"
    );
}

/// Les trois dossiers de memo du produit sont tous couverts, chacun avec sa
/// propre carte independante.
#[tokio::test]
async fn les_trois_dossiers_surveilles_creent_chacun_leur_carte() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    for fichier in [
        "docs/adr/0001-x.md",
        "docs/superpowers/specs/2026-08-12-y.md",
        "docs/superpowers/plans/2026-08-12-z.md",
    ] {
        client
            .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(fichier)])
            .await
            .expect("l'evenement doit etre accepte");
    }

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 3, "chaque fichier obtient sa propre carte");
    assert!(blocs.iter().all(|b| b["type"] == json!("exploration")));
}

/// Hors des trois dossiers surveilles, rien ne se cree - README, guides et
/// commentaires d'API ne decident rien (PRD, "Les quatre types") et
/// n'entrent jamais dans le tableau par ce mecanisme.
#[tokio::test]
async fn hors_des_trois_dossiers_surveilles_rien_ne_se_cree() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    for fichier in ["README.md", "docs/prd/PRD-1.md", "docs/guides/x.md", "web/app/hero/page.tsx"] {
        client
            .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(fichier)])
            .await
            .expect("l'evenement doit etre accepte");
    }

    assert!(ctx.lire_blocs(&repo_id).await.is_empty(), "aucun de ces chemins n'est un dossier surveille");
}

/// Le piege du faux prefixe de dossier (piste de casse de l'issue) :
/// `docs/adr-old/` n'est pas `docs/adr/`, meme si la chaine `docs/adr`
/// prefixe bien les deux au sens des caracteres.
#[tokio::test]
async fn un_dossier_qui_ressemble_a_docs_adr_sans_en_etre_un_nest_pas_surveille() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("docs/adr-old/0001-ancienne-decision.md")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    assert!(ctx.lire_blocs(&repo_id).await.is_empty(), "« docs/adr-old » n'est pas « docs/adr »");
}

/// FR-045, deuxieme moitie du critere central : un fichier deja couvert par
/// un bloc existant (ici, ancre exactement sur ce fichier) ne cree pas de
/// second bloc.
#[tokio::test]
async fn un_fichier_deja_couvert_par_un_bloc_existant_ne_cree_pas_de_second_bloc() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let existant = ctx
        .creer_bloc(&repo_id, "Deja suivi a la main", "technique", "docs/adr/0012-file-attente.md")
        .await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("docs/adr/0012-file-attente.md")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "aucun second bloc ne doit apparaitre");
    assert_eq!(blocs[0]["id"], existant["id"], "c'est bien le meme bloc, pas un doublon");
}

/// La couverture reprend la notion de prefixe segmente du plus profond
/// d'`entamer_par_ecriture()` (#31) : un bloc ancre au DOSSIER couvre tout
/// fichier qu'il contient, pas seulement une carte ancree au fichier
/// lui-meme.
#[tokio::test]
async fn un_bloc_ancre_au_dossier_couvre_tout_fichier_quil_contient() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    ctx.creer_bloc(&repo_id, "Toutes les ADR de ce trimestre", "technique", "docs/adr").await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("docs/adr/0099-nouvelle-decision.md")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "le dossier couvre le fichier, aucune exploration ne doit s'ajouter");
}

/// Le meme piege de faux prefixe, cote couverture cette fois (et non plus
/// cote perimetre des trois dossiers) : un bloc ancre a un dossier voisin ne
/// doit jamais couvrir par une simple co\u{ef}ncidence de caracteres.
#[tokio::test]
async fn un_bloc_ancre_a_un_dossier_voisin_ne_couvre_pas_par_coincidence_de_caracteres() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    ctx.creer_bloc(&repo_id, "Un tout autre chantier", "technique", "docs/adr-general").await;

    client
        .pousser_activite(
            &machine_id,
            &repo_id,
            Some("main"),
            &[ecriture("docs/adr/0001-x.md")],
        )
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    let explorations: Vec<_> = blocs.iter().filter(|b| b["type"] == json!("exploration")).collect();
    assert_eq!(explorations.len(), 1, "« docs/adr-general » ne prefixe pas « docs/adr/0001-x.md » au sens des segments");
}

/// Decision documentee de la migration (voir son commentaire) : la
/// couverture ne filtre PAS sur le statut. Un bloc deja Termine a cette
/// adresse compte tout autant qu'un bloc vivant - contrairement a
/// `entamer_par_ecriture()`, qui ne regarde que les travaux vivants pour une
/// raison differente (ne pas rouvrir un travail livre). Ici, la question
/// n'est pas « faut-il reprendre ce travail » mais « ce chemin porte-t-il
/// deja une carte » ; un bloc termine y repond encore oui.
#[tokio::test]
async fn un_bloc_deja_termine_a_cet_emplacement_couvre_aussi() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "ADR deja livree", "technique", "docs/adr/0001-x.md").await;
    ctx.poser_statut_bloc(bloc["id"].as_str().unwrap(), "done").await;

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("docs/adr/0001-x.md")])
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "le bloc termine couvre deja ce chemin, aucune exploration ne doit s'ajouter");
}

/// FR-045 : une issue (pas seulement un bloc) ancree a ce chemin couvre tout
/// autant - meme routage que #31, blocs et issues sont routables de la meme
/// maniere.
#[tokio::test]
async fn une_issue_ancree_a_ce_chemin_couvre_aussi() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Chantier decoupe", "feature", "docs/adr").await;
    ctx.creer_issue(bloc["id"].as_str().unwrap(), "Premiere ADR", Some("docs/adr/0001-x.md")).await;

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("docs/adr/0001-x.md")])
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert!(
        blocs.iter().all(|b| b["type"] != json!("exploration")),
        "l'issue couvre deja ce chemin, aucune exploration ne doit apparaitre"
    );
}

/// Etancheite entre depots : le meme chemin, non couvert dans un depot A,
/// mais couvert dans un depot B, ne doit influencer ni l'un ni l'autre.
#[tokio::test]
async fn la_couverture_et_la_creation_se_scopent_par_depot() {
    let ctx = common::TestContext::new().await;
    let (client_a, machine_a, repo_a) = repo_de_test(&ctx).await;
    let machine_b = ctx.create_machine("Autre machine").await;
    let repo_b = ctx.creer_repo(&machine_b, &[]).await;

    ctx.creer_bloc(&repo_b, "Couvert seulement chez B", "technique", "docs/adr").await;

    client_a
        .pousser_activite(&machine_a, &repo_a, Some("main"), &[ecriture("docs/adr/0001-x.md")])
        .await
        .expect("l'evenement doit etre accepte");

    let blocs_a = ctx.lire_blocs(&repo_a).await;
    assert_eq!(blocs_a.len(), 1, "le depot A n'a rien qui couvre ce chemin, une exploration doit y apparaitre");
    assert_eq!(blocs_a[0]["type"], json!("exploration"));

    let blocs_b = ctx.lire_blocs(&repo_b).await;
    assert_eq!(blocs_b.len(), 1, "le depot B garde son unique bloc, l'ecriture de A ne l'a pas double");
}

/// Une lecture n'entame ni ne fait apparaitre quoi que ce soit (meme regle
/// que FR-009/#31 : seule une ecriture compte).
#[tokio::test]
async fn une_lecture_ne_cree_rien() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let mut lecture = ecriture("docs/adr/0001-x.md");
    lecture.kind = "read";

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[lecture])
        .await
        .expect("l'evenement doit etre accepte");

    assert!(ctx.lire_blocs(&repo_id).await.is_empty(), "une lecture ne cree jamais d'exploration");
}

/// Casse (pistee par l'issue) : deux agents qui ecrivent le meme fichier au
/// meme instant ne doivent jamais produire deux cartes - l'index unique
/// partiel de la migration est le seul garant reel d'un tel cas, un simple
/// NOT EXISTS avant l'insertion ne le serait pas sous acces concurrent.
#[tokio::test]
async fn deux_ecritures_concurrentes_sur_le_meme_fichier_ne_creent_quune_exploration() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let fichier = "docs/superpowers/plans/2026-08-12-course.md";
    let evenements_a = [ecriture(fichier)];
    let evenements_b = [ecriture(fichier)];
    let (a, b) = tokio::join!(
        client.pousser_activite(&machine_id, &repo_id, Some("main"), &evenements_a),
        client.pousser_activite(&machine_id, &repo_id, Some("main"), &evenements_b),
    );
    a.expect("le premier evenement doit etre accepte");
    b.expect("le second evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    let explorations: Vec<_> = blocs.iter().filter(|bloc| bloc["chemin"] == json!(fichier)).collect();
    assert_eq!(explorations.len(), 1, "deux ecritures simultanees ne doivent produire qu'une seule carte");
}

/// FR-048 : un bloc que le systeme a cree seul se renomme comme n'importe
/// quel autre - rien ne le protege, contrairement aux colonnes `prd_*`
/// (`prd_champs_proteges`, #37). Prouve directement par un PATCH avec le
/// jeton de l'utilisateur, le meme geste qu'un bouton "Renommer" du tableau.
#[tokio::test]
async fn un_bloc_dexploration_cree_par_le_systeme_se_renomme() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("docs/adr/0012-file-attente.md")])
        .await
        .expect("l'evenement doit etre accepte");
    let bloc_id = ctx.lire_blocs(&repo_id).await[0]["id"].as_str().unwrap().to_string();

    ctx.patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "titre": "File d'attente : la vraie histoire" }))
        .await
        .expect("renommer un bloc d'exploration cree par le systeme doit rester possible (FR-048)");

    let apres = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres["titre"], json!("File d'attente : la vraie histoire"));
}

/// FR-048 : il se retype aussi, exactement comme #34 l'a deja prouve pour
/// n'importe quel bloc (FR-032) - ce test verifie juste qu'un bloc d'ORIGINE
/// systeme ne fait l'objet d'aucune exception cachee.
#[tokio::test]
async fn un_bloc_dexploration_cree_par_le_systeme_se_retype() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("docs/adr/0012-file-attente.md")])
        .await
        .expect("l'evenement doit etre accepte");
    let bloc_id = ctx.lire_blocs(&repo_id).await[0]["id"].as_str().unwrap().to_string();

    ctx.patch_bloc_avec_jeton(&ctx.user_token, &bloc_id, json!({ "type": "technique" }))
        .await
        .expect("retyper un bloc d'exploration cree par le systeme doit rester possible (FR-032/FR-048)");

    let apres = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres["type"], json!("technique"));
}

/// FR-048 : il se supprime aussi, avec le jeton de l'utilisateur (jamais la
/// cle de service) - le meme geste qu'un bouton "Supprimer" du tableau.
#[tokio::test]
async fn un_bloc_dexploration_cree_par_le_systeme_se_supprime() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture("docs/adr/0012-file-attente.md")])
        .await
        .expect("l'evenement doit etre accepte");
    let bloc_id = ctx.lire_blocs(&repo_id).await[0]["id"].as_str().unwrap().to_string();

    ctx.supprimer_bloc_avec_jeton(&ctx.user_token, &bloc_id)
        .await
        .expect("supprimer un bloc d'exploration cree par le systeme doit rester possible (FR-048)");

    assert!(ctx.lire_blocs(&repo_id).await.is_empty());
}

/// Casse (pistee par l'issue) : un fichier supprime puis reecrit avec le
/// meme contenu ne doit pas faire apparaitre une seconde carte tant que la
/// premiere existe encore - la couverture se lit en base, jamais sur l'etat
/// du disque.
#[tokio::test]
async fn un_fichier_reecrit_apres_coup_ne_double_pas_la_carte_existante() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let fichier = "docs/adr/0001-x.md";
    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(fichier)])
        .await
        .expect("premiere ecriture acceptee");
    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(fichier)])
        .await
        .expect("seconde ecriture (reecriture) acceptee");

    assert_eq!(ctx.lire_blocs(&repo_id).await.len(), 1, "reecrire le meme fichier ne double jamais sa carte");
}

/// Casse (pistee par l'issue) : mais si la carte a ete SUPPRIMEE entre-temps
/// (FR-048), reecrire le fichier en fait naitre une nouvelle - la suppression
/// n'est pas un etat special, elle rend simplement le chemin de nouveau non
/// couvert.
#[tokio::test]
async fn un_fichier_reecrit_apres_suppression_de_sa_carte_en_recree_une() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let fichier = "docs/adr/0001-x.md";
    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(fichier)])
        .await
        .expect("premiere ecriture acceptee");
    let premiere_id = ctx.lire_blocs(&repo_id).await[0]["id"].as_str().unwrap().to_string();

    ctx.supprimer_bloc_avec_jeton(&ctx.user_token, &premiere_id)
        .await
        .expect("suppression acceptee");
    assert!(ctx.lire_blocs(&repo_id).await.is_empty());

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(fichier)])
        .await
        .expect("reecriture acceptee");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "une carte redevient possible une fois l'ancienne supprimee");
    assert_ne!(blocs[0]["id"], json!(premiere_id), "c'est une carte neuve, pas la ressuscitation de l'ancienne");
}

/// Un nom de fichier tres long et a caracteres inhabituels (piste de casse
/// de l'issue) traverse sans encombre - aucune borne de longueur n'existe sur
/// `titre` ni sur `chemin`, et un titre derive d'un nom de fichier ne fait
/// jamais l'objet d'une validation de contenu.
#[tokio::test]
async fn un_nom_de_fichier_tres_long_et_inhabituel_traverse_sans_encombre() {
    let ctx = common::TestContext::new().await;
    let (client, machine_id, repo_id) = repo_de_test(&ctx).await;

    let nom = format!("docs/adr/{}-décision-éàü-'guillemets'.md", "x".repeat(500));
    let fichier = nom.clone();

    client
        .pousser_activite(&machine_id, &repo_id, Some("main"), &[ecriture(&fichier)])
        .await
        .expect("l'evenement doit etre accepte");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1);
    assert_eq!(blocs[0]["titre"], json!(nom.rsplit('/').next().unwrap()));
}
