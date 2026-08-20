//! Comportement : cesser de surveiller un dossier sans effacer ce qu'on a deja
//! observe (FR-032, FR-035, FR-036, issue #72).
//!
//! Le titre de la tranche est aussi son point delicat, et c'est ce que ce
//! fichier eprouve : retirer un dossier RETIRE UNE LIGNE, et rien d'autre. Ni le
//! dossier sur le disque, ni ce qui a deja ete cartographie n'en souffre. Les
//! depots qui venaient de la restent au catalogue, dates de leur derniere
//! cartographie, et cessent seulement d'etre rafraichis - c'est le comportement
//! prudent que FR-035 retient.
//!
//! Ce qui ne s'etablit pas ici : ce que devient le catalogue, qui vit en base.
//! Ce module ne l'ouvre pas, et c'est justement la preuve a faire - d'ou le test
//! qui compare l'etat du disque avant et apres, et n'y trouve qu'un seul fichier
//! change.
//!
//! Toujours sur des dossiers temporaires : un test ne doit rien devoir a ce qui
//! vit sur la machine, ni toucher a la configuration de l'utilisateur.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bureau::dossiers::{ajouter, retirer, Ajout, Retrait, Surveillance};
use vibemap::{Config, Verrou};

fn bac_a_sable(quoi: &str) -> PathBuf {
    let chemin = std::env::temp_dir().join(format!("bureau-retrait-{}-{quoi}", std::process::id()));
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).expect("bac a sable de test");
    chemin
}

/// Un enfant direct qui porte un `.git` : ce que la cartographie appelle un
/// depot.
fn depot(parent: &Path, nom: &str) {
    std::fs::create_dir_all(parent.join(nom).join(".git")).expect("faux depot");
}

/// Une configuration de lecteur ecrite comme celle d'un utilisateur : des champs
/// qui ne parlent pas des dossiers, et un commentaire.
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

/// Tout ce que le bac porte, chemin par chemin : les dossiers a `None`, les
/// fichiers avec leur contenu.
///
/// C'est l'instrument du test central : comparer deux releves dit exactement ce
/// qui a bouge sur le disque, sans avoir a deviner a l'avance ou regarder.
fn releve(racine: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut vu = BTreeMap::new();
    parcourir(racine, racine, &mut vu);
    vu
}

fn parcourir(racine: &Path, ici: &Path, vu: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
    let Ok(entrees) = std::fs::read_dir(ici) else {
        return;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        let relatif = chemin
            .strip_prefix(racine)
            .expect("un enfant du bac")
            .to_path_buf();
        if chemin.is_dir() {
            vu.insert(relatif, None);
            parcourir(racine, &chemin, vu);
        } else {
            vu.insert(
                relatif,
                Some(std::fs::read(&chemin).expect("fichier lisible")),
            );
        }
    }
}

/* ---------- ce que le retrait fait, et ce qu'il ne fait pas ---------- */

