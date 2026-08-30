//! La machine se declare elle-meme, sans code d'appairage (issues #65, #66).
//!
//! Ces tests traversent la fonction SQL, la RLS de `machines` et la signature
//! du jeton, contre la vraie pile Supabase locale - meme discipline que
//! `appairage.rs`, dont ils sont la suite : ce qui se verifie ici, ce sont des
//! REGLES D'ACCES, et un simulacre ne verifierait rien.
//!
//! Le fil rouge est FR-019 : une machine qui se declare a chaque lancement
//! serait un doublon a chaque lancement. Ce qui l'evite est l'identifiant
//! conserve sur le poste (FR-055), represente a chaque fois - et ces tests
//! rejouent exactement cela.
//!
//! La CHARGE de la declaration (FR-022), elle, ne se verifie pas ici : c'est une
//! propriete du poste, qui n'a besoin d'aucune base, et elle est eprouvee dans
//! `bureau/tests/machine.rs` - qui, lui, tourne dans l'integration continue.

mod common;

use vibemap::declaration::DansLaBase;
use vibemap::declaration::{declarer, jeton_de_machine, representer};

/// Tracer bullet : une session ouverte suffit a declarer la machine, et le
/// jeton rendu sait battre (FR-017).
#[tokio::test]
async fn une_session_suffit_a_declarer_la_machine_et_son_jeton_bat() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("une session ouverte doit suffire a declarer la machine");

    assert_eq!(identite.label, "MacBook de Yarma");

    // Aucun code n'a ete demande ni consomme : la machine est simplement la.
    let machines = ctx.machines_visibles().await;
    assert_eq!(machines.len(), 1, "obtenu : {machines:?}");
    assert_eq!(machines[0]["label"], "MacBook de Yarma");
    assert_eq!(machines[0]["platform"], "macos");

    // Le jeton recu doit suffire a battre, sans rien d'autre.
    vibemap::Supabase::new(&ctx.url, &identite.token)
        .announce(&identite.machine_id, chrono::Utc::now())
        .await
        .expect("le jeton de declaration doit permettre d'ecrire");

    assert!(
        ctx.last_seen_at(&identite.machine_id).await.is_some(),
        "le battement doit avoir atteint la base"
    );
}

/// FR-019 : trois lancements de plus, et la liste ne porte toujours qu'une
/// machine.
///
/// C'est le critere d'acceptation de l'issue, joue tel quel : l'application
/// represente l'identifiant qu'elle a conserve, la base le confirme, et il n'y
/// a rien a declarer.
#[tokio::test]
async fn une_machine_deja_declaree_ne_se_dedouble_pas() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("premiere declaration");

    for lancement in 1..=3 {
        let vue = representer(&ctx.url, &ctx.user_token, &identite.machine_id)
            .await
            .expect("la base doit repondre a l'identifiant conserve");

        assert_eq!(
            vue,
            DansLaBase::Presente {
                label: "MacBook de Yarma".to_string()
            },
            "au lancement {lancement}, l'identifiant conserve doit toujours designer la machine"
        );
    }

    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "trois lancements de plus ne doivent pas ajouter de machine, obtenu : {machines:?}"
    );
}

/// FR-055, l'autre moitie : le trousseau vide ne fabrique pas un doublon.
///
/// L'identifiant conserve permet de redemander un jeton pour LA MEME machine.
/// Sans lui, chaque reinstallation en creerait une nouvelle - et c'est
/// exactement le doublon que le PM a vecu.
#[tokio::test]
async fn un_trousseau_vide_redemande_un_jeton_sans_creer_de_machine() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("premiere declaration");

    let neuf = jeton_de_machine(&ctx.url, &ctx.user_token, &identite.machine_id)
        .await
        .expect("un jeton neuf doit pouvoir etre signe pour une machine a soi");

    // Et il vaut pour LA MEME machine : c'est ce qui distingue une reprise
    // d'une redeclaration.
    vibemap::Supabase::new(&ctx.url, &neuf)
        .announce(&identite.machine_id, chrono::Utc::now())
        .await
        .expect("le jeton neuf doit battre pour la machine d'origine");

    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "redemander un jeton ne cree aucune machine, obtenu : {machines:?}"
    );
}

