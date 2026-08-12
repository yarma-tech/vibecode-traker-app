//! Fermer un travail par la reference d'un commit local (issue #32).
//!
//! Deux familles de tests ici :
//! - ceux qui parlent directement a `rpc/ingerer_commit`, avec des
//!   `CommitLocal` fabriques a la main : ils eprouvent la regle de
//!   fermeture elle-meme (SQL), sans avoir besoin d'un vrai depot sur
//!   disque ;
//! - ceux qui montent un vrai depot git temporaire et appellent
//!   `vibemap::commits::{head, commits_depuis}` : ils eprouvent la lecture
//!   du disque, et la boucle complete telle que le daemon la joue.
//!
//! Comme partout ailleurs (ADR 0001), ça parle a la vraie pile Supabase
//! locale.

mod common;

use chrono::Utc;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use vibemap::CommitLocal;

/// Appaire une machine et rend son identite, comme le ferait le daemon.
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

/// Un repo pret a porter des blocs et des issues, avec son client daemon.
async fn repo_de_test(ctx: &common::TestContext) -> (vibemap::Supabase, String) {
    let machine = machine_reliee(ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &["web/app/checkout", "daemon/src"]).await;
    (vibemap::Supabase::new(&ctx.url, &machine.token), repo_id)
}

fn commit(sha: &str, message: &str) -> CommitLocal {
    CommitLocal { sha: sha.to_string(), message: message.to_string(), authored_at: Utc::now() }
}

fn ref_de(bloc_ou_issue: &serde_json::Value) -> i64 {
    bloc_ou_issue["ref"].as_i64().expect("une reference entiere")
}

// --------------------------------------------------------------------------
// La regle de fermeture, contre `rpc/ingerer_commit` directement.
// --------------------------------------------------------------------------

/// Critere d'acceptation central : un commit dont le message contient six
/// references en ferme six - trois issues, trois blocs simples, pour ne pas
/// dependre du chemin qui les a trouvees.
#[tokio::test]
async fn un_commit_nommant_six_references_en_ferme_six() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let mut refs = Vec::new();
    for i in 0..3 {
        let bloc = ctx
            .creer_bloc(&repo_id, &format!("Bloc simple {i}"), "feature", "web/app/a")
            .await;
        refs.push(ref_de(&bloc));
    }
    let bloc_decoupe = ctx.creer_bloc(&repo_id, "Un bloc a cote", "feature", "web/app/b").await;
    let bloc_decoupe_id = bloc_decoupe["id"].as_str().unwrap().to_string();
    for i in 0..3 {
        let issue = ctx.creer_issue(&bloc_decoupe_id, &format!("Issue {i}"), Some("web/app/b")).await;
        refs.push(ref_de(&issue));
    }

    let message = format!(
        "feat: gros commit\n\nFerme {}",
        refs.iter().map(|r| format!("VM-{r}")).collect::<Vec<_>>().join(", ")
    );

    client
        .ingerer_commit(&repo_id, Some("main"), &commit("aaa111", &message))
        .await
        .expect("l'ingestion doit reussir");

    let blocs = ctx.lire_blocs(&repo_id).await;
    let issues = ctx.lire_issues(&repo_id).await;

    for r in &refs[..3] {
        let bloc = blocs.iter().find(|b| b["ref"].as_i64() == Some(*r)).unwrap();
        assert_eq!(bloc["statut"], json!("done"), "le bloc VM-{r} doit etre ferme");
    }
    for r in &refs[3..] {
        let issue = issues.iter().find(|i| i["ref"].as_i64() == Some(*r)).unwrap();
        assert_eq!(issue["statut"], json!("done"), "l'issue VM-{r} doit etre fermee");
    }
}

