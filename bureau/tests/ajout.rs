//! Comportement : ajouter un dossier surveille sans jamais ouvrir un fichier.
//!
//! C'est la promesse de FR-036 vue de l'exterieur : le fichier de
//! configuration reste le support de stockage, il cesse d'etre une porte. Ce
//! qui se verifie ici, c'est donc ce que devient ce fichier apres un ajout -
//! il a gagne une ligne, et il n'a rien perdu -, et ce que l'application fait
//! ensuite du lecteur pour que le dossier soit surveille tout de suite
//! (FR-033, FR-034).
//!
//! Le selecteur du systeme lui-meme ne se teste pas : c'est une fenetre du
//! systeme, et personne ne clique dedans ici. Tout ce qui vient APRES le choix
//! se teste, et c'est tout ce qui peut casser. La verification a la main est
//! dans `bureau/VERIFICATIONS.md`.
//!
//! Toujours sur des dossiers temporaires : un test ne doit rien devoir a ce qui
//! vit sur la machine, ni toucher a la configuration de l'utilisateur.

use std::path::{Path, PathBuf};

use bureau::dossiers::{abreger, ajouter, Ajout, Surveillance};
use vibemap::{Config, Verrou};

fn bac_a_sable(quoi: &str) -> PathBuf {
    let chemin = std::env::temp_dir().join(format!("bureau-ajout-{}-{quoi}", std::process::id()));
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).expect("bac a sable de test");
    chemin
}

/// Un enfant direct qui porte un `.git` : ce que la cartographie appelle un
/// depot.
fn depot(parent: &Path, nom: &str) {
    std::fs::create_dir_all(parent.join(nom).join(".git")).expect("faux depot");
}

/// Une configuration de lecteur ecrite comme celle d'un utilisateur : des
/// champs qui ne parlent pas des dossiers, et un commentaire.
fn config_ecrite(bac: &Path, corps: &str) -> PathBuf {
    let chemin = bac.join("config.toml");
    std::fs::write(
        &chemin,
        format!(
            "# Configuration de vibemap sur cette machine.\n\
             supabase_url = \"http://127.0.0.1:1\"\n\
             machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
             label = \"machine de test\"\n\
             scan_seconds = 900\n\
             {corps}"
        ),
    )
    .expect("ecriture de la configuration de test");
    chemin
}

