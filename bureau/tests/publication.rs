//! Ce que la publication porte, et ce qu'elle ne porte plus (FR-067, FR-069,
//! FR-079).
//!
//! Trois promesses s'eprouvent ici, et aucune ne se voit depuis le code seul :
//! elles vivent dans les manifestes du depot.
//!
//! La premiere : **un seul numero de version**. Le paquet, le lecteur qu'il
//! embarque et le service d'interface qu'il sert annoncent le meme numero, sans
//! qu'aucun ne soit saisi deux fois. C'est ce qui permet a l'ecran Reglages
//! (#85) de dire ce qu'on execute quand quelque chose cloche.
//!
//! La deuxieme : **l'empaquetage est arme**. Il a ete desactive tant que
//! l'application ne se publiait pas ; un paquet qu'on ne sait pas produire
//! n'existe pas, et un `bundle.active` remis a `false` par megarde rendrait la
//! release muette sans casser une seule compilation.
//!
//! La troisieme : **la publication ne porte plus Linux**. Retrait de portee
//! assume par FR-079, parce que le chemin d'appairage par code disparait
//! (FR-082) et qu'aucun poste Linux neuf ne pourrait plus franchir l'entree.
//! Ce test encode cette decision ; si le PM la refuse, c'est lui qu'on defait
//! en meme temps que le reste.
//!
//! Ces trois-la regardent hors de `bureau/` - la formule Homebrew, le workflow
//! de publication, le manifeste de l'interface. C'est assume : ce sont les
//! seuls endroits ou la promesse s'ecrit, et `cargo test` est le seul harnais
//! qui la garde.

use std::path::{Path, PathBuf};

use bureau::version::VERSION;

/// La racine du depot, voisine de `bureau/`.
fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bureau/ a un parent")
        .to_path_buf()
}

fn lire(chemin: &Path) -> String {
    std::fs::read_to_string(chemin).unwrap_or_else(|erreur| {
        panic!("{} doit etre lisible : {erreur}", chemin.display());
    })
}

/// La version declaree par un manifeste Cargo.
fn version_cargo(chemin: &Path) -> String {
    let manifeste: toml_edit::DocumentMut = lire(chemin).parse().unwrap_or_else(|erreur| {
        panic!("{} doit etre un TOML valide : {erreur}", chemin.display())
    });
    manifeste["package"]["version"]
        .as_str()
        .unwrap_or_else(|| panic!("{} doit declarer package.version", chemin.display()))
        .to_string()
}

/// La configuration du paquet Tauri.
fn config_tauri() -> serde_json::Value {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    serde_json::from_str(&lire(&chemin)).expect("tauri.conf.json doit etre un JSON valide")
}

// --- Un seul numero de version -------------------------------------------

/// La version que l'application rend est celle de son propre manifeste : elle
/// n'est ecrite nulle part a la main.
#[test]
fn la_version_rendue_est_celle_du_manifeste_du_paquet() {
    let manifeste = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert_eq!(
        VERSION,
        version_cargo(&manifeste),
        "la version rendue par l'application doit etre celle de bureau/Cargo.toml"
    );
}

/// `tauri.conf.json` ne porte deliberement aucun champ `version` : Tauri reprend
/// alors celle de `Cargo.toml`. Un numero ecrit ici serait un second endroit ou
/// le corriger, donc un endroit ou l'oublier - le paquet telecharge annoncerait
/// une version, l'ecran Reglages une autre.
#[test]
fn le_paquet_ne_redeclare_pas_la_version() {
    let config = config_tauri();
    match config.get("version") {
        None => {}
        Some(declaree) => assert_eq!(
            declaree.as_str(),
            Some(VERSION),
            "tauri.conf.json redeclare la version : retire ce champ, ou tiens-le \
             egal a celui de bureau/Cargo.toml"
        ),
    }
}

