//! Comportement : la machine se declare une fois, et une seule (issues #65,
//! #66, #67).
//!
//! Ce qui se joue en base - la fonction de declaration, la RLS, la signature du
//! jeton - est eprouve dans `daemon/tests/declaration.rs`, contre la vraie pile
//! Supabase, et le parcours complet dans `bureau/tests/identite.rs`. Ce qui
//! se verifie ici, c'est la DECISION du poste : que faire de l'identifiant
//! conserve, ce qu'on reprend d'une configuration deja presente, et surtout ce
//! qu'on ne fait jamais - redeclarer une machine que la base reconnait.
//!
//! Toujours sur des fichiers temporaires : un test ne doit ni ecrire dans la
//! configuration de la machine, ni ouvrir son trousseau.

use bureau::machine::{
    aligner_l_adresse_de_la_base, au_trousseau, geste_de_reprise, identite_conservee,
    nom_de_la_machine, poser_l_identite, reprise_de_la_configuration, AuTrousseau, EtatMachine,
    Geste, Reprise,
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
    // inscription. Une machine que la base RECONNAIT ne se redeclare pas, quoi
    // que le trousseau reponde.
    for trousseau in [
        AuTrousseau::Present,
        AuTrousseau::Absent,
        AuTrousseau::Refuse("refus".to_string()),
    ] {
        let geste = geste_de_reprise(MACHINE, presente(), || trousseau.clone());
        assert!(
            !matches!(geste, Geste::Redeclarer),
            "un identifiant que la base reconnait ne mene jamais a une inscription, \
             obtenu : {geste:?}"
        );
    }
}

/// FR-056 : une base remise a zero. L'identifiant ne designe plus rien, la
/// machine se redeclare - et le trousseau reste ferme jusque-la.
///
/// Rien a proteger ici : aucune ligne ne correspond a cet identifiant, donc
/// aucune revocation ne peut etre contournee. Et le trousseau n'a rien a dire
/// d'une machine que le compte ne connait plus - la fermeture qui panique le
/// prouve.
#[test]
fn un_identifiant_perdu_se_redeclare_sans_ouvrir_le_trousseau() {
    let geste = geste_de_reprise(MACHINE, DansLaBase::Inconnue, || {
        panic!("le trousseau ne doit pas s'ouvrir pour une machine que le compte ne connait plus")
    });

    assert_eq!(geste, Geste::Redeclarer);
}

/// FR-021 : une machine revoquee s'annonce, et ne se redeclare pas sous une
/// autre identite. Le trousseau reste ferme : rien a y chercher pour une
/// machine dont les ecritures viennent d'etre coupees.
///
/// C'est la distinction qui porte le plus de consequences de cette tranche : la
/// revocation est une decision de l'utilisateur, et la redeclarer la defairait -
/// machine neuve, jeton neuf, battement repris - par le seul fait de rouvrir
/// l'application.
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

