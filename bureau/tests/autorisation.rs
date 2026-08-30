//! La borne de la seule commande du pont qui recoit quelque chose (FR-071).
//!
//! Toutes les autres commandes sont des gestes nommes, sans argument. Celle-ci
//! recoit une adresse a ouvrir dans le navigateur du systeme, parce que la
//! fenetre est seule a pouvoir la demander a Supabase.
//!
//! Ce qui s'eprouve ici est donc precisement ce qui pourrait deraper : une
//! fenetre - ou une page qui aurait pris sa place - obtiendrait sinon un
//! « ouvre ceci pour moi » sans limite. La borne n'est pas chez l'appelant, qui
//! peut mentir : elle est dans l'application, et ces tests sont ce qui
//! l'empeche de s'elargir en silence.
//!
//! Le second volet - le retour d'autorisation vise l'origine locale fixe -
//! s'eprouve du cote de l'interface (`web/lib/autorisation.test.ts`) et par
//! `tests/url.rs`, qui verifie que cette origine ne bouge pas.

use bureau::autorisation::{verifier_l_adresse, Refus, CHEMIN_D_AUTORISATION};
use bureau::sonde::url_de_la_fenetre;

/// L'adresse d'un projet Supabase heberge : c'est le cas courant.
#[test]
fn une_adresse_d_autorisation_supabase_est_ouverte() {
    assert_eq!(
        verifier_l_adresse(
            "https://abcdefghijklm.supabase.co/auth/v1/authorize?provider=github&redirect_to=http%3A%2F%2F127.0.0.1%3A51789%2Fauth%2Fcallback"
        ),
        Ok(())
    );
}

/// Et celle de la pile locale, qui sert en clair sur la boucle locale.
///
/// Sans ce cas, la mise au point sur un poste de developpement serait
/// impossible : la pile Supabase locale n'a pas de certificat.
#[test]
fn la_pile_locale_en_clair_est_ouverte_aussi() {
    for locale in [
        "http://127.0.0.1:54321/auth/v1/authorize?provider=github",
        "http://localhost:54321/auth/v1/authorize?provider=github",
        "http://[::1]:54321/auth/v1/authorize?provider=github",
    ] {
        assert_eq!(verifier_l_adresse(locale), Ok(()), "refusee : {locale}");
    }
}

/// Rien ne sort de la machine en clair.
///
/// L'adresse porte le defi PKCE et l'adresse de retour : la donner a lire au
/// reseau ferait perdre au flux ce qui le protege.
#[test]
fn une_adresse_en_clair_vers_le_dehors_est_refusee() {
    assert_eq!(
        verifier_l_adresse("http://abcdefghijklm.supabase.co/auth/v1/authorize?provider=github"),
        Err(Refus::EnClairHorsDeLaMachine)
    );
    // Un hote qui commence comme la boucle locale n'est pas la boucle locale.
    assert_eq!(
        verifier_l_adresse("http://127.0.0.1.piege.test/auth/v1/authorize"),
        Err(Refus::EnClairHorsDeLaMachine)
    );
}

/// Une adresse bien formee qui ne mene pas a une autorisation ne s'ouvre pas.
///
/// C'est ce qui separe « ouvrir l'autorisation » d'un « ouvre ceci pour moi ».
#[test]
fn une_adresse_qui_ne_vise_pas_une_autorisation_est_refusee() {
    for ailleurs in [
        "https://github.com/",
        "https://example.test/piege",
        "https://abcdefghijklm.supabase.co/rest/v1/machines",
        // Le chemin se TERMINE par l'endroit d'autorisation sans en etre un :
        // un test qui se contenterait d'un suffixe laisserait passer celle-ci.
        "https://piege.test/redirect?to=/auth/v1/authorize",
        "https://piege.test/ailleurs/auth/v1/authorize",
        // Et la casse ne fait pas un chemin different pour un serveur, mais
        // elle en fait un ici : on n'ouvre que ce qu'on reconnait exactement.
        "https://abcdefghijklm.supabase.co/AUTH/V1/AUTHORIZE",
    ] {
        assert_eq!(
            verifier_l_adresse(ailleurs),
            Err(Refus::PasUneAutorisation),
            "acceptee alors qu'elle ne vise pas {CHEMIN_D_AUTORISATION} : {ailleurs}"
        );
    }
}

