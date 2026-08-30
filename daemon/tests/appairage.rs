//! Ce que devient l'appairage par code, une fois le chemin ferme (issue #84).
//!
//! Ce fichier eprouvait l'echange d'un code contre un jeton. Cet echange n'existe
//! plus cote poste (FR-082) : ni l'ecran ni le binaire ne le proposent, et la
//! bibliotheque n'en porte plus la fonction. Ce qui reste a garder, et qui est
//! desormais le sujet de ce fichier, tient en deux promesses opposees :
//!
//! - **rien ne se casse derriere** (FR-026) : les machines reliees par un code
//!   avant ce changement gardent leur jeton, et leurs ecritures sont toujours
//!   acceptees. La fonction `appairer_machine` reste en base - son retrait est un
//!   nettoyage de schema, hors scope -, donc ce test peut fabriquer un jeton
//!   exactement comme l'ancien chemin le faisait, et verifier qu'il ecrit encore ;
//! - **aucune porte ne reste entrouverte** : le binaire n'a plus de sous-commande
//!   d'appairage, et aucun de ses messages ne renvoie vers un code.
//!
//! Le premier test parle a la vraie pile Supabase locale ; les autres n'ont
//! besoin de rien d'autre que du binaire construit.

use std::path::PathBuf;
use std::process::Command;

mod common;

/* ---------- FR-026 : un jeton d'avant ce changement ecrit toujours ---------- */

/// Fabrique une machine comme l'ancien chemin le faisait : un code cree par
/// l'ecran, echange contre un jeton par la fonction de la base.
///
/// POURQUOI l'appel HTTP est ecrit ici et non pris dans la bibliotheque : la
/// bibliotheque ne sait plus faire cela, et c'est precisement ce que cette
/// tranche a retire. Le test doit donc rejouer le geste d'hier a la main pour
/// prouver que son resultat vaut encore aujourd'hui.
async fn machine_appairee_par_code(ctx: &common::TestContext, label: &str) -> (String, String) {
    let code = ctx.creer_code().await;

    let reponse = reqwest::Client::new()
        .post(format!("{}/rest/v1/rpc/appairer_machine", ctx.url))
        .header("apikey", &ctx.anon_key)
        .bearer_auth(&ctx.anon_key)
        .json(&serde_json::json!({
            "p_code": code,
            "p_label": label,
            "p_platform": "darwin",
        }))
        .send()
        .await
        .expect("appel de appairer_machine");

    let statut = reponse.status();
    let corps = reponse.text().await.unwrap_or_default();
    assert!(
        statut.is_success(),
        "appairer_machine a echoue ({statut}) : {corps}"
    );

    let identite: serde_json::Value =
        serde_json::from_str(&corps).expect("reponse JSON de appairer_machine");

    (
        identite["machine_id"]
            .as_str()
            .unwrap_or_else(|| panic!("pas de machine_id dans {identite}"))
            .to_string(),
        identite["token"]
            .as_str()
            .unwrap_or_else(|| panic!("pas de token dans {identite}"))
            .to_string(),
    )
}

/// La promesse de FR-026 : un jeton signe avant ce changement ecrit toujours.
#[tokio::test]
async fn un_jeton_signe_avant_ce_changement_ecrit_toujours() {
    let ctx = common::TestContext::new().await;
    let (machine_id, jeton) = machine_appairee_par_code(&ctx, "MacBook Pro").await;

    let client = vibemap::Supabase::new(&ctx.url, &jeton);
    client
        .announce(&machine_id, chrono::Utc::now())
        .await
        .expect("un jeton d'avant ce changement doit continuer a battre");

    assert!(
        ctx.last_seen_at(&machine_id).await.is_some(),
        "le battement doit avoir atteint la base"
    );

    // Battre ne prouve qu'une colonne : on pousse aussi une carte, qui traverse
    // la RLS de `repos` et de `modules` avec ce meme jeton.
    let empreinte = uuid::Uuid::new_v4().to_string();
    let plan = vibemap::Plan {
        name: "atelier".to_string(),
        identity: format!("local:{empreinte}"),
        root_hash: empreinte,
        remote_owner: None,
        remote_url: None,
        current_branch: Some("main".to_string()),
        loc_total: 10,
        file_count: 1,
        modules: Vec::new(),
    };

    client
        .pousser_plan(&machine_id, &plan)
        .await
        .expect("un jeton d'avant ce changement doit toujours poser sa carte");
}

/// Le jeton herite n'est pas pour autant hors de portee des reglages : revoquer
/// la machine depuis le web coupe ses ecritures, comme pour toute autre.
#[tokio::test]
async fn un_jeton_herite_reste_coupe_par_la_revocation() {
    let ctx = common::TestContext::new().await;
    let (machine_id, jeton) = machine_appairee_par_code(&ctx, "Mac mini").await;

    let client = vibemap::Supabase::new(&ctx.url, &jeton);
    client
        .announce(&machine_id, chrono::Utc::now())
        .await
        .expect("elle bat normalement avant revocation");

    ctx.revoquer(&machine_id).await;

    assert!(
        client
            .announce(&machine_id, chrono::Utc::now())
            .await
            .is_err(),
        "une machine revoquee ne doit plus ecrire, meme avec un jeton herite"
    );
}