/// L'ajout inscrit le dossier a cote des autres, et ne touche a rien d'autre.
///
/// C'est le fond de l'affaire : cette configuration est celle du lecteur, pas
/// un fichier a nous. Une ecriture qui la reconstruirait a partir de ce qu'on a
/// su en relire ferait disparaitre en silence l'adresse de la base, la cadence
/// que l'utilisateur avait reglee, et jusqu'aux commentaires qu'il y a laisses.
/// Il n'aurait aucun moyen de s'en apercevoir avant que le lecteur ne demarre
/// plus.
#[test]
fn ajouter_un_dossier_preserve_les_autres_racines_et_tout_le_reste_du_fichier() {
    let bac = bac_a_sable("preserve");
    let config = config_ecrite(&bac, "roots = [\"~/Developer\", \"~/Travail\"]\n");
    let choisi = bac.join("Sites");
    std::fs::create_dir_all(&choisi).expect("dossier choisi");

    let ajout = ajouter(&config, &choisi);
    assert!(
        matches!(ajout, Ajout::Ajoute { .. }),
        "l'ajout devait aboutir, obtenu : {ajout:?}"
    );

    let relue = Config::load(&config).expect("la configuration reste lisible apres l'ajout");
    assert_eq!(
        relue.roots,
        vec![
            "~/Developer".to_string(),
            "~/Travail".to_string(),
            choisi.display().to_string()
        ],
        "le dossier s'ajoute a cote des anciens, jamais a leur place"
    );
    assert_eq!(
        relue.supabase_url, "http://127.0.0.1:1",
        "l'adresse de la base n'a rien a voir avec les dossiers surveilles"
    );
    assert_eq!(relue.machine_id, "11111111-1111-1111-1111-111111111111");
    assert_eq!(relue.label, "machine de test");
    assert_eq!(
        relue.scan_seconds, 900,
        "une cadence reglee a la main doit survivre a l'ajout d'un dossier"
    );

    let texte = std::fs::read_to_string(&config).expect("configuration relisible");
    assert!(
        texte.contains("# Configuration de vibemap sur cette machine."),
        "le fichier est edite, pas reecrit : ce que l'utilisateur y a mis reste. Obtenu :\n{texte}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Une configuration qui n'ecrit pas ses racines en surveille quand meme une :
/// le lecteur lui en prete une par defaut. L'ajout ne doit pas la faire
/// disparaitre.
///
/// POURQUOI ce cas a part : ecrire simplement le dossier choisi remplacerait
/// une liste implicite d'un element par une liste explicite d'un autre. Le
/// dossier historique cesserait d'etre cartographie, sans que personne n'ait
/// demande a le retirer, et l'ecran ne montrerait plus que le nouveau.
#[test]
fn une_configuration_sans_racines_ne_perd_pas_celle_du_lecteur_par_defaut() {
    let bac = bac_a_sable("defaut");
    let config = config_ecrite(&bac, "");
    let choisi = bac.join("Sites");
    std::fs::create_dir_all(&choisi).expect("dossier choisi");

    let avant = Config::load(&config).expect("configuration de depart");
    assert_eq!(
        avant.roots,
        vec!["~/Developer".to_string()],
        "le lecteur surveille bien une racine par defaut : c'est ce qui est en jeu ici"
    );

    ajouter(&config, &choisi);

    let relue = Config::load(&config).expect("la configuration reste lisible apres l'ajout");
    assert_eq!(
        relue.roots,
        vec!["~/Developer".to_string(), choisi.display().to_string()],
        "la racine surveillee par defaut doit survivre a l'ajout, ecrite noir sur blanc"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// L'ajout rend la liste a jour, comptes compris : c'est elle que l'ecran
/// affiche, sans avoir a redemander.
#[test]
fn un_ajout_rend_la_liste_entiere_avec_le_compte_du_nouveau_dossier() {
    let bac = bac_a_sable("liste");
    let ancien = bac.join("Developer");
    std::fs::create_dir_all(&ancien).expect("ancien dossier");
    let config = config_ecrite(&bac, &format!("roots = [\"{}\"]\n", ancien.display()));

    let choisi = bac.join("Sites");
    depot(&choisi, "carte");
    depot(&choisi, "site");

    let Ajout::Ajoute {
        chemin,
        surveillance: Surveillance::Lue { dossiers },
    } = ajouter(&config, &choisi)
    else {
        panic!("un ajout abouti rend la liste des dossiers surveilles");
    };

    assert_eq!(chemin, choisi.display().to_string());
    assert_eq!(dossiers.len(), 2, "obtenu : {dossiers:?}");
    assert_eq!(
        dossiers[1].chemin,
        choisi.display().to_string(),
        "le dossier ajoute vient au bout de la liste, la ou il a ete ecrit"
    );
    assert_eq!(
        dossiers[1].depots,
        Some(2),
        "l'ecran montre le compte du nouveau dossier tout de suite, obtenu : {:?}",
        dossiers[1]
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Sans configuration a editer, l'ajout se refuse et le dit.
///
/// Il n'en ecrit surtout pas une : une configuration fabriquee ici n'aurait ni
/// adresse de base ni identifiant de machine, et le lecteur la refuserait au
/// demarrage suivant. Le poste doit d'abord etre appaire.
#[test]
fn sans_configuration_l_ajout_se_refuse_et_n_en_fabrique_pas_une() {
    let bac = bac_a_sable("sans-config");
    let jamais_ecrite = bac.join("jamais-ecrite.toml");
    let choisi = bac.join("Sites");
    std::fs::create_dir_all(&choisi).expect("dossier choisi");

    let Ajout::Echoue { raison } = ajouter(&jamais_ecrite, &choisi) else {
        panic!("sans configuration, l'ajout ne peut pas aboutir");
    };
    assert!(
        raison.contains(&jamais_ecrite.display().to_string()),
        "le refus nomme le fichier qui manque, obtenu : {raison}"
    );
    assert!(
        !jamais_ecrite.exists(),
        "l'ajout n'ecrit pas une configuration qui n'existe pas : elle serait incomplete"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Une configuration que le lecteur refuserait ne gagne pas de ligne.
///
/// Ecrire dedans donnerait a l'utilisateur un dossier « ajoute » a une
/// configuration dont rien ne se sert : il chercherait ensuite pourquoi ses
/// depots n'apparaissent pas, alors que le lecteur ne demarre pas du tout.
#[test]
fn une_configuration_que_le_lecteur_refuse_ne_gagne_pas_de_ligne() {
    let bac = bac_a_sable("refusee");
    // Un jeton en clair : le lecteur refuse cette configuration depuis
    // l'appairage, et son message dit quoi faire.
    let config = bac.join("config.toml");
    std::fs::write(
        &config,
        "supabase_url = \"http://127.0.0.1:1\"\n\
         token = \"un-jeton-oublie-la\"\n\
         machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
         label = \"machine de test\"\n\
         roots = [\"~/Developer\"]\n",
    )
    .expect("configuration de test");
    let avant = std::fs::read_to_string(&config).expect("configuration relisible");

    let choisi = bac.join("Sites");
    std::fs::create_dir_all(&choisi).expect("dossier choisi");

    let Ajout::Echoue { raison } = ajouter(&config, &choisi) else {
        panic!("une configuration que le lecteur refuse ne s'edite pas");
    };
    assert!(
        !raison.is_empty(),
        "un refus sans raison ne dit rien a l'utilisateur"
    );
    assert_eq!(
        std::fs::read_to_string(&config).expect("configuration relisible"),
        avant,
        "le fichier ne doit pas avoir bouge d'un caractere"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un ajout n'a jamais lieu a moitie.
///
/// Le remplacement est atomique : a tout instant, la configuration sur le
/// disque est soit l'ancienne entiere, soit la nouvelle entiere. Et il ne
/// laisse rien a cote : un brouillon oublie ferait douter de laquelle des deux
/// fait foi.
#[test]
fn l_ajout_ne_laisse_aucun_brouillon_a_cote_de_la_configuration() {
    let bac = bac_a_sable("brouillon");
    let config = config_ecrite(&bac, "roots = [\"~/Developer\"]\n");
    let choisi = bac.join("Sites");
    std::fs::create_dir_all(&choisi).expect("dossier choisi");

    ajouter(&config, &choisi);

    let voisins: Vec<String> = std::fs::read_dir(&bac)
        .expect("bac lisible")
        .flatten()
        .map(|entree| entree.file_name().to_string_lossy().to_string())
        .filter(|nom| nom.starts_with("config.toml") && nom != "config.toml")
        .collect();
    assert!(
        voisins.is_empty(),
        "aucun fichier de travail ne reste a cote de la configuration, obtenu : {voisins:?}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Une configuration fermee a son seul proprietaire le reste apres l'ajout.
///
/// Le remplacement passe par un fichier neuf, et un fichier neuf nait avec les
/// droits par defaut du compte. Sans reprise explicite des anciens, ajouter un
/// dossier ouvrirait au reste de la machine une configuration que l'utilisateur
/// avait fermee - et rien ne le lui dirait.
#[test]
fn l_ajout_ne_desserre_pas_les_droits_de_la_configuration() {
    use std::os::unix::fs::PermissionsExt;

    let bac = bac_a_sable("droits");
    let config = config_ecrite(&bac, "roots = [\"~/Developer\"]\n");
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600))
        .expect("configuration fermee");
    let choisi = bac.join("Sites");
    std::fs::create_dir_all(&choisi).expect("dossier choisi");

    ajouter(&config, &choisi);

    let droits = std::fs::metadata(&config)
        .expect("configuration relisible")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        droits, 0o600,
        "les droits de la configuration doivent survivre au remplacement, obtenu : {droits:o}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Le chemin choisi s'ecrit comme l'utilisateur ecrit les siens : `~/Sites`, et
/// non `/Users/lea/Sites`.
///
/// La comparaison porte sur les COMPOSANTS du chemin, jamais sur le texte : un
/// dossier personnel `/Users/lea` ne prefixe pas `/Users/leandre/Sites`, et une
/// abreviation faite sur le texte y ecrirait un `~ndre/Sites` qui ne designe
/// rien du tout.
#[test]
fn le_dossier_choisi_s_ecrit_avec_le_tilde_du_dossier_personnel() {
    let maison = Path::new("/Users/lea");

    assert_eq!(abreger(Path::new("/Users/lea/Sites"), maison), "~/Sites");
    assert_eq!(
        abreger(Path::new("/Users/lea/code/dépôts"), maison),
        "~/code/dépôts",
        "un accent dans un nom de dossier n'est pas un cas particulier"
    );
    assert_eq!(
        abreger(maison, maison),
        "~",
        "le dossier personnel lui-meme s'ecrit « ~ », jamais « ~/ »"
    );
    assert_eq!(
        abreger(Path::new("/Users/leandre/Sites"), maison),
        "/Users/leandre/Sites",
        "un dossier personnel qui n'est qu'un prefixe de texte n'en est pas un"
    );
    assert_eq!(
        abreger(Path::new("/Volumes/Disque/code"), maison),
        "/Volumes/Disque/code",
        "hors du dossier personnel, le chemin s'ecrit tel quel"
    );
    assert_eq!(
        abreger(Path::new("/Users/lea/Sites"), Path::new("")),
        "/Users/lea/Sites",
        "sans dossier personnel connu, on n'abrege rien plutot que d'abreger au hasard"
    );
}

/// La forme que lit l'ecran.
///
/// Elle compte autant que le fond : c'est le contrat du pont, et un nom de
/// champ qui bouge laisserait le bouton muet sans que rien ne casse ailleurs.
/// Les trois issues doivent surtout rester distinctes : un selecteur referme
/// n'est pas un echec, et l'ecran ne doit pas pouvoir les confondre.
#[test]
fn les_issues_de_l_ajout_se_lisent_telles_quelles_dans_la_fenetre() {
    let annule = serde_json::to_value(Ajout::Annule).expect("issue serialisable");
    assert_eq!(annule, serde_json::json!({ "issue": "annule" }));

    let echoue = serde_json::to_value(Ajout::Echoue {
        raison: "aucune configuration a /tmp/config.toml".to_string(),
    })
    .expect("issue serialisable");
    assert_eq!(
        echoue,
        serde_json::json!({
            "issue": "echoue",
            "raison": "aucune configuration a /tmp/config.toml",
        })
    );

    let bac = bac_a_sable("forme");
    let config = config_ecrite(&bac, "roots = []\n");
    let choisi = bac.join("Sites");
    depot(&choisi, "carte");

    let rendu = serde_json::to_value(ajouter(&config, &choisi)).expect("issue serialisable");
    assert_eq!(rendu["issue"], "ajoute");
    assert_eq!(rendu["chemin"], choisi.display().to_string());
    assert_eq!(
        rendu["surveillance"]["etat"], "lue",
        "l'ecran lit la liste au meme endroit que d'habitude"
    );
    assert_eq!(rendu["surveillance"]["dossiers"][0]["depots"], 1);

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- le dossier est surveille tout de suite (FR-033, FR-034) ---------- */

/// Une configuration qui ne regarde rien de ce qui vit sur cette machine, et
/// une adresse ou personne ne repond : le lecteur tourne pour de vrai, sans
/// rien toucher.
fn config_du_lecteur(bac: &Path) -> PathBuf {
    let journaux = bac.join("journaux");
    std::fs::create_dir_all(&journaux).expect("dossier de journaux de test");

    let chemin = bac.join("config.toml");
    std::fs::write(
        &chemin,
        format!(
            "supabase_url = \"http://127.0.0.1:1\"\n\
             machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
             label = \"machine de test\"\n\
             roots = []\n\
             claude_projects = \"{}\"\n\
             interval_seconds = 3600\n\
             scan_seconds = 3600\n\
             journal_seconds = 3600\n\
             worktree_seconds = 3600\n\
             commit_seconds = 3600\n",
            journaux.display()
        ),
    )
    .expect("ecriture de la configuration de test");
    chemin
}

/// La reprise relit la configuration ; la relance ordinaire, non. C'est
/// exactement ce qui fait qu'un dossier ajoute est surveille tout de suite.
///
/// POURQUOI cette preuve-la : la configuration n'est lue qu'au demarrage du
/// lecteur, et rien dans sa boucle n'y revient. Un lecteur qu'on ne fait pas
/// repartir tourne donc sur l'ancienne liste, quoi qu'il arrive au fichier.
/// L'observer, c'est mettre la configuration de cote : celui qui la relit
/// echoue, celui qui ne la relit pas continue de tourner. Comparer les adresses
/// des deux lecteurs ne dirait rien - la valeur vit dans la meme case avant et
/// apres un remplacement.
#[test]
fn seule_la_reprise_relit_la_configuration_du_lecteur() {
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");

    let bac = bac_a_sable("relecture");
    let config = config_du_lecteur(&bac);
    let verrou = bac.join("poste.lock");

    let mut lecteur = None;
    assert_eq!(
        bureau::lecteur::relancer(&mut lecteur, &config, &verrou),
        bureau::lecteur::EtatLecteur::EnMarche
    );

    let mise_de_cote = std::fs::read_to_string(&config).expect("configuration relisible");
    std::fs::remove_file(&config).expect("configuration mise de cote");

    assert_eq!(
        bureau::lecteur::relancer(&mut lecteur, &config, &verrou),
        bureau::lecteur::EtatLecteur::EnMarche,
        "une relance ordinaire ne relit rien : elle laisse tourner le lecteur en place"
    );

    let repris = bureau::lecteur::reprendre(&mut lecteur, &config, &verrou);
    assert!(
        matches!(repris, bureau::lecteur::EtatLecteur::EnEchec(_)),
        "la reprise repart de zero et relit la configuration : sans elle, elle echoue. \
         Obtenu : {repris:?}"
    );

    // Remise en place : la reprise redemarre alors pour de bon, et reprend le
    // poste qu'elle avait rendu.
    std::fs::write(&config, mise_de_cote).expect("configuration remise en place");
    assert_eq!(
        bureau::lecteur::reprendre(&mut lecteur, &config, &verrou),
        bureau::lecteur::EtatLecteur::EnMarche
    );
    Verrou::prendre(&verrou, "vibemap").expect_err("le lecteur repris tient le poste");

    drop(lecteur);
    std::fs::remove_dir_all(&bac).ok();
}

/// Un dossier ajoute pendant que le lecteur tourne est pris en compte sans
/// fermer l'application (FR-033), et sans attendre la cartographie periodique
/// (FR-034).
///
/// La sequence complete, dans l'ordre ou l'application la joue : le lecteur
/// tourne, le dossier s'inscrit dans la configuration, le lecteur repart. Il
/// relit alors la liste - `seule_la_reprise_relit_la_configuration_du_lecteur`
/// l'etablit - et le poste ne lui echappe pas au passage.
///
/// Ce qui ne s'etablit pas ici : que la cartographie parte VERS la base. Elle a
/// lieu des le demarrage du lecteur, avant sa premiere boucle, et c'est
/// `daemon/` qui l'eprouve contre une vraie pile.
#[test]
fn un_dossier_ajoute_est_surveille_sans_fermer_l_application() {
    // Le jeton vient de l'environnement : le trousseau du systeme n'a rien a
    // faire dans un test.
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");

    let bac = bac_a_sable("a-chaud");
    let config = config_du_lecteur(&bac);
    let verrou = bac.join("poste.lock");

    let mut lecteur = None;
    bureau::lecteur::relancer(&mut lecteur, &config, &verrou);

    let choisi = bac.join("Sites");
    depot(&choisi, "carte");
    let ajout = ajouter(&config, &choisi);
    let Ajout::Ajoute { surveillance, .. } = &ajout else {
        panic!("l'ajout devait aboutir, obtenu : {ajout:?}");
    };
    let Surveillance::Lue { dossiers } = surveillance else {
        panic!("la liste rendue devait etre lue, obtenu : {surveillance:?}");
    };
    assert_eq!(
        dossiers.last().map(|dossier| dossier.depots),
        Some(Some(1)),
        "l'ecran voit le dossier ajoute et son depot sans rien redemander"
    );

    assert_eq!(
        bureau::lecteur::reprendre(&mut lecteur, &config, &verrou),
        bureau::lecteur::EtatLecteur::EnMarche,
        "apres un ajout, le lecteur repart - c'est ce qui fait la minute de FR-034"
    );
    assert!(
        Config::load(&config)
            .expect("configuration lisible")
            .roots
            .contains(&choisi.display().to_string()),
        "c'est cette liste-la que le lecteur repris vient de lire"
    );

    // Et le poste n'a pas ete perdu au passage : le lecteur repris le tient.
    Verrou::prendre(&verrou, "vibemap").expect_err("le lecteur repris tient le poste");

    drop(lecteur);
    std::fs::remove_dir_all(&bac).ok();
}