/// La regle de #66, prise par tous les bouts a la fois : de tout ce que la base
/// peut repondre, croise a tout ce que le trousseau peut rendre, un seul couple
/// mene a une inscription - l'identifiant que la base ne reconnait plus.
///
/// POURQUOI cette table plutot que les cas un a un : un variant ajoute a
/// `DansLaBase` - « suspendue », « expiree » - traverserait les tests
/// precedents sans les faire tomber. Ici, il faudra decider ce qu'on en fait,
/// et l'ecrire.
#[test]
fn seule_une_machine_que_la_base_ne_connait_plus_se_redeclare() {
    let trousseaux = [
        AuTrousseau::Present,
        AuTrousseau::Absent,
        AuTrousseau::Refuse("acces refuse par l'utilisateur".to_string()),
    ];
    let vues = [
        (DansLaBase::Inconnue, true),
        (
            DansLaBase::Presente {
                label: "MacBook de Yarma".to_string(),
            },
            false,
        ),
        (
            DansLaBase::Revoquee {
                label: "MacBook de Yarma".to_string(),
            },
            false,
        ),
    ];

    for (vue, attendu) in vues {
        for trousseau in &trousseaux {
            let geste = geste_de_reprise(MACHINE, vue.clone(), || trousseau.clone());
            assert_eq!(
                matches!(geste, Geste::Redeclarer),
                attendu,
                "pour {vue:?} avec {trousseau:?}, obtenu : {geste:?}"
            );
        }
    }
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
        EtatMachine::Redeclaree {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
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

    // Et celle de la redeclaration, que la fenetre doit annoncer autrement que
    // la revocation (#66) : c'est sur ce nom d'etat qu'elle fait la difference.
    assert_eq!(
        serde_json::to_value(EtatMachine::Redeclaree {
            machine_id: MACHINE.to_string(),
            label: "MacBook de Yarma".to_string(),
        })
        .expect("etat serialisable"),
        serde_json::json!({
            "etat": "redeclaree",
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

/* ---------- reprendre une machine deja appairee en ligne de commande (#67) --- */

/// FR-057 : ce qu'une configuration de ligne de commande donne a l'application -
/// l'identifiant de machine et les dossiers surveilles.
///
/// C'est le chemin de migration, et il n'en existe pas d'autre : sans lui,
/// l'application declarerait une seconde machine a cote de celle qui bat deja.
#[test]
fn une_configuration_de_ligne_de_commande_rend_sa_machine_et_ses_dossiers() {
    let chemin = chemin_temporaire("reprise");
    std::fs::create_dir_all(chemin.parent().expect("dossier de test")).expect("dossier de test");
    std::fs::write(
        &chemin,
        format!(
            "# la configuration qu'a ecrite `vibemap pair`\n\
             supabase_url = \"http://base-heritee.invalid\"\n\
             machine_id = \"{MACHINE}\"\n\
             label = \"ancien nom\"\n\
             roots = [\"~/Developer\", \"~/Sites\"]\n"
        ),
    )
    .expect("configuration heritee");

    assert_eq!(
        reprise_de_la_configuration(&chemin),
        Some(Reprise {
            machine_id: MACHINE.to_string(),
            dossiers: vec!["~/Developer".to_string(), "~/Sites".to_string()],
        })
    );

    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));
}

/// FR-073 : l'adresse de la base ne se reprend JAMAIS.
///
/// Ce champ est obligatoire dans la configuration du binaire, et il pointe
/// aujourd'hui sur la pile locale de developpement : le reprendre ferait parler
/// l'application publiee a une base de mise au point, sur le poste de chaque
/// utilisateur qui migre.
///
/// La garantie tient par la forme - `Reprise` n'a aucun champ ou loger cette
/// adresse, et l'y ajouter ferait tomber la construction ci-dessus. Ce test
/// ajoute la garantie par la VALEUR : rien de ce qui sort de cette lecture ne
/// porte l'adresse heritee, sous aucun nom.
#[test]
fn la_reprise_ne_rend_jamais_l_adresse_de_la_base() {
    let chemin = chemin_temporaire("reprise-sans-base");
    std::fs::create_dir_all(chemin.parent().expect("dossier de test")).expect("dossier de test");
    std::fs::write(
        &chemin,
        format!(
            "supabase_url = \"http://base-heritee.invalid\"\n\
             machine_id = \"{MACHINE}\"\n\
             roots = [\"~/Developer\"]\n"
        ),
    )
    .expect("configuration heritee");

    let reprise = reprise_de_la_configuration(&chemin).expect("la configuration doit se reprendre");
    let vu = format!("{reprise:?}");

    assert!(
        !vu.contains("base-heritee") && !vu.contains("supabase"),
        "l'adresse de la base ne doit sortir d'ici sous aucun nom, obtenu : {vu}"
    );
    assert!(
        !reprise.dossiers.iter().any(|d| d.contains("://")),
        "les dossiers surveilles ne sont pas des adresses, obtenu : {:?}",
        reprise.dossiers
    );

    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));
}

/// FR-073 : l'adresse heritee s'efface devant celle de l'application, et rien
/// d'autre ne bouge dans le fichier.
///
/// Ce n'est pas une coquetterie : le lecteur embarque lit `supabase_url`, la
/// fenetre non. Sans cet alignement, le poste ecrirait ses cartes dans la pile
/// locale de developpement pendant que la fenetre lirait la base de production,
/// et l'utilisateur verrait sa machine sans jamais voir sa carte.
#[test]
fn l_adresse_heritee_s_aligne_sur_celle_de_l_application() {
    let chemin = chemin_temporaire("alignement");
    std::fs::create_dir_all(chemin.parent().expect("dossier de test")).expect("dossier de test");
    std::fs::write(
        &chemin,
        format!(
            "# ma configuration a moi\n\
             supabase_url = \"http://base-heritee.invalid\"\n\
             machine_id = \"{MACHINE}\"\n\
             roots = [\"~/Developer\", \"~/Sites\"]\n\
             scan_seconds = 42\n"
        ),
    )
    .expect("configuration heritee");

    assert_eq!(
        aligner_l_adresse_de_la_base(&chemin, "https://base-de-l-application.invalid"),
        Ok(true),
        "une adresse qui differe doit etre reecrite"
    );

    let ecrit = std::fs::read_to_string(&chemin).expect("configuration relisible");
    assert!(
        ecrit.contains("https://base-de-l-application.invalid"),
        "obtenu :\n{ecrit}"
    );
    assert!(
        !ecrit.contains("base-heritee.invalid"),
        "l'adresse heritee ne doit plus etre lisible par le lecteur, obtenu :\n{ecrit}"
    );
    assert!(
        ecrit.contains("# ma configuration a moi")
            && ecrit.contains("~/Sites")
            && ecrit.contains("scan_seconds = 42")
            && ecrit.contains(MACHINE),
        "rien d'autre ne doit bouger, obtenu :\n{ecrit}"
    );

    // Et le lancement suivant ne touche plus au disque : la meme adresse ne se
    // reecrit pas mille fois pour une seule migration.
    assert_eq!(
        aligner_l_adresse_de_la_base(&chemin, "https://base-de-l-application.invalid"),
        Ok(false)
    );

    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));
}

