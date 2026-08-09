//! Critere d'acceptation #14.4 : `vibemap --version` affiche la version du paquet.
//!
//! La release et la formule Homebrew installent un binaire ; l'utilisateur doit
//! pouvoir verifier ce qu'il a. La version affichee vient de `Cargo.toml`, donc
//! d'une seule source, et le test s'en assure sans coder la valeur en dur.

use std::process::Command;

/// Le chemin du binaire compile, fourni par cargo aux tests d'integration.
const BINAIRE: &str = env!("CARGO_BIN_EXE_vibemap");

/// La version attendue est celle du paquet, prise a la compilation du test.
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn version_longue_affiche_la_version_du_paquet() {
    let sortie = Command::new(BINAIRE)
        .arg("--version")
        .output()
        .expect("le binaire doit s'executer");

    assert!(sortie.status.success(), "--version doit sortir en succes");

    let texte = String::from_utf8_lossy(&sortie.stdout);
    assert!(
        texte.contains(VERSION),
        "la sortie doit contenir la version {VERSION}, obtenu : {texte}"
    );
    assert!(
        texte.contains("vibemap"),
        "la sortie doit nommer le programme, obtenu : {texte}"
    );
}

#[test]
fn version_courte_affiche_la_meme_chose() {
    let sortie = Command::new(BINAIRE)
        .arg("-V")
        .output()
        .expect("le binaire doit s'executer");

    assert!(sortie.status.success(), "-V doit sortir en succes");

    let texte = String::from_utf8_lossy(&sortie.stdout);
    assert!(
        texte.contains(VERSION),
        "la version courte doit contenir la version {VERSION}, obtenu : {texte}"
    );
}