/// Une base remise a zero : l'identifiant conserve ne designe plus rien.
///
/// Ce que ce test fige, c'est que la base sait le DIRE - `DansLaBase::Inconnue`
/// et rien d'autre -, et qu'une simple lecture ne cree rien au passage. C'est
/// cette reponse-la, et elle seule, qui autorise l'application a redeclarer
/// (#66, FR-056).
#[tokio::test]
async fn une_base_remise_a_zero_ne_fabrique_pas_de_doublon_silencieux() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("premiere declaration");

    ctx.effacer_machine(&identite.machine_id).await;

    let vue = representer(&ctx.url, &ctx.user_token, &identite.machine_id)
        .await
        .expect("la base doit repondre, meme pour un identifiant qu'elle ne connait plus");

    assert_eq!(
        vue,
        DansLaBase::Inconnue,
        "un identifiant qui ne designe plus rien doit se dire tel quel"
    );

    // Et representer ne cree rien : la liste reste vide tant que personne n'a
    // decide de redeclarer.
    let machines = ctx.machines_visibles().await;
    assert!(
        machines.is_empty(),
        "representer un identifiant perdu ne doit rien creer, obtenu : {machines:?}"
    );

    // Le jeton de cette machine ne se resigne pas non plus : elle n'existe plus.
    let erreur = jeton_de_machine(&ctx.url, &ctx.user_token, &identite.machine_id)
        .await
        .expect_err("une machine effacee ne doit plus donner de jeton");
    assert!(
        erreur.to_string().contains("aucun compte connu"),
        "le message doit dire que la machine n'existe plus, obtenu : {erreur}"
    );
}

/// Sans session, rien ne se declare. C'est la RLS et les droits de la fonction
/// qui le disent, pas l'appelant.
#[tokio::test]
async fn une_declaration_sans_session_est_refusee() {
    let ctx = common::TestContext::new().await;

    let erreur = declarer(
        &ctx.url,
        &ctx.anon_key,
        "Machine de personne",
        Some("macos"),
    )
    .await
    .expect_err("une declaration sans session ne doit pas passer");

    assert!(
        !erreur.to_string().is_empty(),
        "un refus sans message ne dit rien a l'utilisateur"
    );

    let machines = ctx.machines_visibles().await;
    assert!(
        machines.is_empty(),
        "aucune machine ne doit avoir ete creee, obtenu : {machines:?}"
    );
}

/// Un jeton de MACHINE ne declare pas de machine.
///
/// C'est la policy `machines_insert_own` qui refuse - `machine_du_jeton() is
/// null` -, et non une garde recopiee dans la fonction. Sans cela, un jeton
/// echappe d'un poste pourrait peupler le compte de machines fantomes.
#[tokio::test]
async fn un_jeton_de_machine_ne_declare_pas_une_seconde_machine() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("premiere declaration");

    let erreur = declarer(&ctx.url, &identite.token, "Mac mini", Some("macos"))
        .await
        .expect_err("un jeton de machine ne doit pas pouvoir declarer une machine");
    assert!(
        !erreur.to_string().is_empty(),
        "un refus sans message ne dit rien"
    );

    // Il ne signe pas non plus de jeton pour lui-meme : un jeton de machine ne
    // fabrique pas de jetons.
    let refus = jeton_de_machine(&ctx.url, &identite.token, &identite.machine_id)
        .await
        .expect_err("un jeton de machine ne doit pas pouvoir en signer un autre");
    assert!(
        !refus.to_string().is_empty(),
        "un refus sans message ne dit rien"
    );

    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "la liste ne doit pas avoir gagne de machine, obtenu : {machines:?}"
    );
}

/// La RLS SEULE, sans passer par aucune fonction.
///
/// POURQUOI ce test existe a cote du precedent : `declarer_machine` s'appuie
/// deliberement sur la policy `machines_insert_own` plutot que de recopier ses
/// regles. Eprouver la fonction ne prouve pas la policy - un autre garde-fou du
/// corps peut refuser a sa place, et le test resterait vert le jour ou la policy
/// s'affaiblirait. Ici, l'insertion est brute : ce qui refuse ne peut etre que
/// la RLS.
#[tokio::test]
async fn la_rls_seule_refuse_de_creer_une_machine_sans_session_ou_par_un_jeton_de_machine() {
    let ctx = common::TestContext::new().await;

    // Sans session : `anon` n'a aucune ligne a lui.
    ctx.tenter_inserer_machine_avec_jeton(&ctx.anon_key, "Machine de personne")
        .await
        .expect_err("la RLS doit refuser une machine creee sans session");

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("declaration");

    // Avec un jeton de MACHINE : `machine_du_jeton() is null` s'y oppose, alors
    // meme que `auth.uid()` designe bien le proprietaire.
    ctx.tenter_inserer_machine_avec_jeton(&identite.token, "Mac mini")
        .await
        .expect_err("la RLS doit refuser une machine creee par un jeton de machine");

    // Et la session de l'utilisateur, elle, passe : sans quoi ce test se
    // contenterait de constater qu'aucune insertion n'aboutit jamais.
    ctx.tenter_inserer_machine_avec_jeton(&ctx.user_token, "Mac mini")
        .await
        .expect("la session de l'utilisateur cree bien une machine");

    // Le compte est la seconde moitie du test : un refus qui laisserait quand
    // meme une ligne derriere lui ne serait pas un refus.
    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        2,
        "seules la declaration et la session ont cree une machine, obtenu : {machines:?}"
    );
}

