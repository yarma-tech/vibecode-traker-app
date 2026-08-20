//! Comportement : refuser un dossier deja surveille, en nommant le coupable
//! (FR-037, FR-075, issue #71).
//!
//! Ce qui s'eprouve ici, c'est la REGLE, pas l'ecriture : un dossier est deja
//! surveille apres depliement du `~`, resolution des liens symboliques et
//! normalisation - barre finale comprise -, et le refus vaut aussi pour un
//! dossier contenu dans un dossier surveille ou qui en contient un. Chaque forme
//! d'ecriture a son cas, pour que le test dise laquelle a regresse.
//!
//! Le `~` se deplie contre `HOME`, que ce fichier pose une fois pour tout le
//! binaire : chaque fichier de `tests/` est un processus a part, et les bacs a
//! sable vivent tous SOUS ce dossier personnel de test. Rien ne touche donc a ce
//! qui vit sur la machine, ni a la configuration de l'utilisateur.

use std::path::{Path, PathBuf};
use std::sync::Once;

use bureau::dossiers::{ajouter, deja_surveille, Ajout, Emboitement};
use vibemap::Config;

static MAISON_POSEE: Once = Once::new();

/// Le dossier personnel de ce processus de test. Le `~` des configurations
/// ecrites ici s'y deplie.
fn maison() -> PathBuf {
    let chemin = std::env::temp_dir().join(format!("bureau-doublon-{}", std::process::id()));
    MAISON_POSEE.call_once(|| {
        std::fs::remove_dir_all(&chemin).ok();
        std::fs::create_dir_all(&chemin).expect("dossier personnel de test");
        std::env::set_var("HOME", &chemin);
    });
    chemin
}

/// Un bac a sable dans ce dossier personnel : les chemins qu'on y pose
/// s'ecrivent donc bien avec un `~`.
fn bac_a_sable(quoi: &str) -> PathBuf {
    let chemin = maison().join(quoi);
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).expect("bac a sable de test");
    chemin
}

/// Le chemin d'un dossier du bac, ecrit avec le `~` que la configuration porte.
fn tilde(chemin: &Path) -> String {
    format!(
        "~/{}",
        chemin
            .strip_prefix(maison())
            .expect("le bac vit dans le dossier personnel de test")
            .display()
    )
}

