//! Comportement : la machine se declare une fois, et une seule (issue #65).
//!
//! Ce qui se joue en base - la fonction de declaration, la RLS, la signature du
//! jeton - est eprouve dans `daemon/tests/declaration.rs`, contre la vraie pile
//! Supabase. Ce qui se verifie ici, c'est la DECISION du poste : que faire de
//! l'identifiant conserve, et surtout ce qu'on ne fait jamais - redeclarer.
//!
//! Toujours sur des fichiers temporaires : un test ne doit ni ecrire dans la
//! configuration de la machine, ni ouvrir son trousseau.

use bureau::machine::{
    au_trousseau, geste_de_reprise, identite_conservee, nom_de_la_machine, poser_l_identite,
    AuTrousseau, EtatMachine, Geste,
};
use std::path::PathBuf;
use vibemap::declaration::{charge_de_declaration, DansLaBase};
use vibemap::trousseau::TrousseauError;

const MACHINE: &str = "11111111-1111-1111-1111-111111111111";

fn chemin_temporaire(quoi: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "bureau-machine-{}-{quoi}/config.toml",
        std::process::id()
    ))
}

fn presente() -> DansLaBase {
    DansLaBase::Presente {
        label: "MacBook de Yarma".to_string(),
    }
}

/// FR-022 : la liste fermee de ce qui sort de la machine ne gagne aucune ligne.
///
/// Sans reseau et sans base : c'est une propriete de la charge elle-meme. Un
/// champ ajoute - un chemin, une racine surveillee, un compte de depots - fait
/// tomber ce test avant que quoi que ce soit ne parte de la machine.
#[test]
fn la_charge_de_declaration_ne_porte_que_le_nom_et_la_plateforme() {
    let charge = charge_de_declaration("MacBook de Yarma", Some("macos"));

    assert_eq!(
        charge,
        serde_json::json!({
            "p_label": "MacBook de Yarma",
            "p_platform": "macos",
        }),
        "la declaration ne transmet que le nom de la machine et sa plateforme"
    );

    let champs: Vec<&str> = charge
        .as_object()
        .expect("la charge est un objet")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        champs,
        vec!["p_label", "p_platform"],
        "deux champs, et pas un de plus : obtenu {champs:?}"
    );

    assert_eq!(
        charge_de_declaration("Mac mini", None),
        serde_json::json!({ "p_label": "Mac mini", "p_platform": null })
    );
}

/// Le chemin ordinaire : la machine est la, son jeton aussi. Rien n'est envoye,
/// rien n'est cree (FR-019).
#[test]
fn une_machine_deja_declaree_se_reprend_sans_rien_envoyer() {
    let geste = geste_de_reprise(MACHINE, presente(), || AuTrousseau::Present);

    assert_eq!(
        geste,
        Geste::Reprendre {
            label: "MacBook de Yarma".to_string()
        },
        "un poste qui se reconnait n'a rien a declarer"
    );
}

/// Le trousseau a ete vide - poste reinstalle, session refaite. Un jeton neuf
/// se redemande pour LA MEME machine.
///
/// C'est ici que se joue FR-055 : sans l'identifiant conserve, ce cas serait une
/// declaration, et la liste des machines gagnerait un doublon a chaque
/// reinstallation.
#[test]
fn un_jeton_absent_se_redemande_pour_la_meme_machine() {
    let geste = geste_de_reprise(MACHINE, presente(), || AuTrousseau::Absent);

    assert_eq!(
        geste,
        Geste::RedemanderLeJeton {
            label: "MacBook de Yarma".to_string()
        },
        "un jeton absent ne declare pas une machine de plus"
    );
}

/// FR-080, la regle la plus lourde de consequences de cette tranche : un acces
/// refuse s'annonce avec de quoi reessayer, et NE REDECLARE RIEN.
///
/// macOS redemande son autorisation des que le programme change, et
/// l'application est un programme different du binaire en ligne de commande :
/// ce refus arrivera au premier lancement de tout poste deja appaire. Le
/// traiter comme un jeton absent creerait un doublon au moment precis ou
/// l'utilisateur hesite devant une boite de dialogue.
#[test]
fn un_trousseau_refuse_s_annonce_et_ne_redeclare_jamais() {
    let geste = geste_de_reprise(MACHINE, presente(), || {
        AuTrousseau::Refuse(
            "le trousseau du systeme est inaccessible : acces refuse par l'utilisateur".to_string(),
        )
    });

    let Geste::Annoncer(EtatMachine::TrousseauRefuse { machine_id, raison }) = geste else {
        panic!("un refus du trousseau doit s'annoncer, obtenu : {geste:?}");
    };

    assert_eq!(machine_id, MACHINE, "la machine d'origine reste la sienne");
    assert!(
        !raison.is_empty(),
        "un refus sans raison ne dit rien a l'utilisateur"
    );

    // Et la preuve par la forme : aucun des gestes possibles n'est une
    // declaration. Le seul chemin qui cree une machine est celui d'un poste
    // SANS identite conservee, et il ne passe pas par ici.
    for trousseau in [
        AuTrousseau::Present,
        AuTrousseau::Absent,
        AuTrousseau::Refuse("refus".to_string()),
    ] {
        let geste = geste_de_reprise(MACHINE, presente(), || trousseau.clone());
        assert!(
            !matches!(geste, Geste::Annoncer(EtatMachine::Declaree { .. })),
            "un identifiant conserve ne mene jamais a une declaration, obtenu : {geste:?}"
        );
    }
}