/// FR-013 : les chemins touches par un commit ne ferment jamais rien. Comme
/// il n'existe plus de `commit_paths`, la garantie est structurelle - ce
/// test prouve seulement qu'un message sans reference ne ferme rien, meme
/// pour un travail vivant a un emplacement que le commit "pourrait" avoir
/// touche.
#[tokio::test]
async fn un_commit_sans_reference_ne_ferme_rien() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Travail vivant", "feature", "web/app/checkout").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    client
        .ingerer_commit(
            &repo_id,
            Some("main"),
            &commit("bbb222", "fix: petit correctif dans web/app/checkout"),
        )
        .await
        .expect("l'ingestion doit reussir meme sans reference");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["statut"], json!("todo"), "sans reference, rien ne bouge");
}

/// FR-014, FR-022 : une reference sur un travail deja termine le rouvre en
/// version suivante et le passe en cours - jamais en "a faire" (FR-023).
#[tokio::test]
async fn une_reference_sur_un_travail_termine_rouvre_en_version_suivante() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Petite correction", "correction", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);

    client
        .ingerer_commit(&repo_id, Some("main"), &commit("ccc1", &format!("fix: VM-{r}")))
        .await
        .expect("premiere fermeture");

    let apres_fermeture = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres_fermeture["statut"], json!("done"));
    assert_eq!(apres_fermeture["version"], json!(1));

    client
        .ingerer_commit(&repo_id, Some("main"), &commit("ccc2", &format!("fix: encore VM-{r}")))
        .await
        .expect("reouverture");

    let apres_reouverture = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        apres_reouverture["statut"],
        json!("doing"),
        "une reference sur un travail termine le rouvre en cours, jamais en a faire"
    );
    assert_eq!(apres_reouverture["version"], json!(2));
}

/// FR-024, et l'avertissement de la tache : le SET d'un UPDATE lit la ligne
/// AVANT l'ecriture, le RETURNING la lit APRES. `fermetures` doit donc
/// porter une ligne par CLOTURE, jamais par reouverture - sinon l'historique
/// mentirait sur ce que chaque commit a fait.
#[tokio::test]
async fn fermetures_ne_porte_que_les_clotures_pas_les_reouvertures() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Va et vient", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);

    // v1 : ferme.
    client
        .ingerer_commit(&repo_id, None, &commit("v1", &format!("VM-{r}")))
        .await
        .expect("premiere fermeture");
    let apres_v1 = ctx.lire_fermetures_bloc(&bloc_id).await;
    assert_eq!(apres_v1.len(), 1, "une fermeture apres la premiere cloture");
    assert_eq!(apres_v1[0]["version"], json!(1));

    // Reouverture en v2 : AUCUNE nouvelle ligne dans fermetures.
    client
        .ingerer_commit(&repo_id, None, &commit("v2-reouvre", &format!("VM-{r}")))
        .await
        .expect("reouverture");
    let apres_reouverture = ctx.lire_fermetures_bloc(&bloc_id).await;
    assert_eq!(
        apres_reouverture.len(),
        1,
        "une reouverture n'est pas une cloture, elle ne doit rien ajouter a fermetures"
    );

    // v2 se referme a son tour : une seconde fermeture, avec version = 2.
    client
        .ingerer_commit(&repo_id, None, &commit("v2-ferme", &format!("VM-{r}")))
        .await
        .expect("seconde fermeture");
    let apres_v2 = ctx.lire_fermetures_bloc(&bloc_id).await;
    assert_eq!(apres_v2.len(), 2, "la seconde cloture ajoute sa propre ligne");
    assert_eq!(apres_v2[1]["version"], json!(2));
    assert_ne!(
        apres_v2[0]["commit_id"], apres_v2[1]["commit_id"],
        "chaque version conserve le commit qui l'a close, pas toujours le meme (FR-024)"
    );
}

/// FR-015 : une reference qui ne correspond a aucun travail du depot est
/// ignoree sans bruit - pas d'erreur, rien ne bouge ailleurs.
#[tokio::test]
async fn une_reference_inconnue_est_ignoree_sans_bruit() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Travail sans rapport", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();

    client
        .ingerer_commit(&repo_id, None, &commit("ddd", "fix: VM-999999 n'existe pas"))
        .await
        .expect("une reference inconnue ne doit pas faire echouer l'ingestion");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(relu["statut"], json!("todo"), "rien ne doit avoir bouge");
}

