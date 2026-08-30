//! La version que l'application porte (FR-068).
//!
//! Une seule source : le champ `version` de `bureau/Cargo.toml`, lu a la
//! compilation. Le paquet macOS reprend ce numero tel quel - `tauri.conf.json`
//! ne porte deliberement aucun champ `version`, faute de quoi le numero serait
//! ecrit a deux endroits et pourrait diverger d'une release a l'autre.
//!
//! Ce module rend la version DISPONIBLE, et rien de plus. L'ecran Reglages, qui
//! l'affiche a cote du nom de la machine, est l'affaire de la tranche #85 :
//! elle n'a qu'a rendre cette constante par le pont.
//!
//! `bureau/tests/publication.rs` eprouve que ce numero est aussi celui du
//! lecteur embarque et du service d'interface.

/// Le numero de version de l'application, fixe a la compilation.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
