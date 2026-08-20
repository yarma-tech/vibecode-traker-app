//! D'ou l'interface est servie (FR-069).
//!
//! Une seule question decide de tout : l'application tourne-t-elle depuis un
//! paquet, ou depuis le depot ? Le paquet doit servir ce qu'il embarque - un
//! poste qui l'a telecharge n'a pas de depot, et un chemin fige a la
//! compilation n'y designe rien. Le depot, lui, doit continuer d'etre servi
//! comme avant : la voie de developpement n'est pas sacrifiee a l'autre.
//!
//! Ce choix se prend sur des chemins, sans rien lire sur le disque : c'est ce
//! qui permet de l'eprouver ici, sans paquet a construire.
//!
//! Les deux derniers tests regardent hors du code, dans `tauri.conf.json` et
//! `web/next.config.ts`. C'est assume : ce que l'application va chercher dans
//! ses ressources n'y est que si l'empaquetage l'y depose, et rien d'autre ne
//! garde ce rendez-vous. Le jour ou l'un des deux bouge seul, le paquet
//! s'ouvre sur sa page d'indisponibilite - et cela ne se voit qu'en
//! telechargeant une release.

use std::path::{Path, PathBuf};

use bureau::service::{ressources_du_paquet, vehicule, Vehicule};

/// Le depot, voisin de `bureau/`.
fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bureau/ a un parent")
        .to_path_buf()
}

/// L'executable tel que macOS le range dans une application installee.
fn executable_du_paquet() -> PathBuf {
    PathBuf::from("/Applications/Vibe Map.app/Contents/MacOS/Vibe Map")
}

/// L'executable tel que `cargo run` le produit.
fn executable_de_compilation() -> PathBuf {
    depot().join("bureau/target/debug/bureau")
}

// --- Reconnaitre un paquet -----------------------------------------------

/// macOS range l'executable dans `Contents/MacOS` et ce que l'application
/// embarque dans `Contents/Resources`. C'est cette forme, et elle seule, qui
/// signale un paquet.
#[test]
fn un_executable_de_paquet_designe_les_ressources_de_l_application() {
    assert_eq!(
        ressources_du_paquet(&executable_du_paquet()),
        Some(PathBuf::from(
            "/Applications/Vibe Map.app/Contents/Resources"
        )),
    );
}

/// Un binaire de compilation n'est dans aucun paquet : il n'a pas de
/// ressources a offrir, et c'est ce qui renvoie l'application vers le depot.
#[test]
fn un_executable_de_compilation_ne_designe_aucune_ressource() {
    assert_eq!(ressources_du_paquet(&executable_de_compilation()), None);
}

/// Un dossier qui porte les bons noms sans etre un paquet ne trompe personne,
/// mais un dossier qui n'en porte qu'un ne doit pas passer pour un paquet.
#[test]
fn un_dossier_qui_ressemble_a_moitie_ne_passe_pas_pour_un_paquet() {
    assert_eq!(
        ressources_du_paquet(Path::new("/tmp/MacOS/bureau")),
        None,
        "sans `Contents` au-dessus, ce n'est pas un paquet"
    );
    assert_eq!(
        ressources_du_paquet(Path::new("/tmp/Contents/bureau")),
        None,
        "sans `MacOS` juste au-dessus, ce n'est pas un paquet"
    );
    assert_eq!(
        ressources_du_paquet(Path::new("bureau")),
        None,
        "un executable sans dossier parent n'est pas un paquet"
    );
}

// --- Choisir la voie ------------------------------------------------------

/// Le paquet sert ce qu'il embarque : le serveur autonome et le Node qui le
/// fait tourner, tous deux dans ses ressources.
#[test]
fn le_paquet_sert_l_interface_qu_il_embarque() {
    let choisi = vehicule(&executable_du_paquet(), &depot().join("web"));

    let racine = PathBuf::from("/Applications/Vibe Map.app/Contents/Resources/service");
    assert_eq!(
        choisi,
        Vehicule::Embarque {
            node: racine.join("node"),
            racine,
        },
    );
}