/// FR-016, avec les vrais messages de ce depot en fixture : `VM-<n>` ferme,
/// `#<n>` ne ferme jamais, meme dans un message de fusion GitHub authentique.
#[tokio::test]
async fn vm_ferme_diese_ne_ferme_pas_avec_les_vrais_messages_du_depot() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Le cout par repo", "feature", "daemon/src").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);

    // Deux vrais messages tires de l'historique de ce depot : `#7` designe
    // une issue GitHub, jamais VM-<r>, quelle que soit sa valeur.
    client
        .ingerer_commit(
            &repo_id,
            Some("main"),
            &commit("real1", "feat(#7): les jetons, le cout et le temps par repo"),
        )
        .await
        .expect("ingestion du premier message reel");
    client
        .ingerer_commit(
            &repo_id,
            Some("main"),
            &commit("real2", "Merge pull request #17 from yarma-tech/feat/7-cout"),
        )
        .await
        .expect("ingestion du second message reel");

    let toujours_ouvert = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        toujours_ouvert["statut"],
        json!("todo"),
        "#7 et #17 designent des issues GitHub, jamais une reference VM-"
    );

    client
        .ingerer_commit(&repo_id, Some("main"), &commit("real3", &format!("fix: VM-{r} regle enfin")))
        .await
        .expect("ingestion de la fermeture");

    let ferme = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(ferme["statut"], json!("done"), "VM-{r} doit fermer, lui");
}

/// Reingerer le meme commit (meme sha) ne ferme rien une seconde fois :
/// l'unicite `(repo_id, sha)` absorbe le doublon avant meme que la
/// fermeture ne soit tentee.
#[tokio::test]
async fn reingerer_le_meme_commit_ne_ferme_rien_une_seconde_fois() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Un seul travail", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);
    let c = commit("meme-sha", &format!("fix: VM-{r}"));

    client.ingerer_commit(&repo_id, None, &c).await.expect("premiere ingestion");
    let apres_premiere = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(apres_premiere["statut"], json!("done"));
    assert_eq!(apres_premiere["version"], json!(1));

    client.ingerer_commit(&repo_id, None, &c).await.expect("reingestion, meme sha");

    let apres_seconde = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        apres_seconde["version"],
        json!(1),
        "le meme commit reingere ne doit ni refermer ni rouvrir une seconde fois"
    );
    assert_eq!(apres_seconde["statut"], json!("done"));
    assert_eq!(
        ctx.lire_fermetures_bloc(&bloc_id).await.len(),
        1,
        "une seule fermeture, malgre les deux ingestions"
    );
}

/// Un bloc decoupe ne se ferme jamais directement par sa propre reference :
/// il derive de ses issues (§4.4 de la conception). Sans la clause qui
/// l'ecarte, cette meme reference heurterait `bloc_statut_protege` (#30) et
/// ferait echouer TOUTE la fermeture, y compris celle des autres references
/// du meme commit - la fonction doit donc l'ignorer silencieusement.
#[tokio::test]
async fn un_bloc_decoupe_nest_jamais_ferme_directement_par_sa_propre_reference() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Chantier a decouper", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let bloc_ref = ref_de(&bloc);
    ctx.creer_issue(&bloc_id, "Premiere moitie", None).await;

    client
        .ingerer_commit(
            &repo_id,
            None,
            &commit("eee", &format!("fix: VM-{bloc_ref} (le bloc, pas l'issue)")),
        )
        .await
        .expect("l'ingestion ne doit pas echouer, meme si le bloc est decoupe");

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        relu["statut"],
        json!("todo"),
        "un bloc decoupe derive de ses issues, une reference sur lui-meme est sans effet"
    );
    assert!(
        ctx.lire_fermetures_bloc(&bloc_id).await.is_empty(),
        "aucune fermeture ne doit avoir ete posee sur un bloc decoupe"
    );
}

