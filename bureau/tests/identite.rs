//! Comportement : l'identite du poste se reprend, ou se redeclare (issues #66
//! et #67 - FR-019, FR-021, FR-055 a FR-057, FR-073).
//!
//! ## Ce test ne tourne PAS dans l'integration continue
//!
//! Il parle a la vraie pile Supabase locale, comme `daemon/tests/*`, et il n'y
//! a pas de base la-bas. Ne pas l'ajouter a la liste de `.github/workflows/ci.yml` :
//! il y echouerait a chaque passage. Il se lance a la main avant chaque fusion :
//!
//! ```sh
//! supabase status                       # pour les deux cles
//! cd bureau && VIBEMAP_TEST_SERVICE_KEY=... VIBEMAP_TEST_ANON_KEY=... \
//!   cargo test --test identite
//! ```
//!
//! ## Ce qu'il verifie, et que rien d'autre ne verifie
//!
//! `bureau/tests/machine.rs` eprouve la DECISION du poste, sans base et sans
//! reseau. `daemon/tests/declaration.rs` eprouve ce que la base fait des
//! demandes qu'on lui adresse. Entre les deux, il reste le parcours complet -
//! lire la configuration du poste, interroger la base, ecrire ce qu'il faut - et
//! c'est lui qui porte les criteres d'acceptation des deux tranches : « la liste
//! des machines n'en gagne aucune nouvelle », « la carte se repeuple ».
//!
//! ## Le trousseau reste ferme
//!
//! `VIBEMAP_TOKEN` est pose partout ici : sans lui, `au_trousseau` irait ouvrir
//! le trousseau du poste qui fait tourner les tests, et macOS ouvrirait une
//! boite de dialogue. Meme precaution que `daemon/tests/appairage.rs`.
//!
//! Le RANGEMENT d'un jeton, lui, n'a pas d'echappatoire - et c'est bien ainsi :
//! le seul geste qui ecrit dans le trousseau ne doit pas pouvoir etre detourne
//! par une variable d'environnement. Aucun test d'ici ne le laisse donc
//! s'executer, et celui qui eprouve la redeclaration s'arrete juste avant (voir
//! son commentaire).

use std::path::PathBuf;

use bureau::machine::{assurer, EtatMachine};
use serde_json::{json, Value};

/* ---------- de quoi parler a la pile locale ---------- */

/// Un compte neuf, avec sa session ouverte. Chaque test a le sien : deux tests
/// ne peuvent pas se marcher dessus, et la liste des machines ne montre que
/// celles de ce compte-la - c'est ce qui donne son sens a « une seule machine ».
struct Contexte {
    url: String,
    cle_de_service: String,
    cle_anonyme: String,
    session: String,
    http: reqwest::Client,
}

