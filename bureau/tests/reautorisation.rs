//! Comportement : signaler une autorisation manquante, et la redemander
//! (FR-061, issue #74).
//!
//! macOS refuse la lecture de `~/Documents`, `~/Desktop`, `~/Downloads` et des
//! volumes externes tant que l'utilisateur ne l'a pas accordee a
//! l'application. Un dossier ajoute peut donc devenir illisible - a l'ajout, ou
//! plus tard, le systeme pouvant retirer un acces deja accorde. Les deux se
//! disent sur la ligne du dossier, et les deux se reparent du meme geste.
//!
//! POURQUOI ce geste-la : un refus deja donne ne se redemande pas au systeme,
//! qui ne repose plus la question. Ce que l'utilisateur designe LUI-MEME au
//! selecteur, en revanche, lui est accorde. Redemander l'autorisation, c'est
//! donc rouvrir le selecteur sur ce meme dossier - et c'est aussi ce qui garde
//! la borne du pont : aucun chemin ne vient de la fenetre.
//!
//! Le selecteur lui-meme ne se teste pas : c'est une fenetre du systeme, et
//! personne ne clique dedans ici. Ce qu'il rend est passe en parametre, et tout
//! ce qui vient APRES se teste - c'est-a-dire tout ce qui peut casser.
//!
//! Toujours sur des dossiers temporaires : un test ne doit rien devoir a ce qui
//! vit sur la machine, ni toucher a la configuration de l'utilisateur.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use bureau::dossiers::{
    ajouter, dossiers_surveilles, redemander, Ajout, Lisibilite, Reautorisation, Surveillance,
};

fn bac_a_sable(quoi: &str) -> PathBuf {
    let chemin = std::env::temp_dir().join(format!("bureau-reautor-{}-{quoi}", std::process::id()));
    std::fs::remove_dir_all(&chemin).ok();
    std::fs::create_dir_all(&chemin).expect("bac a sable de test");
    chemin
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

/// Ce que macOS fait d'un dossier protege : il est bien la, et il ne s'ouvre
/// pas.
fn fermer(chemin: &Path) {
    std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(0o000))
        .expect("fermeture du dossier");
}

/// Ce que l'utilisateur obtient en le designant au selecteur du systeme.
fn ouvrir(chemin: &Path) {
    std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(0o755))
        .expect("ouverture du dossier");
}

/// La lisibilite du premier dossier de la liste rendue.
fn premiere_lisibilite(surveillance: &Surveillance) -> Lisibilite {
    let Surveillance::Lue { dossiers, .. } = surveillance else {
        panic!("la liste rendue devait etre lue, obtenu : {surveillance:?}");
    };
    dossiers
        .first()
        .expect("la liste rendue porte le dossier en cause")
        .lisibilite
}

/* ---------- le refus se voit, a l'ajout comme plus tard ---------- */

/// Un dossier lisible a l'ajout et ferme ensuite se signale a la demande
/// suivante (FR-061).
///
/// C'est la moitie du « y compris lorsque le refus survient apres l'ajout » :
/// macOS peut retirer un acces qu'il avait accorde, et un ecran qui aurait note
/// la lisibilite une fois pour toutes au moment de l'ajout afficherait encore le
/// compte d'avant. La lisibilite se releve a chaque demande, et c'est ce qui la
/// rend vraie.
#[test]
fn un_dossier_ferme_apres_l_ajout_se_signale_a_la_demande_suivante() {
    let bac = bac_a_sable("ferme-apres");
    let config = config_qui_surveille(&bac, &[]);
    let protege = bac.join("Documents");
    std::fs::create_dir_all(protege.join("carte").join(".git")).expect("un depot dans le dossier");

    let ajout = ajouter(&config, &protege);
    let Ajout::Ajoute { surveillance, .. } = &ajout else {
        panic!("l'ajout devait aboutir, obtenu : {ajout:?}");
    };
    assert_eq!(
        premiere_lisibilite(surveillance),
        Lisibilite::Lisible,
        "au moment de l'ajout, le dossier s'ouvre encore"
    );

    // Le systeme retire l'acces, apres coup.
    fermer(&protege);
    let plus_tard = dossiers_surveilles(&config);
    let vu = premiere_lisibilite(&plus_tard);

    // Rouvrir avant d'assener quoi que ce soit : un test qui echoue ne doit pas
    // laisser un dossier illisible derriere lui.
    ouvrir(&protege);
    std::fs::remove_dir_all(&bac).ok();

    assert_eq!(
        vu,
        Lisibilite::AutorisationRefusee,
        "un acces retire apres l'ajout se dit sur la ligne, et ne se donne pas pour un dossier vide"
    );
}

