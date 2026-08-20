//! Le pont des commandes locales : a qui il est ouvert, et ce qu'il rend des
//! dossiers surveilles.
//!
//! Deux promesses s'eprouvent ici, et elles ne tiennent qu'ensemble.
//!
//! La premiere : le pont repond a l'interface que l'application sert elle-meme.
//! Sans cela, l'ecran Reglages ne saurait rien des faits du poste, et cinq
//! tranches suivantes n'auraient pas de porte. La seconde : ce que ce pont rend
//! d'un dossier surveille - son compte de depots au critere de la cartographie,
//! et ce qui l'empeche d'etre lu quand il ne l'est pas.
//!
//! Toujours sur des dossiers temporaires : un test ne doit rien devoir a ce qui
//! vit sur la machine, ni toucher a la configuration de l'utilisateur.

use std::path::{Path, PathBuf};

use bureau::dossiers::{dossiers_surveilles, regarder, Lisibilite, Surveillance};
use bureau::sonde::url_de_la_fenetre;

fn bac_a_sable(quoi: &str) -> PathBuf {
    let chemin = std::env::temp_dir().join(format!("bureau-pont-{}-{quoi}", std::process::id()));
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).expect("bac a sable de test");
    chemin
}

/// Un enfant direct qui porte un `.git` : ce que la cartographie appelle un
/// depot.
fn depot(parent: &Path, nom: &str) {
    std::fs::create_dir_all(parent.join(nom).join(".git")).expect("faux depot");
}

/// Un enfant direct qui n'en porte pas : la cartographie passe a cote, et le
/// compte aussi.
fn dossier_ordinaire(parent: &Path, nom: &str) {
    std::fs::create_dir_all(parent.join(nom)).expect("dossier ordinaire");
}

/// Une configuration de lecteur qui surveille les dossiers donnes.
fn config_qui_surveille(bac: &Path, racines: &[&str]) -> PathBuf {
    let chemin = bac.join("config.toml");
    let roots = racines
        .iter()
        .map(|racine| format!("\"{racine}\""))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        &chemin,
        format!(
            "supabase_url = \"http://127.0.0.1:1\"\n\
             machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
             label = \"machine de test\"\n\
             roots = [{roots}]\n"
        ),
    )
    .expect("ecriture de la configuration de test");
    chemin
}

/// Le pont est ouvert a l'interface que l'application sert, et a elle seule.
///
/// C'est la piece d'infrastructure de cette tranche : la fenetre charge cette
/// origine (`url.rs` le verifie), et sans elle dans les capacites, la meme page
/// ne trouverait aucun pont - exactement comme dans un navigateur ordinaire.
/// Les deux valeurs sont ecrites a deux endroits ; ce test est ce qui les
/// empeche de diverger en silence.
#[test]
fn le_pont_est_ouvert_a_l_interface_servie_depuis_la_machine() {
    let capacites: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json"),
        )
        .expect("les capacites de l'application"),
    )
    .expect("capacites lisibles");

    let origines = capacites["remote"]["urls"]
        .as_array()
        .expect("le pont doit nommer les origines auxquelles il repond");

    assert_eq!(
        origines.len(),
        1,
        "une seule origine, et c'est celle de l'application : tout ajout ici ouvre le disque a \
         quelqu'un d'autre. Obtenu : {origines:?}"
    );
    let origine = origines[0].as_str().expect("une origine est un texte");
    assert!(
        origine.starts_with(&url_de_la_fenetre()),
        "le pont doit s'ouvrir a l'origine que la fenetre charge ({}), obtenu : {origine}",
        url_de_la_fenetre()
    );
}

