//! L'URL de la fenetre ne depend de rien de ce qui traine sur le poste.
//!
//! Le port est fixe (FR-070) : l'adresse de retour d'autorisation est
//! declaree une fois pour toutes chez le fournisseur d'identite, et le cookie
//! de session est lie a l'origine. Une valeur trouvee sur la machine qui
//! deplacerait cette origine casserait les deux en silence, au prochain
//! lancement.

use bureau::sonde::url_de_la_fenetre;

#[test]
fn la_fenetre_charge_toujours_la_meme_origine_locale() {
    assert_eq!(url_de_la_fenetre(), "http://127.0.0.1:51789");
}

#[test]
fn aucune_configuration_du_poste_ne_deplace_cette_origine() {
    let maison = std::env::temp_dir().join(format!(
        "bureau-test-url-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(maison.join(".vibemap")).expect("faux dossier personnel");
    std::fs::write(
        maison.join(".vibemap/config.toml"),
        "supabase_url = \"http://127.0.0.1:54321\"\nport = 3000\nui_port = 8080\n",
    )
    .expect("fausse configuration de ligne de commande");

    // Les valeurs qu'un poste peut porter, et dont aucune ne doit etre lue :
    // celles d'un serveur de developpement lance a cote, et une configuration
    // heritee du binaire en ligne de commande.
    std::env::set_var("HOME", &maison);
    std::env::set_var("XDG_CONFIG_HOME", &maison);
    std::env::set_var("PORT", "3000");
    std::env::set_var("VIBEMAP_PORT", "8080");
    std::env::set_var("VIBEMAP_URL", "https://vibemap.example.com");

    assert_eq!(
        url_de_la_fenetre(),
        "http://127.0.0.1:51789",
        "la fenetre charge l'origine locale fixe, jamais celle qu'une configuration ou une \
         variable d'environnement propose"
    );

    std::fs::remove_dir_all(&maison).ok();
}