/// Le retrait enleve la ligne demandee, et ne touche a rien d'autre du fichier.
///
/// Meme fond que pour l'ajout : cette configuration est celle du lecteur, pas un
/// fichier a nous. Une ecriture qui la reconstruirait a partir de ce qu'on a su
/// en relire ferait disparaitre en silence l'adresse de la base, la cadence
/// reglee a la main, et jusqu'aux commentaires laisses par l'utilisateur.
#[test]
fn retirer_un_dossier_preserve_les_autres_racines_et_tout_le_reste_du_fichier() {
    let bac = bac_a_sable("preserve");
    let config = config_ecrite(
        &bac,
        "roots = [\"~/Developer\", \"~/Sites\", \"~/Travail\"]\n",
    );

    let retrait = retirer(&config, "~/Sites");
    assert!(
        matches!(retrait, Retrait::Retire { .. }),
        "le retrait devait aboutir, obtenu : {retrait:?}"
    );

    let relue = Config::load(&config).expect("la configuration reste lisible apres le retrait");
    assert_eq!(
        relue.roots,
        vec!["~/Developer".to_string(), "~/Travail".to_string()],
        "seule la racine demandee s'en va, et les autres gardent leur ordre"
    );
    assert_eq!(
        relue.supabase_url, "http://127.0.0.1:1",
        "l'adresse de la base n'a rien a voir avec les dossiers surveilles"
    );
    assert_eq!(relue.machine_id, "11111111-1111-1111-1111-111111111111");
    assert_eq!(relue.label, "machine de test");
    assert_eq!(
        relue.scan_seconds, 900,
        "une cadence reglee a la main doit survivre au retrait d'un dossier"
    );

    let texte = std::fs::read_to_string(&config).expect("configuration relisible");
    assert!(
        texte.contains("# Configuration de vibemap sur cette machine."),
        "le fichier est edite, pas reecrit : ce que l'utilisateur y a mis reste. Obtenu :\n{texte}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Le coeur de FR-035 : retirer un dossier n'efface rien de ce qui a deja ete
/// observe.
///
/// POURQUOI cette preuve-la : le catalogue vit en base, et ce module ne l'ouvre
/// pas. Ce qui se demontre ici est donc ce qui rend cette promesse vraie -
/// retirer un dossier n'ecrit QUE dans la configuration du lecteur. Ni les
/// depots du dossier retire, ni rien d'autre sur le disque ne bouge, et aucune
/// autre porte n'est ouverte par ou quoi que ce soit pourrait etre efface.
///
/// Un test qui ne regarderait que la liste des racines ne verrait pas la
/// difference entre « le dossier n'est plus surveille » et « le dossier a ete
/// emporte » : c'est exactement la confusion que cette tranche doit rendre
/// impossible.
#[test]
fn retirer_un_dossier_n_efface_rien_de_ce_qui_a_deja_ete_observe() {
    let bac = bac_a_sable("n-efface-rien");
    let surveille = bac.join("Sites");
    depot(&surveille, "carte");
    depot(&surveille, "site");
    std::fs::write(surveille.join("carte").join("README.md"), b"du travail")
        .expect("un fichier du depot");

    let config = config_ecrite(&bac, &format!("roots = [\"{}\"]\n", surveille.display()));
    let avant = releve(&bac);

    let retrait = retirer(&config, &surveille.display().to_string());
    assert!(
        matches!(retrait, Retrait::Retire { .. }),
        "le retrait devait aboutir, obtenu : {retrait:?}"
    );

    let apres = releve(&bac);
    // Les deux releves, et non le seul dernier : un fichier apparu, disparu ou
    // change doit se voir de la meme facon.
    let tous: BTreeSet<&PathBuf> = apres.keys().chain(avant.keys()).collect();
    let bouge: Vec<&PathBuf> = tous
        .into_iter()
        .filter(|chemin| avant.get(*chemin) != apres.get(*chemin))
        .collect();
    assert_eq!(
        bouge,
        vec![&PathBuf::from("config.toml")],
        "le retrait n'ecrit que dans la configuration du lecteur : tout le reste - les depots \
         deja cartographies compris - doit se retrouver au caractere pres. Obtenu : {bouge:?}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Retirer un dossier qui n'est pas surveille ne fait rien, et le dit.
///
/// Ne rien faire en silence laisserait croire que le geste a porte. Et ne pas
/// reecrire le fichier compte autant : une reecriture a l'identique changerait
/// sa date de modification sans rien changer d'autre, et ferait douter de ce qui
/// s'est passe.
#[test]
fn retirer_un_dossier_absent_ne_touche_pas_au_fichier_et_le_dit() {
    let bac = bac_a_sable("absent");
    let config = config_ecrite(&bac, "roots = [\"~/Developer\"]\n");
    let avant = std::fs::read_to_string(&config).expect("configuration relisible");

    let Retrait::Inconnu {
        chemin,
        surveillance,
    } = retirer(&config, "~/JamaisSurveille")
    else {
        panic!("un dossier absent de la liste ne se retire pas : cela doit se dire");
    };
    assert_eq!(
        chemin, "~/JamaisSurveille",
        "le refus nomme le dossier qu'on a voulu retirer"
    );

    let Surveillance::Lue { dossiers, .. } = surveillance else {
        panic!("le refus rend quand meme la liste du poste, pour que l'ecran se remette d'aplomb");
    };
    assert_eq!(
        dossiers.len(),
        1,
        "la liste rendue est celle du poste, inchangee. Obtenu : {dossiers:?}"
    );

    assert_eq!(
        std::fs::read_to_string(&config).expect("configuration relisible"),
        avant,
        "le fichier ne doit pas avoir bouge d'un caractere"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Une configuration qui n'ecrit pas ses racines en surveille quand meme une :
/// le lecteur lui en prete une par defaut. La retirer doit vraiment la retirer.
///
/// POURQUOI ce cas a part : sans liste ecrite, il n'y a rien a enlever du
/// fichier. Un retrait qui se contenterait d'editer ce qui est ecrit
/// n'enleverait rien du tout, et le lecteur continuerait de cartographier un
/// dossier que l'utilisateur vient de retirer - sans que rien ne le lui dise.
#[test]
fn retirer_la_racine_par_defaut_l_ecrit_noir_sur_blanc_privee_d_elle() {
    let bac = bac_a_sable("defaut");
    let config = config_ecrite(&bac, "");

    let avant = Config::load(&config).expect("configuration de depart");
    assert_eq!(
        avant.roots,
        vec!["~/Developer".to_string()],
        "le lecteur surveille bien une racine par defaut : c'est ce qui est en jeu ici"
    );

    let retrait = retirer(&config, "~/Developer");
    assert!(
        matches!(retrait, Retrait::Retire { .. }),
        "le retrait devait aboutir, obtenu : {retrait:?}"
    );

    let relue = Config::load(&config).expect("la configuration reste lisible apres le retrait");
    assert!(
        relue.roots.is_empty(),
        "la racine implicite doit vraiment cesser d'etre surveillee, obtenu : {:?}",
        relue.roots
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Une configuration ecrite a la main peut porter deux fois le meme chemin. Le
/// retrait les emporte tous les deux : n'en enlever qu'un laisserait le dossier
/// surveille apres un geste qui annonce le contraire.
#[test]
fn un_chemin_ecrit_deux_fois_cesse_vraiment_d_etre_surveille() {
    let bac = bac_a_sable("doublon-ecrit");
    let config = config_ecrite(
        &bac,
        "roots = [\"~/Sites\", \"~/Developer\", \"~/Sites\"]\n",
    );

    retirer(&config, "~/Sites");

    let relue = Config::load(&config).expect("configuration relisible");
    assert_eq!(
        relue.roots,
        vec!["~/Developer".to_string()],
        "aucune occurrence ne doit survivre au retrait, obtenu : {:?}",
        relue.roots
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Sans configuration a editer, le retrait se refuse et le dit.
#[test]
fn sans_configuration_le_retrait_se_refuse_et_n_en_fabrique_pas_une() {
    let bac = bac_a_sable("sans-config");
    let jamais_ecrite = bac.join("jamais-ecrite.toml");

    let Retrait::Echoue { raison } = retirer(&jamais_ecrite, "~/Developer") else {
        panic!("sans configuration, le retrait ne peut pas aboutir");
    };
    assert!(
        raison.contains(&jamais_ecrite.display().to_string()),
        "le refus nomme le fichier qui manque, obtenu : {raison}"
    );
    assert!(
        !jamais_ecrite.exists(),
        "le retrait n'ecrit pas une configuration qui n'existe pas : elle serait incomplete"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un retrait n'a jamais lieu a moitie, et ne desserre pas les droits du
/// fichier.
///
/// Le remplacement passe par un fichier neuf, et un fichier neuf nait avec les
/// droits par defaut du compte : sans reprise explicite des anciens, retirer un
/// dossier ouvrirait au reste de la machine une configuration que l'utilisateur
/// avait fermee.
#[test]
fn le_retrait_ne_laisse_aucun_brouillon_et_ne_desserre_pas_les_droits() {
    use std::os::unix::fs::PermissionsExt;

    let bac = bac_a_sable("brouillon");
    let config = config_ecrite(&bac, "roots = [\"~/Developer\", \"~/Sites\"]\n");
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600))
        .expect("configuration fermee");

    retirer(&config, "~/Sites");

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

/// Retirer puis rajouter le meme dossier le remet a surveiller, une seule fois.
///
/// C'est ce qui fait du retrait un geste reversible : rien n'ayant ete efface,
/// le rajouter suffit a reprendre la cartographie la ou elle en etait. Et le
/// refus du doublon (FR-037) ne doit pas s'y opposer - il n'y a plus de doublon
/// des lors que le dossier est parti.
#[test]
fn retirer_puis_rajouter_reprend_la_surveillance_sans_doublon() {
    let bac = bac_a_sable("aller-retour");
    let surveille = bac.join("Sites");
    depot(&surveille, "carte");
    let ecrit = surveille.display().to_string();
    let config = config_ecrite(&bac, &format!("roots = [\"~/Developer\", \"{ecrit}\"]\n"));

    retirer(&config, &ecrit);
    assert_eq!(
        Config::load(&config)
            .expect("configuration relisible")
            .roots,
        vec!["~/Developer".to_string()],
        "le dossier doit d'abord etre vraiment parti"
    );

    let ajout = ajouter(&config, &surveille);
    let Ajout::Ajoute { surveillance, .. } = &ajout else {
        panic!("un dossier retire doit pouvoir etre rajoute, obtenu : {ajout:?}");
    };
    let Surveillance::Lue { dossiers, .. } = surveillance else {
        panic!("la liste rendue devait etre lue, obtenu : {surveillance:?}");
    };
    assert_eq!(
        dossiers.last().map(|dossier| dossier.depots),
        Some(Some(1)),
        "ses depots reprennent leur cartographie, sans que rien n'ait ete perdu entre-temps"
    );

    let relue = Config::load(&config).expect("configuration relisible");
    assert_eq!(
        relue.roots,
        vec!["~/Developer".to_string(), ecrit],
        "le dossier revient une fois, jamais deux, obtenu : {:?}",
        relue.roots
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// La forme que lit l'ecran.
///
/// Elle compte autant que le fond : c'est le contrat du pont, et un nom de champ
/// qui bouge laisserait le bouton muet sans que rien ne casse ailleurs. Les
/// trois issues doivent surtout rester distinctes - « rien a retirer » n'est pas
/// « retire », et l'ecran ne doit pas pouvoir les confondre.
#[test]
fn les_issues_du_retrait_se_lisent_telles_quelles_dans_la_fenetre() {
    let echoue = serde_json::to_value(Retrait::Echoue {
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
    let reste = bac.join("Developer");
    depot(&reste, "carte");
    let config = config_ecrite(
        &bac,
        &format!("roots = [\"{}\", \"~/Sites\"]\n", reste.display()),
    );

    let rendu = serde_json::to_value(retirer(&config, "~/Sites")).expect("issue serialisable");
    assert_eq!(rendu["issue"], "retire");
    assert_eq!(rendu["chemin"], "~/Sites");
    assert_eq!(
        rendu["surveillance"]["etat"], "lue",
        "l'ecran lit la liste au meme endroit que d'habitude"
    );
    assert_eq!(rendu["surveillance"]["dossiers"][0]["depots"], 1);
    assert_eq!(
        rendu["surveillance"]["dossiers"][1],
        serde_json::Value::Null,
        "le dossier retire ne figure plus dans la liste rendue"
    );

    let inconnu = serde_json::to_value(retirer(&config, "~/Sites")).expect("issue serialisable");
    assert_eq!(
        inconnu["issue"], "inconnu",
        "le meme retrait, une seconde fois, n'a plus rien a retirer"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- le dossier cesse d'etre cartographie (FR-033, FR-035) ---------- */

/// Une configuration qui ne regarde rien de ce qui vit sur cette machine, et une
/// adresse ou personne ne repond : le lecteur tourne pour de vrai, sans rien
/// toucher.
fn config_du_lecteur(bac: &Path, racines: &str) -> PathBuf {
    let journaux = bac.join("journaux");
    std::fs::create_dir_all(&journaux).expect("dossier de journaux de test");

    let chemin = bac.join("config.toml");
    std::fs::write(
        &chemin,
        format!(
            "supabase_url = \"http://127.0.0.1:1\"\n\
             machine_id = \"11111111-1111-1111-1111-111111111111\"\n\
             label = \"machine de test\"\n\
             roots = [{racines}]\n\
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

/// Un dossier retire pendant que le lecteur tourne cesse d'etre cartographie
/// sans fermer l'application (FR-033, FR-035).
///
/// La sequence complete, dans l'ordre ou l'application la joue : le lecteur
/// tourne, le dossier quitte la configuration, le lecteur repart. Il relit alors
/// la liste - `bureau/tests/ajout.rs` etablit que seule la reprise la relit - et
/// le poste ne lui echappe pas au passage.
///
/// Ce qui ne s'etablit pas ici : que les lignes deja ecrites en base restent en
/// place. Elles y sont, et ce module n'a aucun moyen de les toucher :
/// `retirer_un_dossier_n_efface_rien_de_ce_qui_a_deja_ete_observe` le montre du
/// cote du disque.
#[test]
fn un_dossier_retire_cesse_d_etre_cartographie_sans_fermer_l_application() {
    // Le jeton vient de l'environnement : le trousseau du systeme n'a rien a
    // faire dans un test.
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");

    let bac = bac_a_sable("a-chaud");
    let surveille = bac.join("Sites");
    depot(&surveille, "carte");
    let ecrit = surveille.display().to_string();
    let config = config_du_lecteur(&bac, &format!("\"{ecrit}\""));
    let verrou = bac.join("poste.lock");

    let mut lecteur = None;
    bureau::lecteur::relancer(&mut lecteur, &config, &verrou);

    let retrait = retirer(&config, &ecrit);
    let Retrait::Retire { surveillance, .. } = &retrait else {
        panic!("le retrait devait aboutir, obtenu : {retrait:?}");
    };
    let Surveillance::Lue { dossiers, .. } = surveillance else {
        panic!("la liste rendue devait etre lue, obtenu : {surveillance:?}");
    };
    assert!(
        dossiers.is_empty(),
        "l'ecran voit le dossier disparaitre de la liste sans rien redemander, obtenu : \
         {dossiers:?}"
    );

    assert_eq!(
        bureau::lecteur::reprendre(&mut lecteur, &config, &verrou),
        bureau::lecteur::EtatLecteur::EnMarche,
        "apres un retrait, le lecteur repart : c'est ce qui le fait cesser de cartographier"
    );
    assert!(
        Config::load(&config)
            .expect("configuration lisible")
            .roots
            .is_empty(),
        "c'est cette liste-la que le lecteur repris vient de lire"
    );

    // Le dossier et ses depots sont toujours la : on a cesse de les regarder, on
    // ne les a pas emportes.
    assert!(
        surveille.join("carte").join(".git").exists(),
        "cesser de surveiller n'efface rien sur le disque"
    );

    // Et le poste n'a pas ete perdu au passage : le lecteur repris le tient.
    Verrou::prendre(&verrou, "vibemap").expect_err("le lecteur repris tient le poste");

    drop(lecteur);
    std::fs::remove_dir_all(&bac).ok();
}