impl Contexte {
    async fn nouveau() -> Self {
        // Sans cette pose, le premier `assurer` qui reprend une machine irait
        // ouvrir le trousseau du poste qui fait tourner les tests.
        std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");

        let url = std::env::var("VIBEMAP_TEST_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:54321".to_string());
        let cle_de_service = std::env::var("VIBEMAP_TEST_SERVICE_KEY").expect(
            "VIBEMAP_TEST_SERVICE_KEY manquante. Lance `supabase status` et exporte la cle service_role.",
        );
        let cle_anonyme = std::env::var("VIBEMAP_TEST_ANON_KEY").expect(
            "VIBEMAP_TEST_ANON_KEY manquante. Lance `supabase status` et exporte la cle anon.",
        );
        let http = reqwest::Client::new();

        // L'horloge ne suffit pas a distinguer deux comptes : les tests partent
        // de front, et macOS ne rend pas la nanoseconde. Le compteur, lui, ne
        // rend jamais deux fois la meme valeur.
        static COMPTEUR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let courriel = format!(
            "bureau-{}-{}-{}@test.vibemap.local",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("horloge")
                .as_nanos(),
            COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let mot_de_passe = "motdepasse-de-test-1234";

        let cree: Value = http
            .post(format!("{url}/auth/v1/admin/users"))
            .header("apikey", &cle_de_service)
            .bearer_auth(&cle_de_service)
            .json(&json!({
                "email": courriel,
                "password": mot_de_passe,
                "email_confirm": true,
            }))
            .send()
            .await
            .expect("creation de l'utilisateur de test")
            .json()
            .await
            .expect("reponse JSON de creation");
        assert!(
            cree["id"].is_string(),
            "pas d'utilisateur de test cree : {cree}"
        );

        let session: Value = http
            .post(format!("{url}/auth/v1/token?grant_type=password"))
            .header("apikey", &cle_de_service)
            .json(&json!({ "email": courriel, "password": mot_de_passe }))
            .send()
            .await
            .expect("ouverture de session")
            .json()
            .await
            .expect("reponse JSON de session");

        let jeton = session["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("pas de jeton dans {session}"))
            .to_string();

        Contexte {
            url,
            cle_de_service,
            cle_anonyme,
            session: jeton,
            http,
        }
    }

    /// Les machines que la SESSION de l'utilisateur voit, exactement comme la
    /// liste de l'accueil web. Jamais avec la cle de service : elle verrait
    /// celles de tous les comptes de la base de test, et « une seule machine »
    /// ne voudrait plus rien dire.
    async fn machines_visibles(&self) -> Vec<Value> {
        self.http
            .get(format!(
                "{}/rest/v1/machines?select=id,label,revoked_at&order=label",
                self.url
            ))
            .header("apikey", &self.cle_anonyme)
            .bearer_auth(&self.session)
            .send()
            .await
            .expect("lecture des machines de l'utilisateur")
            .json::<Value>()
            .await
            .expect("reponse JSON de lecture des machines")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Ce qu'a fait le binaire en ligne de commande avant que l'application
    /// n'existe : une machine sur le compte, et un jeton range au trousseau.
    async fn machine_deja_appairee(&self, label: &str) -> String {
        vibemap::declaration::declarer(&self.url, &self.session, label, Some("macos"))
            .await
            .expect("la machine de depart doit se declarer")
            .machine_id
    }

    /// Revoque une machine, comme le ferait le bouton des reglages.
    async fn revoquer(&self, machine_id: &str) {
        let reponse = self
            .http
            .patch(format!("{}/rest/v1/machines?id=eq.{machine_id}", self.url))
            .header("apikey", &self.cle_de_service)
            .bearer_auth(&self.cle_de_service)
            .header("Prefer", "return=representation")
            .json(&json!({ "revoked_at": "now()" }))
            .send()
            .await
            .expect("revocation de la machine");

        let code = reponse.status();
        let corps = reponse.text().await.unwrap_or_default();
        assert!(
            code.is_success(),
            "la revocation a echoue ({code}) : {corps}"
        );
        assert!(
            corps.trim() != "[]",
            "la revocation n'a touche aucune ligne : {corps}"
        );
    }

    /// Efface une machine, comme le ferait une base remise a zero.
    async fn effacer_machine(&self, machine_id: &str) {
        let reponse = self
            .http
            .delete(format!("{}/rest/v1/machines?id=eq.{machine_id}", self.url))
            .header("apikey", &self.cle_de_service)
            .bearer_auth(&self.cle_de_service)
            .send()
            .await
            .expect("effacement de la machine");
        assert!(
            reponse.status().is_success(),
            "l'effacement a echoue ({})",
            reponse.status()
        );
    }
}

/* ---------- le poste, tel que le binaire l'a laisse ---------- */

/// L'adresse que la configuration heritee designe : la pile locale de
/// developpement d'un autre poste, celle que FR-073 interdit de suivre.
const BASE_HERITEE: &str = "http://base-heritee.invalid";

fn dossier_de_test(quoi: &str) -> PathBuf {
    let dossier =
        std::env::temp_dir().join(format!("bureau-identite-{}-{quoi}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).expect("dossier de test");
    dossier
}

/// La configuration qu'a laissee `vibemap pair` sur un poste deja appaire :
/// une adresse de base locale, un identifiant de machine, deux dossiers
/// surveilles, une cadence reglee a la main et un commentaire.
fn config_de_ligne_de_commande(dossier: &std::path::Path, machine_id: &str) -> PathBuf {
    let chemin = dossier.join("config.toml");
    std::fs::write(
        &chemin,
        format!(
            "# ma configuration a moi\n\
             supabase_url = \"{BASE_HERITEE}\"\n\
             machine_id = \"{machine_id}\"\n\
             label = \"MacBook de Yarma\"\n\
             roots = [\"~/Developer\", \"~/Sites\"]\n\
             scan_seconds = 42\n"
        ),
    )
    .expect("configuration de ligne de commande");
    chemin
}

/* ---------- #67 : reprendre une machine deja appairee ---------- */

/// Le critere de l'issue #67, joue tel quel : un poste ou `vibemap` etait deja
/// appaire et surveillait `~/Developer` et `~/Sites`. On ouvre l'application
/// pour la premiere fois, et la liste des machines n'en gagne aucune.
#[tokio::test]
async fn un_poste_deja_appaire_se_reprend_sans_gagner_de_machine() {
    let ctx = Contexte::nouveau().await;
    let machine_id = ctx.machine_deja_appairee("MacBook de Yarma").await;

    let dossier = dossier_de_test("deja-appaire");
    let chemin = config_de_ligne_de_commande(&dossier, &machine_id);

    let etat = assurer(&chemin, &ctx.url, &ctx.session, "MacBook de Yarma", "macos").await;

    assert_eq!(
        etat,
        EtatMachine::Reprise {
            machine_id: machine_id.clone(),
            label: "MacBook de Yarma".to_string(),
        },
        "un poste deja appaire se reprend, il ne se declare pas"
    );

    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "la liste ne doit gagner aucune machine, obtenu : {machines:?}"
    );
    assert_eq!(machines[0]["id"], machine_id);

    // Le fichier existant n'a ete ni efface ni vide de ses dossiers (FR-057).
    let ecrit = std::fs::read_to_string(&chemin).expect("configuration relisible");
    assert!(
        ecrit.contains("~/Developer") && ecrit.contains("~/Sites"),
        "les dossiers surveilles restent, obtenu :\n{ecrit}"
    );
    assert!(
        ecrit.contains("# ma configuration a moi") && ecrit.contains("scan_seconds = 42"),
        "le commentaire et la cadence restent, obtenu :\n{ecrit}"
    );
    assert_eq!(
        ecrit.matches("~/Developer").count(),
        1,
        "aucun dossier n'est duplique, obtenu :\n{ecrit}"
    );

    // Et rouvrir encore ne cree toujours rien (FR-019).
    let deuxieme = assurer(&chemin, &ctx.url, &ctx.session, "MacBook de Yarma", "macos").await;
    assert!(matches!(deuxieme, EtatMachine::Reprise { .. }));
    assert_eq!(ctx.machines_visibles().await.len(), 1);

    let _ = std::fs::remove_dir_all(&dossier);
}

/// FR-073 : l'application parle a la base de sa compilation, et le lecteur
/// qu'elle embarque aussi.
///
/// La premiere moitie se lit dans le test precedent - la machine a ete retrouvee
/// dans la base qu'on a passee a `assurer`, alors que le fichier en designait une
/// autre. La seconde est ici : `supabase_url` est le SEUL endroit ou le lecteur
/// lise une adresse. Le laisser pointer sur la pile de developpement ferait
/// ecrire les cartes dans une base pendant que la fenetre en lirait une autre.
#[tokio::test]
async fn la_base_heritee_ne_detourne_ni_la_fenetre_ni_le_lecteur() {
    let ctx = Contexte::nouveau().await;
    let machine_id = ctx.machine_deja_appairee("MacBook de Yarma").await;

    let dossier = dossier_de_test("base-heritee");
    let chemin = config_de_ligne_de_commande(&dossier, &machine_id);

    let etat = assurer(&chemin, &ctx.url, &ctx.session, "MacBook de Yarma", "macos").await;
    assert!(
        matches!(etat, EtatMachine::Reprise { .. }),
        "obtenu : {etat:?}"
    );

    let ecrit = std::fs::read_to_string(&chemin).expect("configuration relisible");
    assert!(
        !ecrit.contains(BASE_HERITEE),
        "l'adresse heritee ne doit plus etre lisible par le lecteur, obtenu :\n{ecrit}"
    );
    assert!(
        ecrit.contains(&ctx.url),
        "le lecteur doit lire la base de l'application, obtenu :\n{ecrit}"
    );

    // Et c'est bien la configuration du lecteur : il la relit telle quelle.
    let config =
        vibemap::Config::load(&chemin).expect("le lecteur doit relire cette configuration");
    assert_eq!(config.supabase_url, ctx.url);
    assert_eq!(config.machine_id, machine_id);
    assert_eq!(config.roots, vec!["~/Developer", "~/Sites"]);

    let _ = std::fs::remove_dir_all(&dossier);
}

/* ---------- #66 : redeclarer, ou se taire ---------- */

/// FR-021 : la machine a ete revoquee depuis le web. L'application l'annonce, et
/// n'inscrit AUCUNE machine de remplacement.
///
/// C'est le critere le plus lourd de consequences des deux tranches : redeclarer
/// ici defairait la revocation - machine neuve, jeton neuf, battement repris -
/// par le seul fait de rouvrir l'application.
#[tokio::test]
async fn une_machine_revoquee_s_annonce_et_n_inscrit_aucune_remplacante() {
    let ctx = Contexte::nouveau().await;
    let machine_id = ctx.machine_deja_appairee("MacBook de Yarma").await;
    ctx.revoquer(&machine_id).await;

    let dossier = dossier_de_test("revoquee");
    let chemin = config_de_ligne_de_commande(&dossier, &machine_id);

    // Trois lancements : « quand j'attends, alors l'application ne cree aucune
    // machine de remplacement ».
    for tour in 1..=3 {
        let etat = assurer(&chemin, &ctx.url, &ctx.session, "MacBook de Yarma", "macos").await;
        assert_eq!(
            etat,
            EtatMachine::Revoquee {
                machine_id: machine_id.clone(),
                label: "MacBook de Yarma".to_string(),
            },
            "au lancement {tour}, la revocation doit s'annoncer telle quelle"
        );

        let machines = ctx.machines_visibles().await;
        assert_eq!(
            machines.len(),
            1,
            "au lancement {tour}, aucune machine de remplacement, obtenu : {machines:?}"
        );
        assert!(
            machines[0]["revoked_at"].is_string(),
            "la seule machine du compte reste la revoquee, obtenu : {machines:?}"
        );
    }

    // Et le poste garde son identite : rien n'a ete reecrit dans son fichier.
    let ecrit = std::fs::read_to_string(&chemin).expect("configuration relisible");
    assert!(ecrit.contains(&machine_id), "obtenu :\n{ecrit}");

    let _ = std::fs::remove_dir_all(&dossier);
}

/// FR-056 : l'identifiant conserve ne designe plus rien, et la machine est
/// inscrite a nouveau - une fois, et une seule.
///
/// ## Pourquoi ce test s'arrete a un `Echec`
///
/// Le parcours reel finit par ranger le jeton neuf au trousseau du systeme, et
/// aucun test ne doit ecrire dans le trousseau du poste qui le fait tourner.
/// Le dossier de configuration est donc rendu impossible a ecrire : la machine
/// est bien inscrite dans la base - c'est ce qu'on veut prouver -, puis
/// `poser_l_identite` echoue, et `assurer` s'arrete la, avant le trousseau.
///
/// Ce qui est verifie ici est donc precisement ce qui manquerait ailleurs : que
/// l'inscription a EU LIEU pour une identite disparue, et une seule fois. Que
/// l'etat rendu s'appelle ensuite `Redeclaree` se lit dans
/// `bureau/tests/machine.rs`, ou l'ordre des trois etapes est fige.
#[tokio::test]
async fn une_identite_disparue_fait_inscrire_une_machine_et_une_seule() {
    let ctx = Contexte::nouveau().await;
    let perdue = ctx.machine_deja_appairee("MacBook de Yarma").await;
    ctx.effacer_machine(&perdue).await;

    let dossier = dossier_de_test("disparue");
    let chemin = config_de_ligne_de_commande(&dossier, &perdue);
    interdire_l_ecriture(&dossier);

    let etat = assurer(&chemin, &ctx.url, &ctx.session, "MacBook de Yarma", "macos").await;

    // L'arret attendu : la machine est inscrite, la configuration ne s'ecrit
    // pas, et le poste le dit plutot que d'aller ouvrir le trousseau.
    let EtatMachine::Echec { raison } = &etat else {
        rendre_l_ecriture(&dossier);
        let _ = std::fs::remove_dir_all(&dossier);
        panic!(
            "un dossier de configuration impossible a ecrire doit s'annoncer, obtenu : {etat:?}"
        );
    };
    assert!(
        raison.contains("config.toml"),
        "le message doit nommer le fichier qui resiste, obtenu : {raison}"
    );

    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "une identite disparue fait inscrire UNE machine, obtenu : {machines:?}"
    );
    assert_ne!(
        machines[0]["id"].as_str(),
        Some(perdue.as_str()),
        "la machine inscrite porte une identite neuve, obtenu : {machines:?}"
    );
    assert!(
        machines[0]["revoked_at"].is_null(),
        "la machine inscrite emet, obtenu : {machines:?}"
    );

    rendre_l_ecriture(&dossier);
    let _ = std::fs::remove_dir_all(&dossier);
}

/// Rend le dossier impossible a ecrire, sans toucher a ce qu'il contient : la
/// configuration se relit, et le fichier voisin de l'ecriture atomique ne peut
/// pas y naitre.
fn interdire_l_ecriture(dossier: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dossier, std::fs::Permissions::from_mode(0o500))
        .expect("dossier de test en lecture seule");
}

fn rendre_l_ecriture(dossier: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(dossier, std::fs::Permissions::from_mode(0o700));
}