/// Une configuration de lecteur qui surveille les dossiers donnes.
fn config_qui_surveille(bac: &Path, racines: &[String]) -> PathBuf {
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

/// Ce que l'ajout a refuse, ou un echec de test qui dit ce qu'on a obtenu.
fn refus(ajout: Ajout) -> (String, String, Emboitement) {
    match ajout {
        Ajout::Refuse { chemin, deja, cas } => (chemin, deja, cas),
        autre => panic!("le dossier devait etre refuse, obtenu : {autre:?}"),
    }
}

/* ---------- le meme dossier, sous ses quatre ecritures ---------- */

/// Un dossier deja surveille se reconnait sous chacune des formes par
/// lesquelles le selecteur du systeme peut le rendre.
///
/// Les quatre cas sont separes a dessein : si l'un regresse - la barre finale,
/// disons -, le test dit laquelle, la ou un cas unique dirait seulement
/// « le doublon ne marche plus ».
#[test]
fn le_meme_dossier_ecrit_a_l_identique_est_refuse_en_nommant_le_coupable() {
    let bac = bac_a_sable("meme");
    let developer = bac.join("Developer");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    let ecrit = tilde(&developer);
    let config = config_qui_surveille(&bac, std::slice::from_ref(&ecrit));
    let avant = std::fs::read_to_string(&config).expect("configuration relisible");

    let (chemin, deja, cas) = refus(ajouter(&config, &developer));
    assert_eq!(deja, ecrit, "le refus nomme le dossier deja surveille");
    assert_eq!(cas, Emboitement::Meme);
    assert_eq!(
        chemin, ecrit,
        "le dossier choisi s'ecrit comme l'utilisateur ecrit les siens"
    );
    assert_eq!(
        std::fs::read_to_string(&config).expect("configuration relisible"),
        avant,
        "un refus ne fait pas gagner une ligne a la configuration"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// La barre finale ne fait pas un autre dossier. Le selecteur en rend une sur
/// certains chemins, et un utilisateur qui la voit apparaitre ne comprendrait
/// pas que le meme dossier passe deux fois.
#[test]
fn une_barre_finale_ne_fait_pas_un_autre_dossier() {
    let bac = bac_a_sable("barre");
    let developer = bac.join("Developer");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    let ecrit = tilde(&developer);
    let config = config_qui_surveille(&bac, std::slice::from_ref(&ecrit));

    let avec_barre = PathBuf::from(format!("{}/", developer.display()));
    let (_, deja, cas) = refus(ajouter(&config, &avec_barre));
    assert_eq!(deja, ecrit);
    assert_eq!(cas, Emboitement::Meme);

    std::fs::remove_dir_all(&bac).ok();
}

/// Le chemin absolu du meme dossier, la ou la configuration porte un `~`.
///
/// C'est l'ecriture que le selecteur du systeme rend TOUJOURS : sans le
/// depliement, aucun doublon ne serait jamais vu.
#[test]
fn le_chemin_absolu_d_un_dossier_ecrit_avec_un_tilde_est_refuse() {
    let bac = bac_a_sable("absolu");
    let developer = bac.join("Developer");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    let ecrit = tilde(&developer);
    assert!(
        ecrit.starts_with("~/"),
        "la configuration doit bien porter un ~ pour que ce test ait un sens, obtenu : {ecrit}"
    );
    let config = config_qui_surveille(&bac, std::slice::from_ref(&ecrit));

    // Ce que le selecteur rend : le chemin deplie, tel qu'il est sur le disque.
    let (_, deja, cas) = refus(ajouter(&config, &developer));
    assert_eq!(deja, ecrit);
    assert_eq!(cas, Emboitement::Meme);

    std::fs::remove_dir_all(&bac).ok();
}

/// Un lien symbolique qui pointe sur un dossier surveille designe ce dossier.
///
/// Le systeme le sait, et lui seul : aucune comparaison de texte ne rapproche
/// `~/raccourci` de `~/Developer`. C'est la raison d'etre de la resolution que
/// FR-075 demande.
#[test]
fn un_lien_symbolique_vers_un_dossier_surveille_est_refuse() {
    let bac = bac_a_sable("lien");
    let developer = bac.join("Developer");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    let ecrit = tilde(&developer);
    let config = config_qui_surveille(&bac, std::slice::from_ref(&ecrit));

    let raccourci = bac.join("raccourci");
    std::os::unix::fs::symlink(&developer, &raccourci).expect("lien symbolique de test");

    let (_, deja, cas) = refus(ajouter(&config, &raccourci));
    assert_eq!(
        deja, ecrit,
        "le refus nomme le dossier surveille, pas le lien qu'on vient de choisir"
    );
    assert_eq!(cas, Emboitement::Meme);

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- l'emboitement, dans les deux sens ---------- */

/// Un dossier DEDANS un dossier surveille : ses depots remontent deja de la.
#[test]
fn un_dossier_contenu_dans_un_dossier_surveille_est_refuse_en_le_nommant() {
    let bac = bac_a_sable("contenu");
    let developer = bac.join("Developer");
    let dedans = developer.join("vibecode-traker-app");
    std::fs::create_dir_all(&dedans).expect("dossier emboite");
    let ecrit = tilde(&developer);
    let config = config_qui_surveille(&bac, std::slice::from_ref(&ecrit));

    let (_, deja, cas) = refus(ajouter(&config, &dedans));
    assert_eq!(
        deja, ecrit,
        "le refus dit lequel des deux est deja surveille"
    );
    assert_eq!(cas, Emboitement::Contenu);

    std::fs::remove_dir_all(&bac).ok();
}

/// Un dossier qui CONTIENT un dossier surveille : deux racines emboitees
/// cartographieraient les memes depots deux fois.
#[test]
fn un_dossier_qui_contient_un_dossier_surveille_est_refuse_en_le_nommant() {
    let bac = bac_a_sable("contient");
    let developer = bac.join("Developer");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    let ecrit = tilde(&developer);
    let config = config_qui_surveille(&bac, std::slice::from_ref(&ecrit));

    // Le dossier personnel lui-meme, qui contient le dossier surveille.
    let (_, deja, cas) = refus(ajouter(&config, &bac));
    assert_eq!(deja, ecrit);
    assert_eq!(
        cas,
        Emboitement::Contient,
        "les deux sens d'emboitement ne se disent pas pareil : l'un se corrige en \
         choisissant autre chose, l'autre en retirant la racine etroite"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- ce qui doit passer ---------- */

/// Un voisin non emboite s'ajoute, et c'est tout l'interet d'une regle qui juge
/// sur les composants du chemin.
#[test]
fn un_dossier_voisin_et_non_emboite_est_accepte() {
    let bac = bac_a_sable("voisin");
    let developer = bac.join("Developer");
    let sites = bac.join("Sites");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    std::fs::create_dir_all(&sites).expect("dossier voisin");
    let config = config_qui_surveille(&bac, &[tilde(&developer)]);

    let ajout = ajouter(&config, &sites);
    assert!(
        matches!(ajout, Ajout::Ajoute { .. }),
        "un voisin n'est pas un doublon, obtenu : {ajout:?}"
    );
    assert!(
        Config::load(&config)
            .expect("configuration lisible")
            .roots
            .contains(&tilde(&sites)),
        "le voisin est bien inscrit"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un dossier dont le nom COMMENCE par celui d'un dossier surveille n'est pas
/// dedans.
///
/// C'est le piege exact d'une comparaison de texte : `~/Developer` prefixe
/// `~/Developer-2` caractere par caractere, et refuserait un dossier
/// parfaitement legitime en nommant un coupable qui n'en est pas un.
#[test]
fn un_dossier_dont_le_nom_commence_par_celui_d_un_surveille_n_est_pas_dedans() {
    let bac = bac_a_sable("prefixe");
    let developer = bac.join("Developer");
    let voisin = bac.join("Developer-2");
    std::fs::create_dir_all(&developer).expect("dossier surveille");
    std::fs::create_dir_all(&voisin).expect("dossier voisin");
    let config = config_qui_surveille(&bac, &[tilde(&developer)]);

    let ajout = ajouter(&config, &voisin);
    assert!(
        matches!(ajout, Ajout::Ajoute { .. }),
        "« Developer-2 » n'est pas dans « Developer », obtenu : {ajout:?}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- la regle, sans passer par un fichier ---------- */

/// Le premier dossier surveille qui s'oppose est celui qui est nomme, et il est
/// nomme dans l'ecriture de la configuration.
///
/// POURQUOI cette exigence-la : refuser en disant `/Users/lea/Developer` a
/// quelqu'un qui a ecrit `~/Developer` l'enverrait chercher dans sa
/// configuration une ligne qui n'y est pas sous cette forme.
#[test]
fn le_refus_nomme_le_coupable_dans_l_ecriture_de_la_configuration() {
    let bac = bac_a_sable("ecriture");
    let developer = bac.join("Developer");
    std::fs::create_dir_all(&developer).expect("dossier surveille");

    let surveilles = vec![(tilde(&developer), developer.clone())];
    let conflit = deja_surveille(&surveilles, &developer).expect("le doublon devait etre vu");
    assert_eq!(conflit.deja, tilde(&developer));
    assert!(
        conflit.deja.starts_with('~'),
        "le coupable se nomme comme l'utilisateur l'a ecrit, obtenu : {}",
        conflit.deja
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Une racine qui n'existe plus sur le disque continue de s'opposer a elle-meme.
///
/// Le systeme ne sait pas resoudre un chemin qui ne mene nulle part : la regle
/// retombe alors sur une normalisation de composants. Sans cela, un dossier
/// renomme pourrait etre ajoute deux fois, et la configuration porterait deux
/// lignes pour un seul dossier introuvable.
#[test]
fn une_racine_introuvable_s_oppose_encore_a_elle_meme() {
    let bac = bac_a_sable("introuvable");
    let disparu = bac.join("Developer-renomme");
    let surveilles = vec![(tilde(&disparu), disparu.clone())];

    let avec_barre = PathBuf::from(format!("{}/", disparu.display()));
    let conflit = deja_surveille(&surveilles, &avec_barre)
        .expect("un dossier absent s'oppose encore a lui-meme");
    assert_eq!(conflit.cas, Emboitement::Meme);

    std::fs::remove_dir_all(&bac).ok();
}

/// Une liste vide ne s'oppose a rien : il n'y a pas de coupable a nommer.
#[test]
fn sans_aucun_dossier_surveille_rien_ne_s_oppose() {
    let bac = bac_a_sable("liste-vide");
    assert_eq!(deja_surveille(&[], &bac), None);

    std::fs::remove_dir_all(&bac).ok();
}

/* ---------- la forme que lit l'ecran ---------- */

/// Le refus se lit tel quel dans la fenetre : c'est le contrat du pont, et
/// l'ecran a besoin des trois pieces - le dossier choisi, le coupable, et
/// lequel des deux emboitements - pour dire quoi faire.
#[test]
fn le_refus_se_lit_tel_quel_dans_la_fenetre() {
    let bac = bac_a_sable("forme");
    let developer = bac.join("Developer");
    let dedans = developer.join("app");
    std::fs::create_dir_all(&dedans).expect("dossier emboite");
    let config = config_qui_surveille(&bac, &[tilde(&developer)]);

    let rendu = serde_json::to_value(ajouter(&config, &dedans)).expect("issue serialisable");
    assert_eq!(rendu["issue"], "refuse");
    assert_eq!(rendu["deja"], tilde(&developer));
    assert_eq!(rendu["chemin"], tilde(&dedans));
    assert_eq!(rendu["cas"], "contenu");

    std::fs::remove_dir_all(&bac).ok();
}