/// Fermer la derniere issue d'un bloc decoupe, par reference, doit deriver
/// le bloc en "termine" - la fermeture (F4) et la derivation (F5) doivent
/// composer sans se marcher dessus.
#[tokio::test]
async fn fermer_toutes_les_issues_dun_bloc_decoupe_le_derive_en_termine() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Bloc a deux issues", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let issue_a = ctx.creer_issue(&bloc_id, "Moitie A", None).await;
    let issue_b = ctx.creer_issue(&bloc_id, "Moitie B", Some("web/app/a/b")).await;

    let message = format!("feat: VM-{} et VM-{}", ref_de(&issue_a), ref_de(&issue_b));
    client
        .ingerer_commit(&repo_id, None, &commit("fff", &message))
        .await
        .expect("les deux issues doivent se fermer");

    let bloc_relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        bloc_relu["statut"],
        json!("done"),
        "toutes les issues fermees derivent le bloc en termine (FR-017)"
    );
}

/// Un compte ne voit jamais les commits d'un autre : meme garantie que sur
/// `blocs` et `issues`, ici verifiee sur `commits`.
#[tokio::test]
async fn un_compte_ne_voit_pas_les_commits_dun_autre() {
    let ctx = common::TestContext::new().await;
    let autre = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    client
        .ingerer_commit(&repo_id, None, &commit("secret", "fix: rien qui regarde l'autre compte"))
        .await
        .expect("ingestion normale");

    let http = reqwest::Client::new();
    let reponse = http
        .get(format!("{}/rest/v1/commits?repo_id=eq.{repo_id}&select=*", ctx.url))
        .header("apikey", &ctx.anon_key)
        .bearer_auth(&autre.user_token)
        .send()
        .await
        .expect("lecture avec le jeton d'un autre compte");

    let lus: serde_json::Value = reponse.json().await.expect("reponse JSON");
    assert_eq!(
        lus.as_array().map(Vec::len),
        Some(0),
        "le compte d'un autre utilisateur ne doit voir aucun commit de ce repo"
    );
}

/// `fermer_par_reference` est en security definer et executable par tout
/// compte authentifie (c'est ce qui permet a `ingerer_commit` de l'appeler
/// depuis les droits du daemon). Sans verifier que l'appelant a le droit de
/// toucher le depot du commit, n'importe quel compte peut fermer le travail
/// d'un autre en connaissant - ou en devinant - un `commit_id`. Reproduit
/// directement contre `rpc/fermer_par_reference`, avec le jeton d'un compte
/// qui n'a AUCUN droit sur le depot vise.
#[tokio::test]
async fn un_compte_ne_peut_pas_fermer_un_travail_dans_le_depot_dun_autre() {
    let ctx = common::TestContext::new().await;
    let intrus = common::TestContext::new().await;
    let (_client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Travail de la victime", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);

    // Un commit du depot de la victime, pose sans passer par `ingerer_commit`
    // (RLS bypass volontaire ici : simule un commit deja connu, quelle que
    // soit la maniere dont il est arrive en base) - c'est `fermer_par_reference`
    // elle-meme qui doit refuser d'agir dessus pour un appelant etranger.
    let commit_id = ctx.poser_commit_brut(&repo_id, "vole", &format!("fix: VM-{r}")).await;

    let http = reqwest::Client::new();
    let reponse = http
        .post(format!("{}/rest/v1/rpc/fermer_par_reference", ctx.url))
        .header("apikey", &ctx.anon_key)
        .bearer_auth(&intrus.user_token)
        .json(&json!({ "p_commit_id": commit_id }))
        .send()
        .await
        .expect("appel de fermer_par_reference par l'intrus");

    // On ne juge pas au code HTTP : la fonction reste executable (le daemon
    // en a besoin), le refus doit se lire dans la donnee, pas dans le
    // transport.
    let _ = reponse.status();

    let relu = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(
        relu["statut"],
        json!("todo"),
        "un compte sans droit sur ce depot ne doit jamais pouvoir fermer son travail"
    );
    assert!(
        ctx.lire_fermetures_bloc(&bloc_id).await.is_empty(),
        "aucune fermeture ne doit avoir ete posee par un appelant sans droit sur le depot"
    );
}

/// FR-016, au-dela de la seule distinction `VM-`/`#` : le motif doit
/// reconnaitre une reference en MOT isole, jamais comme sous-chaine d'un
/// autre mot. Sans frontiere de mot, `VM-7` a l'interieur de `SVM-7` (ou
/// `KVM-3`, jargon courant de machine virtuelle) fermerait un travail que le
/// message n'a jamais nomme - exactement ce que le PRD interdit (« mieux
/// vaut rater une fermeture que d'en inventer une »).
#[tokio::test]
async fn svm_et_kvm_ne_ferment_rien_la_reference_doit_etre_un_mot_isole() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc_svm = ctx.creer_bloc(&repo_id, "Pas la machine virtuelle", "feature", "web/app/a").await;
    let bloc_svm_id = bloc_svm["id"].as_str().unwrap().to_string();
    let r_svm = ref_de(&bloc_svm);

    let bloc_kvm = ctx.creer_bloc(&repo_id, "Pas non plus celle-la", "feature", "web/app/b").await;
    let bloc_kvm_id = bloc_kvm["id"].as_str().unwrap().to_string();
    let r_kvm = ref_de(&bloc_kvm);

    client
        .ingerer_commit(&repo_id, None, &commit("svm", &format!("chore: SVM-{r_svm} mise a niveau")))
        .await
        .expect("ingestion");
    client
        .ingerer_commit(&repo_id, None, &commit("kvm", &format!("chore: deploiement KVM-{r_kvm} en cours")))
        .await
        .expect("ingestion");

    assert_eq!(
        ctx.lire_bloc(&bloc_svm_id).await["statut"],
        json!("todo"),
        "« SVM-{r_svm} » contient « VM-{r_svm} » comme sous-chaine, ça ne doit pas fermer"
    );
    assert_eq!(
        ctx.lire_bloc(&bloc_kvm_id).await["statut"],
        json!("todo"),
        "« KVM-{r_kvm} » contient « VM-{r_kvm} » comme sous-chaine, ça ne doit pas fermer non plus"
    );
}