/// Une base remise a zero : l'identifiant ne designe plus rien. On le dit, et on
/// ne touche pas au trousseau.
///
/// La redeclaration annoncee est la tranche suivante (#66, FR-056). Ce que ce
/// test fige, c'est l'absence de redeclaration SILENCIEUSE - et le fait que la
/// porte reste ouverte : `Inconnue` porte l'identifiant perdu, que #66 aura a
/// remplacer.
#[test]
fn un_identifiant_perdu_se_dit_sans_ouvrir_le_trousseau() {
    let geste = geste_de_reprise(MACHINE, DansLaBase::Inconnue, || {
        panic!("le trousseau ne doit pas s'ouvrir pour une machine que le compte ne connait plus")
    });

    assert_eq!(
        geste,
        Geste::Annoncer(EtatMachine::Inconnue {
            machine_id: MACHINE.to_string()
        })
    );
}

/// FR-021 : une machine revoquee s'annonce, et ne se redeclare pas sous une
/// autre identite. Le trousseau reste ferme : rien a y chercher pour une
/// machine dont les ecritures viennent d'etre coupees.
#[test]
fn une_machine_revoquee_s_annonce_sans_ouvrir_le_trousseau() {
    let geste = geste_de_reprise(
        MACHINE,
        DansLaBase::Revoquee {
            label: "MacBook de Yarma".to_string(),
        },
        || panic!("le trousseau ne doit pas s'ouvrir pour une machine revoquee"),
    );

    assert_eq!(
        geste,
        Geste::Annoncer(EtatMachine::Revoquee {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
        })
    );
}

/// Le jeton de la machine ne transite JAMAIS par la fenetre.
///
/// La forme le garantit - `EtatMachine` n'a pas de champ ou le loger -, et ce
/// test le rend observable : c'est exactement ce que la fenetre recoit.
#[test]
fn ce_que_la_fenetre_recoit_ne_porte_aucun_jeton() {
    let etats = [
        EtatMachine::Reprise {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
        },
        EtatMachine::Declaree {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
        },
        EtatMachine::Revoquee {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
        },
        EtatMachine::Inconnue {
            machine_id: MACHINE.to_string(),
        },
        EtatMachine::TrousseauRefuse {
            machine_id: MACHINE.to_string(),
            raison: "acces refuse".to_string(),
        },
        EtatMachine::Echec {
            raison: "la base ne repond pas".to_string(),
        },
    ];

    for etat in etats {
        let vu = serde_json::to_value(&etat).expect("etat serialisable");
        let objet = vu.as_object().expect("un objet");

        for champ in objet.keys() {
            assert!(
                !champ.contains("token") && !champ.contains("jeton"),
                "aucun champ ne doit porter un jeton, obtenu « {champ} » dans {vu}"
            );
        }
        assert!(
            objet.contains_key("etat"),
            "la fenetre distingue les cas par leur etat, obtenu : {vu}"
        );
    }

    // Et la forme exacte, telle que la page la lit : un nom de champ qui bouge
    // la laisserait muette sans que rien ne casse ici.
    assert_eq!(
        serde_json::to_value(EtatMachine::Reprise {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
        })
        .expect("etat serialisable"),
        serde_json::json!({
            "etat": "reprise",
            "machine_id": MACHINE,
            "label": "MacBook de Yarma",
        })
    );
}

/// FR-055 : l'identifiant se conserve dans la configuration du poste, a cote du
/// jeton, et se relit tel quel au lancement suivant.
#[test]
fn l_identifiant_conserve_se_relit_au_lancement_suivant() {
    let chemin = chemin_temporaire("aller-retour");
    let _ = std::fs::remove_file(&chemin);

    assert_eq!(
        identite_conservee(&chemin),
        None,
        "un poste neuf ne conserve aucune identite"
    );

    poser_l_identite(
        &chemin,
        "http://127.0.0.1:54321",
        MACHINE,
        "MacBook de Yarma",
    )
    .expect("l'identite doit s'ecrire");

    assert_eq!(
        identite_conservee(&chemin).as_deref(),
        Some(MACHINE),
        "c'est cet identifiant, et lui seul, que le poste represente"
    );

    // Aucun secret dans le fichier (FR-018) : le jeton est au trousseau.
    let ecrit = std::fs::read_to_string(&chemin).expect("configuration relisible");
    assert!(
        !ecrit.contains("token"),
        "le jeton n'a rien a faire dans un fichier, obtenu :\n{ecrit}"
    );
    assert!(
        ecrit.contains("roots"),
        "un poste neuf doit surveiller quelque chose, obtenu :\n{ecrit}"
    );

    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));
}