/// Un dossier renomme dit « introuvable », jamais « autorisation manquante » -
/// et les deux se lisent cote a cote dans la meme liste.
///
/// Ce ne sont ni la meme cause ni le meme geste de reparation : l'un se retire
/// ou se recree, l'autre se redemande au systeme. Les confondre enverrait
/// l'utilisateur reparer ce qui n'est pas casse.
#[test]
fn un_dossier_renomme_et_un_dossier_ferme_ne_se_disent_pas_de_la_meme_facon() {
    let bac = bac_a_sable("deux-causes");
    let renomme = bac.join("Developer-renomme");
    let protege = bac.join("Documents");
    std::fs::create_dir_all(&protege).expect("dossier protege");
    fermer(&protege);

    let config = config_qui_surveille(
        &bac,
        &[
            &renomme.display().to_string(),
            &protege.display().to_string(),
        ],
    );
    let rendu = dossiers_surveilles(&config);

    ouvrir(&protege);
    std::fs::remove_dir_all(&bac).ok();

    let Surveillance::Lue { dossiers, .. } = rendu else {
        panic!("une configuration lisible rend la liste des dossiers");
    };
    assert_eq!(
        dossiers[0].lisibilite,
        Lisibilite::Introuvable,
        "obtenu : {:?}",
        dossiers[0]
    );
    assert_eq!(
        dossiers[1].lisibilite,
        Lisibilite::AutorisationRefusee,
        "obtenu : {:?}",
        dossiers[1]
    );
    assert_eq!(
        dossiers[0].depots, None,
        "on ne compte pas ce qui n'est pas la"
    );
    assert_eq!(
        dossiers[1].depots, None,
        "on ne compte pas non plus ce qu'on ne peut pas lire"
    );
}

/* ---------- redemander l'autorisation ---------- */