/// Le pendant positif du test precedent : une fois la frontiere de mot
/// imposee, `VM-<n>` doit encore fermer a toutes les positions ou un
/// message reel le pose - en tete, collee a une ponctuation finale, et
/// entre parentheses.
#[tokio::test]
async fn vm_ferme_a_toutes_les_positions_de_mot_debut_fin_parentheses() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc_debut = ctx.creer_bloc(&repo_id, "Ferme en tete de message", "feature", "web/app/a").await;
    let bloc_debut_id = bloc_debut["id"].as_str().unwrap().to_string();
    let r_debut = ref_de(&bloc_debut);

    let bloc_ponct = ctx.creer_bloc(&repo_id, "Ferme colle a un point", "feature", "web/app/b").await;
    let bloc_ponct_id = bloc_ponct["id"].as_str().unwrap().to_string();
    let r_ponct = ref_de(&bloc_ponct);

    let bloc_paren = ctx.creer_bloc(&repo_id, "Ferme entre parentheses", "feature", "web/app/c").await;
    let bloc_paren_id = bloc_paren["id"].as_str().unwrap().to_string();
    let r_paren = ref_de(&bloc_paren);

    client
        .ingerer_commit(&repo_id, None, &commit("debut", &format!("VM-{r_debut} regle le probleme")))
        .await
        .expect("ingestion");
    client
        .ingerer_commit(&repo_id, None, &commit("ponct", &format!("fix: VM-{r_ponct}.")))
        .await
        .expect("ingestion");
    client
        .ingerer_commit(&repo_id, None, &commit("paren", &format!("fix: corrige (VM-{r_paren})")))
        .await
        .expect("ingestion");

    assert_eq!(ctx.lire_bloc(&bloc_debut_id).await["statut"], json!("done"), "reference en tete de message");
    assert_eq!(ctx.lire_bloc(&bloc_ponct_id).await["statut"], json!("done"), "reference collee a un point final");
    assert_eq!(ctx.lire_bloc(&bloc_paren_id).await["statut"], json!("done"), "reference entre parentheses");
}