/// Une configuration heritee du binaire en ligne de commande garde ses dossiers
/// surveilles et ses commentaires (FR-057) - et son adresse de base, elle, est
/// remplacee par celle de l'application (FR-073).
#[test]
fn poser_l_identite_ne_perd_ni_les_dossiers_ni_les_commentaires() {
    let chemin = chemin_temporaire("heritee");
    std::fs::create_dir_all(chemin.parent().expect("dossier de test")).expect("dossier de test");
    std::fs::write(
        &chemin,
        "# ma configuration a moi\n\
         supabase_url = \"http://base-heritee.invalid\"\n\
         machine_id = \"22222222-2222-2222-2222-222222222222\"\n\
         label = \"ancien nom\"\n\
         roots = [\"~/Developer\", \"~/Sites\"]\n\
         scan_seconds = 42\n",
    )
    .expect("configuration heritee");

    poser_l_identite(
        &chemin,
        "http://127.0.0.1:54321",
        MACHINE,
        "MacBook de Yarma",
    )
    .expect("l'identite doit s'ecrire");

    let ecrit = std::fs::read_to_string(&chemin).expect("configuration relisible");
    assert!(
        ecrit.contains("~/Sites") && ecrit.contains("~/Developer"),
        "les dossiers surveilles ne doivent pas disparaitre, obtenu :\n{ecrit}"
    );
    assert!(
        ecrit.contains("# ma configuration a moi"),
        "les commentaires de l'utilisateur restent, obtenu :\n{ecrit}"
    );
    assert!(
        ecrit.contains("scan_seconds = 42"),
        "les cadences reglees a la main restent, obtenu :\n{ecrit}"
    );
    assert!(
        !ecrit.contains("base-heritee.invalid"),
        "une configuration heritee ne detourne pas l'adresse de la base (FR-073), obtenu :\n{ecrit}"
    );
    assert_eq!(identite_conservee(&chemin).as_deref(), Some(MACHINE));

    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));
}

/// Une configuration a laquelle il manque un champ ne cache pas l'identifiant
/// qui, lui, est la : le relire est ce qui evite une redeclaration.
#[test]
fn une_configuration_incomplete_rend_quand_meme_son_identifiant() {
    let chemin = chemin_temporaire("incomplete");
    std::fs::create_dir_all(chemin.parent().expect("dossier de test")).expect("dossier de test");
    std::fs::write(&chemin, format!("machine_id = \"{MACHINE}\"\n")).expect("configuration");

    assert_eq!(identite_conservee(&chemin).as_deref(), Some(MACHINE));

    // Un identifiant vide n'est pas un identifiant : le poste ne se reconnait
    // pas, et il vaut mieux declarer que representer du vide.
    std::fs::write(&chemin, "machine_id = \"\"\n").expect("configuration");
    assert_eq!(identite_conservee(&chemin), None);

    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));
}

/// FR-020 : le nom vient du systeme d'exploitation, et il est affichable tel
/// quel. On ne verifie pas SA valeur - elle depend du Mac - mais qu'il y en a
/// toujours une : un nom vide ferait refuser la declaration par la base.
#[test]
fn la_machine_porte_toujours_un_nom() {
    let nom = nom_de_la_machine();

    assert!(!nom.trim().is_empty(), "obtenu : « {nom} »");
    assert!(
        !nom.contains('\n'),
        "le nom s'affiche sur une ligne, obtenu : « {nom} »"
    );
}

/// Ce que le trousseau rend se traduit sans jamais confondre les deux refus.
///
/// Sur un identifiant tire au hasard, pour ne toucher a aucune entree du poste.
/// On compare a ce que le trousseau a REELLEMENT repondu plutot qu'a une valeur
/// attendue d'avance : une machine d'integration sans session graphique rend un
/// acces impossible la ou un Mac rend « aucune entree », et les deux traductions
/// doivent etre justes - c'est precisement leur difference que FR-080 exige de
/// ne pas perdre.
#[test]
fn ce_que_le_trousseau_rend_se_traduit_sans_confondre_absence_et_refus() {
    std::env::remove_var("VIBEMAP_TOKEN");
    let inconnue = format!("machine-de-test-{}-sans-jeton", std::process::id());

    match vibemap::trousseau::lire(&inconnue) {
        Ok(_) => panic!("un identifiant tire au hasard ne doit porter aucun jeton"),
        Err(TrousseauError::Absent(_)) => assert_eq!(
            au_trousseau(&inconnue),
            AuTrousseau::Absent,
            "aucune entree est un jeton ABSENT - il se redemande pour la meme machine"
        ),
        Err(inaccessible) => assert_eq!(
            au_trousseau(&inconnue),
            AuTrousseau::Refuse(inaccessible.to_string()),
            "un trousseau inaccessible est un REFUS - il s'annonce, et ne redeclare rien"
        ),
    }

    // L'echappatoire que le lecteur honore deja : quand elle est posee, le
    // trousseau n'est pas ouvert du tout, et les deux vehicules lisent le meme
    // jeton.
    std::env::set_var("VIBEMAP_TOKEN", "jeton-de-test");
    assert_eq!(au_trousseau(&inconnue), AuTrousseau::Present);
    std::env::remove_var("VIBEMAP_TOKEN");
}