/// Un selecteur referme sans choix ne change aucune autorisation.
///
/// Rien n'a ete demande au systeme, et l'ecran n'a rien a annoncer : un geste
/// repris n'est pas un echec.
#[test]
fn un_selecteur_referme_sans_choix_ne_change_aucune_autorisation() {
    let bac = bac_a_sable("annule");
    let protege = bac.join("Documents");
    std::fs::create_dir_all(&protege).expect("dossier protege");
    let config = config_qui_surveille(&bac, &[&protege.display().to_string()]);

    assert_eq!(
        redemander(&config, &protege.display().to_string(), None),
        Reautorisation::Annulee
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// La lisibilite se releve APRES le choix, jamais avant.
///
/// POURQUOI c'est tout l'enjeu : c'est le choix lui-meme qui accorde l'acces.
/// Une lisibilite relevee trop tot - ou retenue de la liste affichee - dirait
/// encore « refuse » d'un dossier qui vient de s'ouvrir, et le bouton semblerait
/// sans effet alors que l'autorisation vient d'etre accordee. La preuve : le
/// meme dossier, redemande deux fois, rend deux issues differentes selon ce que
/// le disque repond a cet instant.
#[test]
fn la_lisibilite_se_releve_apres_le_choix_et_non_avant() {
    let bac = bac_a_sable("apres-le-choix");
    let protege = bac.join("Documents");
    std::fs::create_dir_all(&protege).expect("dossier protege");
    let ecrit = protege.display().to_string();
    let config = config_qui_surveille(&bac, &[&ecrit]);

    fermer(&protege);
    let refusee = redemander(&config, &ecrit, Some(&protege));

    // Ce que le selecteur du systeme accorde quand l'utilisateur designe le
    // dossier lui-meme.
    ouvrir(&protege);
    let accordee = redemander(&config, &ecrit, Some(&protege));

    std::fs::remove_dir_all(&bac).ok();

    let Reautorisation::Refusee {
        chemin,
        surveillance,
    } = refusee
    else {
        panic!(
            "un dossier qui ne s'ouvre toujours pas ne s'annonce pas accorde, obtenu : {refusee:?}"
        );
    };
    assert_eq!(chemin, ecrit, "le refus nomme le dossier en cause");
    assert_eq!(
        premiere_lisibilite(&surveillance),
        Lisibilite::AutorisationRefusee,
        "la liste rendue avec le refus dit encore ce qui cloche"
    );

    let Reautorisation::Accordee {
        chemin,
        surveillance,
    } = accordee
    else {
        panic!("un dossier redevenu lisible s'annonce accorde, obtenu : {accordee:?}");
    };
    assert_eq!(chemin, ecrit);
    assert_eq!(
        premiere_lisibilite(&surveillance),
        Lisibilite::Lisible,
        "la ligne reprend son affichage normal du meme coup, sans rien redemander"
    );
}

/// Designer un AUTRE dossier n'accorde rien au dossier attendu.
///
/// Le systeme n'accorde l'acces qu'a ce qui a ete designe : annoncer un succes
/// ici ferait croire l'affaire reglee, et l'utilisateur retrouverait la meme
/// ligne barree au rafraichissement suivant. Le refus nomme les DEUX dossiers -
/// celui qu'on attendait et celui qui a ete choisi -, sans quoi on ne saurait
/// pas ce qui a mal tourne.
#[test]
fn designer_un_autre_dossier_n_accorde_rien_au_dossier_attendu() {
    let bac = bac_a_sable("autre");
    let protege = bac.join("Documents");
    let voisin = bac.join("Bureau");
    std::fs::create_dir_all(&protege).expect("dossier protege");
    std::fs::create_dir_all(&voisin).expect("dossier voisin");
    let ecrit = protege.display().to_string();
    let config = config_qui_surveille(&bac, &[&ecrit]);

    let rendu = redemander(&config, &ecrit, Some(&voisin));

    let Reautorisation::AutreDossier { attendu, choisi } = rendu else {
        panic!("un autre dossier designe n'est pas une autorisation accordee, obtenu : {rendu:?}");
    };
    assert_eq!(attendu, ecrit);
    assert_eq!(
        choisi,
        voisin.display().to_string(),
        "le refus nomme aussi ce qui a ete choisi a la place"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Le meme dossier ecrit autrement reste le meme dossier.
///
/// La configuration porte souvent une barre finale ou un lien symbolique pose
/// ailleurs ; le selecteur, lui, rend un chemin resolu. Une comparaison de texte
/// les dirait differents et refuserait l'autorisation que l'utilisateur vient
/// d'accorder - au bon dossier. C'est le meme jugement que le doublon (FR-075).
#[test]
fn le_meme_dossier_ecrit_autrement_reste_le_meme_dossier() {
    let bac = bac_a_sable("ecritures");
    let vrai = bac.join("Documents");
    std::fs::create_dir_all(&vrai).expect("dossier protege");
    let raccourci = bac.join("raccourci");
    std::os::unix::fs::symlink(&vrai, &raccourci).expect("lien symbolique de test");

    // Ecrit avec une barre finale, comme un chemin colle a la main.
    let avec_barre = format!("{}/", vrai.display());
    let config = config_qui_surveille(&bac, &[&avec_barre]);
    let rendu = redemander(&config, &avec_barre, Some(&vrai));
    assert!(
        matches!(rendu, Reautorisation::Accordee { .. }),
        "une barre finale ne fait pas deux dossiers, obtenu : {rendu:?}"
    );

    // Surveille par le lien, designe par le vrai chemin : le systeme n'accorde
    // l'acces qu'a un seul endroit, et c'est le meme.
    let par_le_lien = raccourci.display().to_string();
    let config = config_qui_surveille(&bac, &[&par_le_lien]);
    let rendu = redemander(&config, &par_le_lien, Some(&vrai));
    assert!(
        matches!(rendu, Reautorisation::Accordee { .. }),
        "un lien symbolique et sa cible designent le meme endroit, obtenu : {rendu:?}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Un dossier qui n'est plus surveille n'a plus d'autorisation a redemander.
///
/// L'ecran a pu vieillir - le dossier vient d'etre retire ailleurs. Accorder une
/// autorisation a un dossier qui ne figure plus dans la liste ne servirait a
/// rien, et un succes annonce ici enverrait attendre une cartographie qui
/// n'aura pas lieu.
#[test]
fn un_dossier_qui_n_est_plus_surveille_n_a_plus_d_autorisation_a_redemander() {
    let bac = bac_a_sable("plus-surveille");
    let orphelin = bac.join("Documents");
    std::fs::create_dir_all(&orphelin).expect("dossier orphelin");
    let config = config_qui_surveille(&bac, &[]);

    let rendu = redemander(&config, &orphelin.display().to_string(), Some(&orphelin));

    let Reautorisation::Inconnu {
        chemin,
        surveillance,
    } = rendu
    else {
        panic!("un dossier hors de la liste n'a rien a redemander, obtenu : {rendu:?}");
    };
    assert_eq!(chemin, orphelin.display().to_string());
    let Surveillance::Lue { dossiers, .. } = surveillance else {
        panic!("le refus rend quand meme la liste du poste, pour que l'ecran se remette d'aplomb");
    };
    assert!(
        dossiers.is_empty(),
        "la liste rendue est celle du poste, obtenu : {dossiers:?}"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// Sans configuration lisible, on ne sait pas quel dossier etait attendu : il
/// n'y a pas d'autorisation a redemander ici, et la raison le dit.
#[test]
fn sans_configuration_il_n_y_a_pas_d_autorisation_a_redemander() {
    let bac = bac_a_sable("sans-config");
    let dossier = bac.join("Documents");
    std::fs::create_dir_all(&dossier).expect("dossier de test");

    let rendu = redemander(
        &bac.join("jamais-ecrite.toml"),
        &dossier.display().to_string(),
        Some(&dossier),
    );

    let Reautorisation::Inconnu { surveillance, .. } = rendu else {
        panic!("sans configuration, rien ne peut etre accorde, obtenu : {rendu:?}");
    };
    let Surveillance::SansConfiguration { raison } = surveillance else {
        panic!("« on ne sait pas ce qui est surveille » n'est pas « rien n'est surveille »");
    };
    assert!(
        !raison.is_empty(),
        "un refus sans raison ne dit rien a l'utilisateur"
    );

    std::fs::remove_dir_all(&bac).ok();
}

/// La forme que lit l'ecran.
///
/// Elle compte autant que le fond : c'est le contrat du pont, et un nom de champ
/// qui bouge laisserait le bouton muet sans que rien ne casse ailleurs. Les cinq
/// issues doivent surtout rester distinctes : elles appellent cinq gestes
/// differents, et l'ecran ne doit pas pouvoir les confondre.
#[test]
fn les_issues_de_la_redemande_se_lisent_telles_quelles_dans_la_fenetre() {
    let annulee = serde_json::to_value(Reautorisation::Annulee).expect("issue serialisable");
    assert_eq!(annulee, serde_json::json!({ "issue": "annulee" }));

    let bac = bac_a_sable("forme");
    let protege = bac.join("Documents");
    std::fs::create_dir_all(protege.join("carte").join(".git")).expect("un depot dans le dossier");
    let voisin = bac.join("Bureau");
    std::fs::create_dir_all(&voisin).expect("dossier voisin");
    let ecrit = protege.display().to_string();
    let config = config_qui_surveille(&bac, &[&ecrit]);

    let accordee =
        serde_json::to_value(redemander(&config, &ecrit, Some(&protege))).expect("serialisable");
    assert_eq!(accordee["issue"], "accordee");
    assert_eq!(accordee["chemin"], ecrit);
    assert_eq!(
        accordee["surveillance"]["etat"], "lue",
        "l'ecran lit la liste au meme endroit que d'habitude"
    );
    assert_eq!(
        accordee["surveillance"]["dossiers"][0]["lisibilite"],
        "lisible"
    );
    assert_eq!(accordee["surveillance"]["dossiers"][0]["depots"], 1);

    let autre =
        serde_json::to_value(redemander(&config, &ecrit, Some(&voisin))).expect("serialisable");
    assert_eq!(autre["issue"], "autre_dossier");
    assert_eq!(autre["attendu"], ecrit);
    assert_eq!(autre["choisi"], voisin.display().to_string());

    fermer(&protege);
    let refusee =
        serde_json::to_value(redemander(&config, &ecrit, Some(&protege))).expect("serialisable");
    ouvrir(&protege);
    std::fs::remove_dir_all(&bac).ok();

    assert_eq!(refusee["issue"], "refusee");
    assert_eq!(refusee["chemin"], ecrit);
    assert_eq!(
        refusee["surveillance"]["dossiers"][0]["lisibilite"], "autorisation_refusee",
        "la ligne continue de dire ce qui cloche"
    );
}