/// Sans configuration, il n'y a rien a reprendre : on part sur une declaration
/// neuve. C'est le poste que personne n'a jamais appaire.
#[test]
fn sans_configuration_il_n_y_a_rien_a_reprendre() {
    let chemin = chemin_temporaire("jamais-appaire");
    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_dir_all(chemin.parent().expect("dossier de test"));

    assert_eq!(reprise_de_la_configuration(&chemin), None);
    assert_eq!(identite_conservee(&chemin), None);
}

/// Une configuration qui n'a pas d'identifiant de machine ne se reprend pas non
/// plus : ce poste ne sait pas a quelle machine il appartient, et le seul chemin
/// honnete est la declaration. Ses dossiers, eux, ne bougent pas.
#[test]
fn une_configuration_sans_machine_ne_se_reprend_pas_et_garde_ses_dossiers() {
    let chemin = chemin_temporaire("sans-machine");
    std::fs::create_dir_all(chemin.parent().expect("dossier de test")).expect("dossier de test");
    std::fs::write(&chemin, "roots = [\"~/Developer\", \"~/Sites\"]\n").expect("configuration");

    assert_eq!(reprise_de_la_configuration(&chemin), None);

    // Et la declaration qui suit ecrit l'identite sans emporter les dossiers.
    poser_l_identite(
        &chemin,
        "http://127.0.0.1:54321",
        MACHINE,
        "MacBook de Yarma",
    )
    .expect("l'identite doit s'ecrire");

    assert_eq!(
        reprise_de_la_configuration(&chemin),
        Some(Reprise {
            machine_id: MACHINE.to_string(),
            dossiers: vec!["~/Developer".to_string(), "~/Sites".to_string()],
        }),
        "les dossiers surveilles ne doivent etre ni effaces ni dupliques (FR-057)"
    );

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