/// Rien de ce que le paquet sert ne passe par le depot. C'est le defaut que
/// cette tranche corrige : un poste qui telecharge l'application n'a pas de
/// depot, et le chemin fige a la compilation n'y designe rien.
#[test]
fn le_paquet_ne_va_rien_chercher_dans_le_depot() {
    let choisi = vehicule(&executable_du_paquet(), &depot().join("web"));

    let Vehicule::Embarque { racine, node } = choisi else {
        panic!("un executable de paquet doit servir l'interface embarquee");
    };
    for chemin in [&racine, &node] {
        assert!(
            !chemin.starts_with(depot()),
            "{} passe par le depot, que le poste qui telecharge n'a pas",
            chemin.display()
        );
    }
}

/// La voie de developpement reste : lancee depuis le depot, l'application sert
/// le `web/` du depot, comme avant cette tranche.
#[test]
fn le_depot_reste_servi_depuis_le_depot() {
    let web = depot().join("web");
    assert_eq!(
        vehicule(&executable_de_compilation(), &web),
        Vehicule::Depot { racine: web },
    );
}

// --- Le rendez-vous avec l'empaquetage ------------------------------------

/// L'application cherche l'interface dans `Contents/Resources/service` ;
/// l'empaquetage doit l'y deposer. Les deux noms sont ecrits a deux endroits,
/// et rien d'autre ne les tient ensemble.
#[test]
fn le_paquet_depose_le_service_la_ou_l_application_le_cherche() {
    let bureau = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(bureau.join("tauri.conf.json")).unwrap())
            .expect("tauri.conf.json doit etre un JSON valide");

    assert_eq!(
        config["bundle"]["resources"]["service-embarque"].as_str(),
        Some("service"),
        "tauri.conf.json doit embarquer `service-embarque` sous `service` : \
         c'est la que src/service.rs va chercher l'interface"
    );

    let avant = config["build"]["beforeBuildCommand"]
        .as_str()
        .expect("l'empaquetage doit assembler l'interface avant de bundler");
    // Le chemin porte `bureau/` : l'empaquetage lance cette commande depuis la
    // racine du depot, pas depuis `bureau/`.
    assert!(
        avant.contains("bureau/embarquer-le-service.sh"),
        "l'empaquetage doit appeler bureau/embarquer-le-service.sh, sinon \
         `service-embarque` est vide ou perime"
    );
    assert!(
        bureau.join("embarquer-le-service.sh").is_file(),
        "le script d'assemblage annonce par tauri.conf.json doit exister"
    );
}

/// Sans sortie autonome, il n'y a pas de `server.js` a embarquer : il faudrait
/// recopier tout `node_modules` et un `npm` dans le paquet.
///
/// L'interface ne sort ainsi que sur demande, pour laisser le site heberge et
/// la voie de developpement a leur `next start`. Le nom de la demande est donc
/// ecrit des deux cotes, et ces deux-la doivent se repondre.
#[test]
fn l_interface_sort_en_service_autonome_quand_l_empaquetage_le_demande() {
    const DEMANDE: &str = "VIBEMAP_SERVICE_EMBARQUE";

    let config = std::fs::read_to_string(depot().join("web/next.config.ts"))
        .expect("web/next.config.ts doit etre lisible");
    assert!(
        config.contains("\"standalone\"") && config.contains(DEMANDE),
        "web/next.config.ts doit sortir en \"standalone\" quand {DEMANDE} est \
         pose : c'est ce que le paquet embarque"
    );

    let script = std::fs::read_to_string(depot().join("bureau/embarquer-le-service.sh"))
        .expect("bureau/embarquer-le-service.sh doit etre lisible");
    assert!(
        script.contains(DEMANDE),
        "embarquer-le-service.sh doit poser {DEMANDE} avant de construire \
         l'interface, sinon il n'assemble rien"
    );
}