/// Le jeton signe vaut pour SA machine, et pour aucune autre - meme au sein du
/// meme compte. C'est la propriete que l'appairage par code avait deja, et que
/// la declaration ne doit pas perdre en route.
#[tokio::test]
async fn le_jeton_declare_n_ecrit_que_sa_propre_machine() {
    let ctx = common::TestContext::new().await;

    let a = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("declaration de A");
    let b = declarer(&ctx.url, &ctx.user_token, "Mac mini", Some("macos"))
        .await
        .expect("declaration de B");

    let resultat = vibemap::Supabase::new(&ctx.url, &a.token)
        .announce(&b.machine_id, chrono::Utc::now())
        .await;

    assert!(
        resultat.is_err(),
        "le jeton de la machine A ne doit pas pouvoir ecrire la ligne de la machine B"
    );
    assert!(
        ctx.last_seen_at(&b.machine_id).await.is_none(),
        "la machine B ne doit pas avoir bouge"
    );
}

/// Le jeton d'une machine d'un AUTRE compte ne se signe pas.
#[tokio::test]
async fn le_jeton_d_une_machine_d_un_autre_compte_ne_se_signe_pas() {
    let sien = common::TestContext::new().await;
    let autre = common::TestContext::new().await;

    let machine = declarer(
        &sien.url,
        &sien.user_token,
        "MacBook de Yarma",
        Some("macos"),
    )
    .await
    .expect("declaration sur le premier compte");

    let erreur = jeton_de_machine(&autre.url, &autre.user_token, &machine.machine_id)
        .await
        .expect_err("un autre compte ne doit pas obtenir le jeton de cette machine");
    assert!(
        erreur.to_string().contains("aucun compte connu"),
        "le message doit dire que la machine n'est pas la sienne, obtenu : {erreur}"
    );

    // Et il ne la voit meme pas : la RLS la lui cache.
    let vue = representer(&autre.url, &autre.user_token, &machine.machine_id)
        .await
        .expect("la lecture doit aboutir, filtree par la RLS");
    assert_eq!(vue, DansLaBase::Inconnue);
}

/// FR-021 : une machine revoquee se dit revoquee, et ne se resigne pas.
///
/// Ce qui compte ici, c'est que la revocation ne se lise PAS comme une machine
/// inconnue : la premiere s'annonce et fait cesser d'emettre, la seconde fait
/// redeclarer (#66, FR-056). Les confondre redeclarerait la machine que
/// l'utilisateur venait justement de couper - et lui rendrait un jeton neuf, ce
/// qui defait la revocation par le seul fait de rouvrir l'application.
#[tokio::test]
async fn une_machine_revoquee_se_dit_revoquee_et_ne_se_resigne_pas() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("declaration");

    ctx.revoquer(&identite.machine_id).await;

    let vue = representer(&ctx.url, &ctx.user_token, &identite.machine_id)
        .await
        .expect("la base doit repondre");
    assert_eq!(
        vue,
        DansLaBase::Revoquee {
            label: "MacBook de Yarma".to_string()
        },
        "une machine revoquee ne doit pas se lire comme une machine inconnue"
    );

    let erreur = jeton_de_machine(&ctx.url, &ctx.user_token, &identite.machine_id)
        .await
        .expect_err("une machine revoquee ne doit pas obtenir de jeton neuf");
    assert!(
        erreur.to_string().contains("revoquee"),
        "le message doit parler de revocation, obtenu : {erreur}"
    );
}

/// La revocation coupe les ecritures, et elle reste lisible comme une revocation
/// (#66, FR-021).
///
/// Les deux moities de l'exigence, dans le meme test parce qu'elles ne valent
/// que l'une avec l'autre : une application qui cesse d'ecrire sans savoir dire
/// pourquoi laisse un poste muet, et une application qui sait dire pourquoi tout
/// en continuant d'ecrire n'a rien revoque du tout.
#[tokio::test]
async fn une_machine_revoquee_cesse_d_ecrire_et_le_motif_reste_une_revocation() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("declaration");

    // Elle bat d'abord : sans cela, ce test se contenterait de constater qu'un
    // battement n'aboutit jamais.
    let poste = vibemap::Supabase::new(&ctx.url, &identite.token);
    poste
        .announce(&identite.machine_id, chrono::Utc::now())
        .await
        .expect("la machine doit battre avant sa revocation");
    let dernier = ctx
        .last_seen_at(&identite.machine_id)
        .await
        .expect("un battement avant revocation");

    ctx.revoquer(&identite.machine_id).await;

    // 1. Elle n'ecrit plus. C'est la RLS qui l'arrete - `revoked_at is null`
    //    dans `machines_update_own` -, pas une garde du lecteur.
    let refus = poste
        .announce(&identite.machine_id, chrono::Utc::now())
        .await
        .expect_err("une machine revoquee ne doit plus pouvoir ecrire");
    assert!(
        !refus.to_string().is_empty(),
        "un refus sans message ne dit rien"
    );
    assert_eq!(
        ctx.last_seen_at(&identite.machine_id).await,
        Some(dernier),
        "le battement refuse ne doit avoir rien ecrit"
    );

    // 2. Et le motif que l'application remonte a sa fenetre reste une
    //    REVOCATION : c'est ce qui l'empeche de redeclarer.
    assert_eq!(
        representer(&ctx.url, &ctx.user_token, &identite.machine_id)
            .await
            .expect("la base doit repondre"),
        DansLaBase::Revoquee {
            label: "MacBook de Yarma".to_string()
        },
    );

    // 3. Et la liste n'a gagne aucune machine de remplacement.
    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "une revocation ne cree aucune machine, obtenu : {machines:?}"
    );
    assert!(
        machines[0]["revoked_at"].is_string(),
        "la seule machine du compte est bien celle qui a ete revoquee, obtenu : {machines:?}"
    );
}