/* ---------- le binaire n'a plus de chemin par code ---------- */

/// Un dossier de configuration neuf : le Mac ou l'application n'a jamais ete
/// ouverte. `XDG_CONFIG_HOME` est ce que lit `Config::chemin_par_defaut`.
fn poste_neuf() -> PathBuf {
    let dossier = std::env::temp_dir().join(format!("vibemap-poste-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dossier).expect("creation du dossier de configuration de test");
    dossier
}

/// Le binaire, lance sur un poste ou l'application n'a jamais ecrit sa
/// configuration. Rend (succes, sortie standard, sortie d'erreur).
fn lancer(arguments: &[&str], maison: &PathBuf) -> (bool, String, String) {
    let sortie = Command::new(env!("CARGO_BIN_EXE_vibemap"))
        .args(arguments)
        .env("XDG_CONFIG_HOME", maison)
        // Sans quoi le lecteur irait chercher un jeton au trousseau du poste
        // qui fait tourner les tests, et macOS pourrait ouvrir une fenetre.
        .env("VIBEMAP_TOKEN", "jeton-de-test")
        .output()
        .expect("lancement du binaire vibemap");

    (
        sortie.status.success(),
        String::from_utf8_lossy(&sortie.stdout).to_string(),
        String::from_utf8_lossy(&sortie.stderr).to_string(),
    )
}

/// Le mot « code » comme MOT, jamais comme morceau d'un autre (« codebase »).
///
/// Un `contains("code")` laisserait passer l'inverse de ce qu'on veut : il
/// tomberait sur des mots innocents et raterait la seule chose qui compte, une
/// phrase qui reclame un code d'appairage a un utilisateur qui n'en a plus.
fn mentionne_un_code(texte: &str) -> bool {
    texte
        .split(|c: char| !c.is_alphanumeric())
        .any(|mot| mot.eq_ignore_ascii_case("code") || mot.eq_ignore_ascii_case("codes"))
}

/// L'aide ne liste plus de sous-commande d'appairage.
#[test]
fn l_aide_ne_propose_plus_de_relier_par_code() {
    let maison = poste_neuf();
    let (succes, sortie, _) = lancer(&["--help"], &maison);

    assert!(succes, "`vibemap --help` doit reussir");
    assert!(
        !sortie.contains("pair"),
        "l'aide ne doit plus lister `pair`, obtenu :\n{sortie}"
    );
    assert!(
        !mentionne_un_code(&sortie),
        "l'aide ne doit plus parler d'un code, obtenu :\n{sortie}"
    );
    assert!(
        sortie.contains("application de bureau"),
        "l'aide doit dire d'ou vient la configuration, obtenu :\n{sortie}"
    );
}

/// L'ancienne commande ne relie plus rien, et le dit.
#[test]
fn la_sous_commande_d_appairage_ne_relie_plus_rien() {
    let maison = poste_neuf();
    let (succes, _, erreur) = lancer(&["pair", "7K4-M2Q"], &maison);

    assert!(!succes, "`vibemap pair` ne doit plus reussir");
    assert!(
        erreur.contains("application de bureau"),
        "le message doit renvoyer vers l'application de bureau, obtenu : {erreur}"
    );
    assert!(
        !mentionne_un_code(&erreur),
        "le message ne doit plus reclamer de code, obtenu : {erreur}"
    );
    assert!(
        !maison.join("vibemap").join("config.toml").exists(),
        "aucune configuration ne doit avoir ete ecrite : plus rien ne relie la machine"
    );
}

/// Le critere d'acceptation, de bout en bout : un Mac ou l'application n'a
/// jamais ete ouverte, le binaire lance nu.
#[test]
fn sur_un_poste_neuf_le_binaire_renvoie_vers_l_application() {
    let maison = poste_neuf();
    let (succes, _, erreur) = lancer(&[], &maison);

    assert!(
        !succes,
        "sans configuration, le lecteur ne doit pas demarrer"
    );
    assert!(
        erreur.contains("application de bureau"),
        "le message doit renvoyer vers l'application de bureau, obtenu : {erreur}"
    );
    assert!(
        !mentionne_un_code(&erreur),
        "le message ne doit plus renvoyer vers un code, obtenu : {erreur}"
    );
}

/* ---------- aucun message ne renvoie plus vers un code ---------- */

/// Chacune des erreurs qui nommait `vibemap pair <code>` nomme desormais
/// l'application de bureau.
#[test]
fn aucun_message_du_binaire_ne_renvoie_vers_un_code() {
    let messages = [
        vibemap::ConfigError::Introuvable(PathBuf::from("/tmp/config.toml")).to_string(),
        vibemap::ConfigError::JetonEnClair(PathBuf::from("/tmp/config.toml")).to_string(),
        vibemap::trousseau::TrousseauError::Absent("m-1".to_string()).to_string(),
        vibemap::ApiError::MachineInconnue("m-1".to_string()).to_string(),
    ];

    for message in messages {
        assert!(
            !mentionne_un_code(&message),
            "ce message renvoie encore vers un code : {message}"
        );
        assert!(
            !message.contains("vibemap pair"),
            "ce message renvoie encore vers une commande disparue : {message}"
        );
        assert!(
            message.contains("application de bureau"),
            "ce message doit nommer l'application de bureau : {message}"
        );
    }
}
