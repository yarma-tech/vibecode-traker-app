//! Ce que la commande de pont « lire le contexte » rend du poste : la version
//! qu'on execute, le nom de cette machine, et ou en est le depouillement du
//! passe (issues #81, #85 - FR-050, FR-065, FR-068).
//!
//! Deux promesses s'eprouvent ici.
//!
//! La premiere : les quatre etats du depouillement ne se confondent PAS. « Rien
//! a depouiller » n'est pas « zero sur zero », « jamais commence » n'est pas
//! « termine », et une reprise repart de son avancement au lieu de retomber a
//! zero. C'est ce dernier point qui rend observable que le depouillement REPREND
//! au lieu de recommencer - la promesse de FR-049 et FR-078, invisible partout
//! ailleurs.
//!
//! La seconde : la forme rendue. C'est le contrat du pont, et un nom de champ
//! qui bouge laisserait la section muette sans que rien ne casse ailleurs. Les
//! marques ecrites ici le sont a la main, dans le format exact que le lecteur
//! pose sur le disque (`daemon/src/depouillement.rs`) : c'est ce qui fait de ces
//! tests le pendant lecteur de ce que le depouillement ecrit.
//!
//! Toujours sur des dossiers temporaires : un test ne doit rien devoir a ce qui
//! vit sur la machine, ni toucher a la configuration de l'utilisateur.

use std::path::{Path, PathBuf};

use bureau::contexte::{contexte, depouillement, Depouillement};
use bureau::version::VERSION;

fn bac_a_sable(quoi: &str) -> PathBuf {
    let chemin =
        std::env::temp_dir().join(format!("bureau-contexte-{}-{quoi}", std::process::id()));
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).expect("bac a sable de test");
    chemin
}

/// Une configuration de lecteur qui nomme la machine.
fn config_nommee(bac: &Path, label: &str) -> PathBuf {
    let chemin = bac.join("config.toml");
    std::fs::write(
        &chemin,
        format!(
            "supabase_url = \"http://127.0.0.1:1\"\n\
             machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
             label = \"{label}\"\n"
        ),
    )
    .expect("ecriture de la configuration de test");
    chemin
}

/// La marque du depouillement, telle que le lecteur l'ecrit a cote de la
/// configuration.
///
/// Ecrite a la main, et c'est voulu : ce sont les noms de champs de
/// `daemon/src/depouillement.rs` qui font le contrat entre celui qui depouille
/// et l'ecran qui le regarde. Un champ renomme d'un cote se voit ici.
fn marque(config: &Path, contenu: serde_json::Value) {
    std::fs::write(
        vibemap::depouillement::chemin_de_la_marque(config),
        contenu.to_string(),
    )
    .expect("ecriture de la marque de test");
}

/* ---------- les quatre etats, et ils ne se confondent pas (FR-050) ---------- */