/// FR-056, le critere du PRD joue de bout en bout : base remise a zero, on
/// rouvre, la machine est redeclaree et la liste n'en porte qu'UNE.
///
/// C'est la difference qui compte avec la tranche precedente : elle figeait
/// l'absence de redeclaration silencieuse ; celle-ci exige la redeclaration, et
/// exige qu'elle ne se double pas.
#[tokio::test]
async fn une_base_remise_a_zero_redeclare_une_machine_et_une_seule() {
    let ctx = common::TestContext::new().await;

    let perdue = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("premiere declaration");
    ctx.effacer_machine(&perdue.machine_id).await;

    // Ce que l'application constate au lancement suivant, et la seule reponse
    // qui autorise a redeclarer.
    assert_eq!(
        representer(&ctx.url, &ctx.user_token, &perdue.machine_id)
            .await
            .expect("la base doit repondre"),
        DansLaBase::Inconnue
    );

    let neuve = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("la machine doit pouvoir se redeclarer");

    assert_ne!(
        neuve.machine_id, perdue.machine_id,
        "la machine redeclaree porte une identite neuve"
    );

    let machines = ctx.machines_visibles().await;
    assert_eq!(
        machines.len(),
        1,
        "la liste ne doit porter que la machine redeclaree, obtenu : {machines:?}"
    );
    assert_eq!(machines[0]["id"], neuve.machine_id);

    // Et elle bat : « la carte se repeuple » n'est vrai que si le jeton neuf
    // ecrit pour de bon.
    vibemap::Supabase::new(&ctx.url, &neuve.token)
        .announce(&neuve.machine_id, chrono::Utc::now())
        .await
        .expect("la machine redeclaree doit battre");
    assert!(ctx.last_seen_at(&neuve.machine_id).await.is_some());

    // Et rouvrir encore ne redeclare plus rien : l'identifiant neuf designe
    // maintenant quelque chose (FR-019).
    assert_eq!(
        representer(&ctx.url, &ctx.user_token, &neuve.machine_id)
            .await
            .expect("la base doit repondre"),
        DansLaBase::Presente {
            label: "MacBook de Yarma".to_string()
        }
    );
}

/// La porte de la redeclaration, et ce qui ne doit surtout pas l'ouvrir.
///
/// Depuis #66, `DansLaBase::Inconnue` fait CREER une machine. Tout ce qui se
/// lirait « inconnue » a tort en fabriquerait donc une a chaque lancement : une
/// session expiree, un jeton illisible, une pile qui repond de travers. Ces
/// refus doivent remonter comme des ERREURS - la fenetre les annonce, et rien
/// n'est declare.
#[tokio::test]
async fn une_session_qui_ne_vaut_rien_ne_se_lit_pas_comme_une_identite_perdue() {
    let ctx = common::TestContext::new().await;

    let identite = declarer(&ctx.url, &ctx.user_token, "MacBook de Yarma", Some("macos"))
        .await
        .expect("declaration");

    for (quoi, jeton) in [
        ("sans session", ctx.anon_key.as_str()),
        ("un jeton illisible", "pas-un-jeton-du-tout"),
    ] {
        let erreur = representer(&ctx.url, jeton, &identite.machine_id)
            .await
            .expect_err(&format!(
                "{quoi} ne doit pas se lire comme une identite perdue"
            ));
        assert!(
            !erreur.to_string().is_empty(),
            "{quoi} : un refus sans message ne dit rien"
        );
    }

    // Et la machine, elle, est toujours la : rien de tout cela ne l'a touchee.
    let machines = ctx.machines_visibles().await;
    assert_eq!(machines.len(), 1, "obtenu : {machines:?}");
}