/// Decision explicite, testee : le PRD ne definit que la forme MAJUSCULE
/// `VM-7` (FR-016). `vm-7` en minuscules n'est pas reconnu - une variante en
/// minuscule est plus probable comme mot ordinaire ("vm-7" est un nom
/// d'hote courant en infra) que comme reference deliberement copiee depuis
/// le tableau, et « mieux vaut rater une fermeture que d'en inventer une »
/// tranche en faveur du refus.
#[tokio::test]
async fn vm_minuscule_ne_ferme_pas_seule_la_forme_majuscule_est_reconnue() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;

    let bloc = ctx.creer_bloc(&repo_id, "Nom d'hote ambigu", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);

    client
        .ingerer_commit(&repo_id, None, &commit("minuscule", &format!("chore: vm-{r} redemarree")))
        .await
        .expect("ingestion");

    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("todo"),
        "vm-{r} en minuscules ne doit pas fermer, seule la forme VM- majuscule le fait"
    );
}

// --------------------------------------------------------------------------
// La lecture du disque, avec un vrai depot git.
// --------------------------------------------------------------------------

/// Un depot jetable, avec une identite git locale prete a committer.
fn depot_git_temporaire(nom: &str) -> PathBuf {
    let racine = std::env::temp_dir().join(format!("vibemap-commits-{nom}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&racine).unwrap();

    Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .current_dir(&racine)
        .status()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@vibemap.local"])
        .current_dir(&racine)
        .status()
        .expect("git config user.email");
    Command::new("git")
        .args(["config", "user.name", "Vibe Map Test"])
        .current_dir(&racine)
        .status()
        .expect("git config user.name");

    racine
}

fn committer(racine: &Path, fichier: &str, contenu: &str, message: &str) -> String {
    std::fs::write(racine.join(fichier), contenu).unwrap();
    Command::new("git").args(["add", "-A"]).current_dir(racine).status().expect("git add");
    Command::new("git")
        .args(["commit", "--quiet", "-m", message])
        .current_dir(racine)
        .status()
        .expect("git commit");

    vibemap::head(racine).expect("HEAD doit exister apres un commit")
}

/// Meme chose, mais avec une date d'auteur imposee : deux commits sur des
/// branches qui divergent peuvent tomber dans la meme seconde d'horloge
/// reelle, et l'ordre de `git log` entre deux lignes non liees devient alors
/// indetermine (granularite seconde du timestamp git). Des dates explicites
/// et distinctes rendent l'ordre reproductible, sans rapport avec la
/// vitesse d'execution du test.
fn committer_a(racine: &Path, fichier: &str, contenu: &str, message: &str, quand: &str) -> String {
    std::fs::write(racine.join(fichier), contenu).unwrap();
    Command::new("git").args(["add", "-A"]).current_dir(racine).status().expect("git add");
    Command::new("git")
        .args(["commit", "--quiet", "-m", message])
        .env("GIT_AUTHOR_DATE", quand)
        .env("GIT_COMMITTER_DATE", quand)
        .current_dir(racine)
        .status()
        .expect("git commit");

    vibemap::head(racine).expect("HEAD doit exister apres un commit")
}

/// `head()` rend `None` sur un depot qui n'a encore aucun commit.
#[test]
fn head_rend_none_sans_commit() {
    let racine = depot_git_temporaire("vide");
    assert_eq!(vibemap::head(&racine), None);
}

/// `commits_depuis` lit du plus ancien au plus recent, et ecarte les
/// fusions : une fusion n'a jamais nomme un travail par elle-meme, seuls les
/// commits qu'elle integre l'ont deja fait un par un (§5.2 de la conception).
#[test]
fn commits_depuis_lit_du_plus_ancien_au_plus_recent_et_ecarte_les_fusions() {
    let racine = depot_git_temporaire("ordre");
    let avant = committer_a(&racine, "a.txt", "1", "premier commit, avant la fenetre", "2024-01-01T00:00:01Z");

    let un = committer_a(&racine, "b.txt", "1", "VM-1 : deuxieme commit", "2024-01-01T00:00:02Z");
    let deux = committer_a(&racine, "c.txt", "1", "VM-2 : troisieme commit", "2024-01-01T00:00:03Z");

    // Une branche qui diverge, fusionnee sans fast-forward : le commit de
    // fusion ne doit jamais apparaitre dans `commits_depuis`.
    Command::new("git")
        .args(["checkout", "-b", "cote", &avant])
        .current_dir(&racine)
        .status()
        .expect("checkout de la branche laterale");
    let lateral = committer_a(&racine, "d.txt", "1", "VM-3 : commit lateral", "2024-01-01T00:00:04Z");
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&racine)
        .status()
        .expect("retour sur main");
    Command::new("git")
        .args(["merge", "--no-ff", "-m", "merge: fusion de cote", "cote"])
        .current_dir(&racine)
        .status()
        .expect("fusion");

    let commits = vibemap::commits_depuis(&racine, &avant);
    let shas: Vec<&str> = commits.iter().map(|c| c.sha.as_str()).collect();

    assert_eq!(
        shas,
        vec![un.as_str(), deux.as_str(), lateral.as_str()],
        "les trois commits nommes doivent apparaitre, du plus ancien au plus recent, \
         sans le commit de fusion"
    );
    assert!(commits[0].message.contains("VM-1"));
    assert_eq!(commits.len(), 3, "le commit de fusion ne doit pas etre compte");
}

/// La boucle complete telle que le daemon la joue, sur un vrai depot : au
/// premier passage, `HEAD` est pose sans rien fermer (le passe n'est jamais
/// rejoue) ; au passage suivant, seuls les commits nouveaux ferment.
#[tokio::test]
async fn premier_passage_pose_head_sans_rien_fermer_puis_les_commits_suivants_ferment() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("boucle");

    let bloc = ctx.creer_bloc(&repo_id, "Travail suivi par git", "feature", "web/app/a").await;
    let bloc_id = bloc["id"].as_str().unwrap().to_string();
    let r = ref_de(&bloc);

    // Historique deja present AVANT que le daemon ne connaisse ce depot : il
    // ne doit jamais etre rejoue, meme s'il nomme une reference valide.
    committer(&racine, "avant.txt", "1", &format!("VM-{r} : ne doit jamais fermer"));

    // Premier passage du daemon : HEAD est pose, rien n'est ingere.
    let tete = vibemap::head(&racine).expect("HEAD doit exister");
    client.poser_dernier_commit(&repo_id, &tete).await.expect("pose de HEAD");

    assert_eq!(
        ctx.lire_bloc(&bloc_id).await["statut"],
        json!("todo"),
        "le commit d'avant le branchement du depot ne doit jamais fermer"
    );
    assert!(ctx.lire_commits(&repo_id).await.is_empty(), "rien ne doit encore etre ingere");

    // Un vrai commit, apres branchement : celui-la doit fermer.
    committer(&racine, "apres.txt", "1", &format!("fix: VM-{r} enfin regle"));

    let dernier = client.dernier_commit_connu(&repo_id).await.expect("lecture").expect("connu");
    let nouveaux = vibemap::commits_depuis(&racine, &dernier);
    assert_eq!(nouveaux.len(), 1, "un seul commit nouveau depuis le premier passage");

    let branche = vibemap::branche_courante(&racine);
    for commit in &nouveaux {
        client.ingerer_commit(&repo_id, branche.as_deref(), commit).await.expect("ingestion");
    }
    client
        .poser_dernier_commit(&repo_id, &nouveaux.last().unwrap().sha)
        .await
        .expect("avance la position de lecture");

    let ferme = ctx.lire_bloc(&bloc_id).await;
    assert_eq!(ferme["statut"], json!("done"), "le commit poste apres branchement doit fermer");

    let commits_ingeres = ctx.lire_commits(&repo_id).await;
    assert_eq!(commits_ingeres.len(), 1, "seul le commit apres branchement doit exister en base");
    assert_eq!(commits_ingeres[0]["branch"], json!("main"));
}