/// Une adresse qui donne a lire un hote et en vise un autre est refusee.
///
/// `https://vrai.supabase.co@piege.test/auth/v1/authorize` part chez
/// `piege.test`. C'est le contournement classique d'un controle qui se
/// contenterait de chercher un nom dans l'adresse.
#[test]
fn une_adresse_qui_cache_sa_destination_est_refusee() {
    assert_eq!(
        verifier_l_adresse("https://abcdefghijklm.supabase.co@piege.test/auth/v1/authorize"),
        Err(Refus::AdresseSuspecte)
    );
    assert_eq!(
        verifier_l_adresse("https:///auth/v1/authorize"),
        Err(Refus::AdresseSuspecte),
        "une adresse sans hote ne mene nulle part"
    );
    assert_eq!(
        verifier_l_adresse("https://abcdefghijklm.supabase.co/auth/v1/authorize\nSet-Cookie: x=1"),
        Err(Refus::AdresseSuspecte),
        "une adresse legitime ne porte ni saut de ligne ni espace"
    );
}

/// Ce qui n'est pas une adresse web ne s'ouvre pas.
///
/// Un `file://` ferait ouvrir un fichier de la machine, un `javascript:` ferait
/// executer du code : ni l'un ni l'autre n'est une page d'autorisation.
#[test]
fn ce_qui_n_est_pas_une_adresse_web_est_refuse() {
    for hors_sujet in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/html,<script>1</script>",
        "vibemap://auth/v1/authorize",
        "/auth/v1/authorize",
        "",
    ] {
        assert_eq!(
            verifier_l_adresse(hors_sujet),
            Err(Refus::ProtocoleRefuse),
            "acceptee : {hors_sujet}"
        );
    }
}

/// Un refus se dit, et il nomme quelque chose.
///
/// La fenetre affiche cette phrase telle quelle (FR-016) : un refus muet
/// laisserait l'ecran de connexion sans rien a lire et sans rien a reprendre.
#[test]
fn tout_refus_porte_une_phrase_lisible() {
    for refus in [
        Refus::ProtocoleRefuse,
        Refus::EnClairHorsDeLaMachine,
        Refus::AdresseSuspecte,
        Refus::PasUneAutorisation,
    ] {
        let phrase = refus.to_string();
        assert!(phrase.len() > 20, "refus trop court : {phrase}");
        assert!(
            phrase.contains("Vibe Map"),
            "un refus dit qui refuse : {phrase}"
        );
    }
}

/// L'adresse de retour que la fenetre demande est celle que la fenetre charge.
///
/// Les deux valeurs sont ecrites a deux endroits - ici en Rust, et dans
/// `web/lib/autorisation.ts` - et le fournisseur d'identite ne connait que
/// celle qui lui a ete declaree. Ce test est ce qui les empeche de diverger en
/// silence, comme `pont.rs` le fait pour l'origine du pont.
#[test]
fn l_adresse_de_retour_de_l_interface_est_l_origine_de_la_fenetre() {
    let interface = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/lib/autorisation.ts"),
    )
    .expect("le module d'autorisation de l'interface");

    assert!(
        interface.contains("export const ORIGINE_LOCALE"),
        "l'interface doit nommer l'origine de retour une seule fois, et l'exporter"
    );
    assert!(
        interface.contains(&format!("\"{}\"", url_de_la_fenetre())),
        "l'interface doit demander un retour vers {}",
        url_de_la_fenetre()
    );
    assert!(
        interface.contains("\"/auth/callback\""),
        "le retour d'autorisation vise l'echangeur de `web/app/auth/callback`"
    );
}

/// La pile Supabase accepte ce retour, sans quoi il n'aboutirait jamais.
///
/// Une adresse de retour absente de la liste blanche n'est pas refusee : elle
/// est IGNOREE, et le code atterrit sur `site_url`, ou personne ne l'echange.
/// La fenetre attendrait un retour deja perdu, sans rien a annoncer - c'est
/// exactement le genre de panne qu'aucun ecran ne sait raconter.
#[test]
fn la_pile_supabase_accepte_le_retour_vers_l_origine_de_la_fenetre() {
    let config = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../supabase/config.toml"),
    )
    .expect("la configuration de la pile Supabase");

    let attendue = format!("\"{}/**\"", url_de_la_fenetre());
    assert!(
        config.contains(&attendue),
        "`additional_redirect_urls` doit porter {attendue}"
    );
    assert!(
        config.contains("[auth.external.github]"),
        "le fournisseur GitHub doit etre declare : c'est par lui qu'on entre"
    );
}