/// Le pont ne repond a aucune origine du dehors.
///
/// La borne du PRD ne porte pas sur le nombre de commandes mais sur leur
/// nature ; celle-ci porte sur qui les appelle. Une page distante qui pourrait
/// les appeler ferait d'un site compromis un executant de commandes locales -
/// c'est l'option explicitement ecartee.
#[test]
fn aucune_origine_du_dehors_ne_peut_appeler_les_commandes_locales() {
    let capacites = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json"),
    )
    .expect("les capacites de l'application");
    let lues: serde_json::Value = serde_json::from_str(&capacites).expect("capacites lisibles");

    for origine in lues["remote"]["urls"]
        .as_array()
        .expect("le pont nomme ses origines")
    {
        let origine = origine.as_str().expect("une origine est un texte");
        assert!(
            origine.starts_with("http://127.0.0.1:"),
            "seule la boucle locale est admise, obtenu : {origine}"
        );
    }
}

/// Le compte de depots, au critere exact de la cartographie : les enfants
/// DIRECTS qui portent un `.git`.
///
/// C'est ce qui fait que ce compte repond a « pourquoi ce depot n'apparait
/// pas » : un compte plus genereux que la cartographie annoncerait des depots
/// qui ne remonteront jamais.
#[test]
fn le_compte_ne_retient_que_les_enfants_directs_porteurs_d_un_depot() {
    let bac = bac_a_sable("compte");
    depot(&bac, "carte");
    depot(&bac, "site");
    dossier_ordinaire(&bac, "notes");
    // Un depot plus profond : la cartographie ne descend pas, ce compte non
    // plus.
    depot(&bac.join("notes"), "brouillon");

    let vu = regarder("~/Developer", &bac);
    assert_eq!(vu.lisibilite, Lisibilite::Lisible);
    assert_eq!(
        vu.depots,
        Some(2),
        "deux enfants directs portent un .git, obtenu : {vu:?}"
    );
    assert_eq!(
        vu.chemin, "~/Developer",
        "l'ecran montre le chemin tel que l'utilisateur l'a ecrit"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un dossier bien ouvert et vide se dit « zero », jamais « illisible ».
///
/// Les deux se corrigent autrement : l'un attend qu'on y mette un depot,
/// l'autre qu'on repare le chemin.
#[test]
fn un_dossier_sans_aucun_depot_se_dit_zero_et_non_illisible() {
    let bac = bac_a_sable("vide");
    dossier_ordinaire(&bac, "rien-ici");

    let vu = regarder("~/Vide", &bac);
    assert_eq!(vu.lisibilite, Lisibilite::Lisible);
    assert_eq!(vu.depots, Some(0), "obtenu : {vu:?}");

    std::fs::remove_dir_all(&bac).ok();
}

/// Un dossier renomme sur le disque : sa ligne le signale, et son compte
/// disparait plutot que de tomber a zero (FR-029).
///
/// Un zero se lirait comme un dossier vide, ce qui enverrait l'utilisateur y
/// chercher des depots qui n'y sont pas parce que le dossier lui-meme n'y est
/// plus.
#[test]
fn un_dossier_introuvable_se_signale_et_ne_compte_pas_zero() {
    let bac = bac_a_sable("introuvable");
    let disparu = bac.join("Developer-renomme");

    let vu = regarder("~/Developer", &disparu);
    assert_eq!(vu.lisibilite, Lisibilite::Introuvable, "obtenu : {vu:?}");
    assert_eq!(
        vu.depots, None,
        "un dossier qui n'existe plus n'a pas zero depot : il n'a pas de compte"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un acces refuse par le systeme ne se donne pas pour un dossier disparu.
///
/// Sur macOS, c'est le cas courant des dossiers proteges tant que
/// l'application n'a pas recu l'autorisation : il n'y a rien a recreer, il y a
/// une autorisation a accorder.
#[test]
fn un_acces_refuse_se_distingue_d_un_dossier_disparu() {
    use std::os::unix::fs::PermissionsExt;

    let bac = bac_a_sable("refuse");
    let ferme = bac.join("interdit");
    std::fs::create_dir_all(&ferme).expect("dossier a fermer");
    depot(&ferme, "carte");
    std::fs::set_permissions(&ferme, std::fs::Permissions::from_mode(0o000))
        .expect("fermeture du dossier");

    let vu = regarder("~/Interdit", &ferme);

    // Rouvrir avant d'assener quoi que ce soit : un test qui echoue ne doit pas
    // laisser un dossier illisible derriere lui.
    std::fs::set_permissions(&ferme, std::fs::Permissions::from_mode(0o755)).ok();
    std::fs::remove_dir_all(&bac).ok();

    assert_eq!(
        vu.lisibilite,
        Lisibilite::AutorisationRefusee,
        "obtenu : {vu:?}"
    );
    assert_eq!(
        vu.depots, None,
        "on ne compte pas ce qu'on ne peut pas lire"
    );
}

/// La liste vient de la configuration du lecteur, dans l'ordre ou elle y est
/// ecrite, et le `~` s'y deplie.
#[test]
fn la_liste_suit_la_configuration_du_lecteur() {
    let bac = bac_a_sable("liste");
    let premier = bac.join("Developer");
    let second = bac.join("Sites");
    std::fs::create_dir_all(&premier).expect("premier dossier");
    std::fs::create_dir_all(&second).expect("second dossier");
    depot(&premier, "carte");

    let config = config_qui_surveille(
        &bac,
        &[
            &premier.display().to_string(),
            &second.display().to_string(),
        ],
    );

    let Surveillance::Lue { dossiers } = dossiers_surveilles(&config) else {
        panic!("une configuration lisible rend la liste des dossiers");
    };
    assert_eq!(dossiers.len(), 2, "obtenu : {dossiers:?}");
    assert_eq!(dossiers[0].depots, Some(1));
    assert_eq!(dossiers[1].depots, Some(0));

    std::fs::remove_dir_all(&bac).ok();
}

/// Sans configuration, l'ecran ne doit pas conclure « aucun dossier
/// surveille ».
///
/// « On ne sait pas ce qui est surveille » et « rien n'est surveille » ne se
/// corrigent pas de la meme facon, et la forme rendue les separe pour que
/// l'ecran ne puisse pas les confondre.
#[test]
fn une_configuration_absente_ne_se_donne_pas_pour_une_liste_vide() {
    let bac = bac_a_sable("sans-config");

    let rendu = dossiers_surveilles(&bac.join("jamais-ecrite.toml"));
    let Surveillance::SansConfiguration { raison } = rendu else {
        panic!("une configuration absente n'est pas une liste vide, obtenu : {rendu:?}");
    };
    assert!(
        !raison.is_empty(),
        "un refus sans raison ne dit rien a l'utilisateur"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// La forme que lit l'ecran.
///
/// Elle compte autant que le fond : c'est le contrat du pont, et un nom de
/// champ qui bouge laisserait la section muette sans que rien ne casse
/// ailleurs.
#[test]
fn la_liste_se_lit_telle_quelle_dans_la_fenetre() {
    let bac = bac_a_sable("forme");
    depot(&bac, "carte");
    let config = config_qui_surveille(&bac, &[&bac.display().to_string()]);

    let rendu = serde_json::to_value(dossiers_surveilles(&config)).expect("liste serialisable");
    assert_eq!(rendu["etat"], "lue");
    assert_eq!(rendu["dossiers"][0]["lisibilite"], "lisible");
    assert_eq!(rendu["dossiers"][0]["depots"], 1);
    assert_eq!(rendu["dossiers"][0]["chemin"], bac.display().to_string());

    let disparu = serde_json::to_value(regarder("~/Parti", &bac.join("parti")))
        .expect("dossier serialisable");
    assert_eq!(disparu["lisibilite"], "introuvable");
    assert_eq!(
        disparu["depots"],
        serde_json::Value::Null,
        "l'ecran doit pouvoir distinguer « pas de compte » de « zero »"
    );

    let sans_config = serde_json::to_value(dossiers_surveilles(&bac.join("absente.toml")))
        .expect("refus serialisable");
    assert_eq!(sans_config["etat"], "sans_configuration");

    std::fs::remove_dir_all(&bac).ok();
}