/// Un depouillement en cours rend ses DEUX nombres : l'avancement sur le total.
#[test]
fn un_depouillement_en_cours_rend_son_avancement_sur_le_total() {
    let bac = bac_a_sable("en-cours");
    let config = config_nommee(&bac, "machine de test");
    marque(
        &config,
        serde_json::json!({
            "termine_a": null,
            "depouille_jusqu_a": "2026-08-19T10:00:00Z",
            "journaux": 120,
            "total": 400,
        }),
    );

    assert_eq!(
        depouillement(&config),
        Depouillement::EnCours {
            journaux: 120,
            total: 400
        }
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un depouillement termine dit QUAND il s'est termine, et non un avancement.
#[test]
fn un_depouillement_termine_dit_quand_il_s_est_termine() {
    let bac = bac_a_sable("termine");
    let config = config_nommee(&bac, "machine de test");
    marque(
        &config,
        serde_json::json!({
            "termine_a": "2026-08-19T14:32:10Z",
            "depouille_jusqu_a": "2026-08-19T14:32:10Z",
            "journaux": 400,
            "total": 400,
        }),
    );

    let Depouillement::Termine { quand, journaux } = depouillement(&config) else {
        panic!("obtenu : {:?}", depouillement(&config));
    };
    assert_eq!(journaux, 400);
    assert!(
        quand.starts_with("2026-08-19T14:32:10"),
        "l'heure de fin doit etre celle que la marque porte, dans une forme que la fenetre \
         sait lire ; obtenu : {quand}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Le Mac ou Claude Code n'a jamais tourne : il n'y a RIEN a depouiller
/// (FR-065).
///
/// Le passage a bien abouti, et il n'a trouve aucun journal. L'annoncer comme un
/// avancement - « 0 sur 0 » - se lirait comme un depouillement qui ne bouge pas,
/// et enverrait chercher une panne la ou il n'y en a pas.
#[test]
fn un_poste_sans_aucun_journal_dit_qu_il_n_y_a_rien_a_depouiller() {
    let bac = bac_a_sable("rien");
    let config = config_nommee(&bac, "machine de test");
    marque(
        &config,
        serde_json::json!({
            "termine_a": "2026-08-19T14:32:10Z",
            "depouille_jusqu_a": null,
            "journaux": 0,
            "total": 0,
        }),
    );

    let vu = depouillement(&config);
    assert_eq!(vu, Depouillement::RienADepouiller);
    assert!(
        !matches!(vu, Depouillement::EnCours { .. }),
        "« rien a depouiller » ne doit jamais se donner pour un avancement"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Sans marque sur le disque, personne n'a encore rien depouille ici.
///
/// « Jamais commence » et « termine » ne se corrigent pas de la meme facon : le
/// premier attend le lecteur, le second n'attend plus rien.
#[test]
fn un_depouillement_jamais_commence_ne_se_donne_pas_pour_termine() {
    let bac = bac_a_sable("jamais");
    let config = config_nommee(&bac, "machine de test");

    assert_eq!(depouillement(&config), Depouillement::Jamais);

    std::fs::remove_dir_all(&bac).ok();
}

/// Une marque a moitie ecrite ne fait pas paniquer l'ecran : elle se lit comme
/// une marque absente, et l'application reste utilisable.
#[test]
fn une_marque_illisible_ne_fait_pas_tomber_l_ecran() {
    let bac = bac_a_sable("corrompue");
    let config = config_nommee(&bac, "machine de test");
    std::fs::write(
        vibemap::depouillement::chemin_de_la_marque(&config),
        "{ ceci n'est pas du JSON",
    )
    .expect("marque corrompue");

    assert_eq!(depouillement(&config), Depouillement::Jamais);

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- une reprise repart de son avancement (FR-049, FR-078) ---------- */

/// Le passage qui reprend part de ce qui a deja ete depouille, jamais de zero.
///
/// C'est LE fait que cette tranche rend observable. Quatre cents journaux
/// depouilles hier, vingt ecrits depuis : l'ecran doit lire 400 sur 420, et non
/// 0 sur 20 - qui donnerait a croire que tout est a refaire - ni « termine »,
/// qui parlerait du passage d'hier alors qu'un autre est en train de courir.
#[test]
fn une_reprise_repart_de_son_avancement_et_non_de_zero() {
    let bac = bac_a_sable("reprise");
    let config = config_nommee(&bac, "machine de test");
    marque(
        &config,
        serde_json::json!({
            // L'heure de fin est celle du passage d'HIER : elle est encore la,
            // et elle ne dit rien du passage en cours.
            "termine_a": "2026-08-18T14:32:10Z",
            "depouille_jusqu_a": "2026-08-19T09:00:00Z",
            "journaux": 400,
            "total": 420,
        }),
    );

    assert_eq!(
        depouillement(&config),
        Depouillement::EnCours {
            journaux: 400,
            total: 420
        },
        "une reprise doit se lire depuis son avancement, et non repartir de zero \
         ni se dire terminee"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- la version et le nom de la machine (FR-068, FR-020) ---------- */

/// La version rendue par le pont est celle du manifeste du paquet, jamais un
/// numero saisi a la main.
#[test]
fn le_pont_rend_la_version_du_manifeste_du_paquet() {
    let bac = bac_a_sable("version");
    let config = config_nommee(&bac, "machine de test");

    let manifeste: toml_edit::DocumentMut =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("le manifeste du paquet")
            .parse()
            .expect("manifeste lisible");
    let declaree = manifeste["package"]["version"]
        .as_str()
        .expect("bureau/Cargo.toml declare package.version");

    assert_eq!(
        contexte(&config).version,
        declaree,
        "la version que le pont rend doit etre celle de bureau/Cargo.toml"
    );
    assert_eq!(contexte(&config).version, VERSION);

    std::fs::remove_dir_all(&bac).ok();
}

/// Le nom de la machine est celui que la configuration porte - donc celui qui
/// est parti avec la declaration, et que la liste des machines affiche
/// (issue #65).
#[test]
fn le_nom_de_la_machine_est_celui_que_porte_la_declaration() {
    let bac = bac_a_sable("nom");
    let config = config_nommee(&bac, "MacBook de Yarma");

    assert_eq!(
        contexte(&config).machine.as_deref(),
        Some("MacBook de Yarma")
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Sans configuration lisible, l'ecran se tait sur le nom - et dit tout le
/// reste.
///
/// Un nom invente a cette place se lirait comme le nom du poste. La version, qui
/// ne doit rien a la configuration, continue elle de s'afficher : c'est meme le
/// moment ou l'on cherche a savoir ce qu'on execute.
#[test]
fn une_configuration_absente_ne_donne_aucun_nom_et_n_empeche_pas_le_reste() {
    let bac = bac_a_sable("sans-config");
    let config = bac.join("jamais-ecrite.toml");

    let vu = contexte(&config);
    assert_eq!(vu.machine, None);
    assert!(!vu.version.is_empty());
    assert_eq!(vu.depouillement, Depouillement::Jamais);

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- la forme que lit l'ecran ---------- */

/// Elle compte autant que le fond : c'est le contrat du pont.
#[test]
fn le_contexte_se_lit_tel_quel_dans_la_fenetre() {
    let bac = bac_a_sable("forme");
    let config = config_nommee(&bac, "MacBook de Yarma");
    marque(
        &config,
        serde_json::json!({
            "termine_a": null,
            "depouille_jusqu_a": "2026-08-19T10:00:00Z",
            "journaux": 12,
            "total": 400,
        }),
    );

    let rendu = serde_json::to_value(contexte(&config)).expect("contexte serialisable");
    assert_eq!(rendu["machine"], "MacBook de Yarma");
    assert_eq!(rendu["version"], VERSION);
    assert_eq!(rendu["depouillement"]["etat"], "en_cours");
    assert_eq!(rendu["depouillement"]["journaux"], 12);
    assert_eq!(rendu["depouillement"]["total"], 400);

    marque(
        &config,
        serde_json::json!({
            "termine_a": "2026-08-19T14:32:10Z",
            "depouille_jusqu_a": null,
            "journaux": 0,
            "total": 0,
        }),
    );
    let rendu = serde_json::to_value(contexte(&config)).expect("contexte serialisable");
    assert_eq!(
        rendu["depouillement"]["etat"], "rien_a_depouiller",
        "l'ecran doit pouvoir distinguer « rien a depouiller » d'un avancement"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// La commande est ouverte a la fenetre, sans quoi l'ecran n'a personne a qui
/// demander.
///
/// Un module qui rend le contexte sans commande qui l'expose ne se voit nulle
/// part : rien ne casse a la compilation, et la section reste muette dans
/// l'application.
#[test]
fn la_commande_est_ouverte_a_la_fenetre() {
    let main = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
        .expect("le fil principal de l'application");
    let (_, apres) = main
        .split_once("tauri::generate_handler![")
        .expect("l'application declare les commandes de son pont");
    let (handler, _) = apres
        .split_once(']')
        .expect("la liste des commandes se ferme");

    assert!(
        handler.contains("contexte_du_poste"),
        "« lire le contexte » doit figurer parmi les commandes du pont, obtenu : {handler}"
    );
}