/// Le lecteur est embarque, jamais telecharge (FR-069) : le paquet et lui sont
/// forcement du meme jour.
#[test]
fn le_lecteur_embarque_porte_la_meme_version() {
    let manifeste = depot().join("daemon").join("Cargo.toml");
    assert_eq!(
        VERSION,
        version_cargo(&manifeste),
        "le lecteur embarque doit porter la version du paquet : aligne \
         daemon/Cargo.toml sur bureau/Cargo.toml"
    );
}

/// Le service d'interface est servi par l'application, pas chez un hebergeur :
/// il suit la meme version qu'elle (FR-069).
#[test]
fn le_service_d_interface_porte_la_meme_version() {
    let manifeste = depot().join("web").join("package.json");
    let paquet: serde_json::Value =
        serde_json::from_str(&lire(&manifeste)).expect("web/package.json doit etre un JSON valide");
    assert_eq!(
        paquet.get("version").and_then(|v| v.as_str()),
        Some(VERSION),
        "le service d'interface doit porter la version du paquet : aligne \
         web/package.json sur bureau/Cargo.toml"
    );
}

// --- L'empaquetage est arme ----------------------------------------------

/// Sans empaquetage, la release ne porte rien a telecharger et le lien du
/// premier lancement (FR-058) vise le vide.
#[test]
fn l_empaquetage_est_actif() {
    let config = config_tauri();
    assert_eq!(
        config["bundle"]["active"].as_bool(),
        Some(true),
        "l'empaquetage doit rester actif : sans lui, l'etiquette ne publie \
         aucune application"
    );
}

/// Une seule cible macOS, et son icone au format que le systeme sait afficher.
/// Une icone absente ne casse aucune compilation : elle donne une application
/// sans visage dans le Dock.
#[test]
fn le_paquet_porte_son_icone_macos() {
    let config = config_tauri();
    let icones = config["bundle"]["icon"]
        .as_array()
        .expect("bundle.icon doit etre une liste");
    let icns = icones
        .iter()
        .filter_map(|icone| icone.as_str())
        .find(|icone| icone.ends_with(".icns"))
        .expect("bundle.icon doit porter une icone .icns : c'est celle que macOS affiche");
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join(icns);
    assert!(
        chemin.is_file(),
        "l'icone {} annoncee par tauri.conf.json doit exister",
        chemin.display()
    );
}

// --- La publication ne porte plus Linux ----------------------------------

/// La formule ne sert plus qu'un Mac Apple Silicon. Son volet Linux pointait une
/// archive que la publication ne produit plus : il aurait renvoye l'utilisateur
/// sur un lien mort.
#[test]
fn la_formule_homebrew_ne_sert_plus_que_macos() {
    let formule = lire(&depot().join("Formula").join("vibemap.rb"));
    assert!(
        !formule.contains("on_linux"),
        "Formula/vibemap.rb ne doit plus resoudre d'installation Linux (FR-079)"
    );
    assert!(
        !formule.contains("unknown-linux"),
        "Formula/vibemap.rb ne doit plus referencer d'archive Linux (FR-079)"
    );
    assert!(
        formule.contains("on_macos"),
        "Formula/vibemap.rb doit continuer de servir macOS"
    );
    assert!(
        formule.contains("aarch64-apple-darwin"),
        "Formula/vibemap.rb doit viser l'archive Apple Silicon"
    );
}

/// La chaine de publication ne compile plus la cible Linux. Le jour ou le PM
/// refuse ce retrait de portee, ce test tombe avec le reste - c'est voulu.
#[test]
fn la_publication_ne_compile_plus_pour_linux() {
    let workflow = lire(
        &depot()
            .join(".github")
            .join("workflows")
            .join("release.yml"),
    );
    assert!(
        !workflow.contains("unknown-linux"),
        ".github/workflows/release.yml ne doit plus compiler de cible Linux (FR-079)"
    );
    assert!(
        workflow.contains("aarch64-apple-darwin"),
        ".github/workflows/release.yml doit continuer de publier la cible Apple Silicon"
    );
}
