//! Configuration du daemon.
//!
//! Un daemon n'a pas d'ecran. Quand sa configuration cloche, le message
//! d'erreur est le seul endroit ou l'utilisateur peut comprendre pourquoi :
//! il nomme le champ fautif et dit quoi faire.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(
        "aucune configuration a {0}. \
         Lance `vibemap pair <code>` avec le code affiche par l'application web pour la creer."
    )]
    Introuvable(PathBuf),

    #[error("configuration illisible a {chemin} : {source}")]
    Illisible {
        chemin: PathBuf,
        source: std::io::Error,
    },

    #[error("configuration invalide a {chemin} : {source}")]
    Invalide {
        chemin: PathBuf,
        source: toml::de::Error,
    },

    #[error(
        "un jeton est ecrit en clair dans {0}. Depuis l'appairage, le jeton vit dans le \
         trousseau du systeme : retire la ligne `token` du fichier et relance \
         `vibemap pair <code>` pour relier cette machine."
    )]
    JetonEnClair(PathBuf),
}

fn battement_par_defaut() -> u64 {
    30
}

fn scan_par_defaut() -> u64 {
    300
}

fn racines_par_defaut() -> Vec<String> {
    vec!["~/Developer".to_string()]
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Racine de l'API Supabase.
    pub supabase_url: String,

    /// Reste d'une configuration d'avant l'appairage. Sa presence est une
    /// erreur : le jeton n'a plus rien a faire dans un fichier.
    #[serde(default)]
    token: Option<String>,

    /// Identifiant de cette machine dans la table `machines`.
    pub machine_id: String,

    /// Nom lisible affiche dans l'application.
    pub label: String,

    /// Periode du battement de coeur, en secondes.
    #[serde(default = "battement_par_defaut")]
    pub interval_seconds: u64,

    /// Dossiers ou chercher les repos. Le daemon regarde leurs enfants
    /// directs et retient ceux qui contiennent un `.git`.
    #[serde(default = "racines_par_defaut")]
    pub roots: Vec<String>,

    /// Periode entre deux cartographies, en secondes.
    #[serde(default = "scan_par_defaut")]
    pub scan_seconds: u64,
}

impl Config {
    pub fn load(chemin: &Path) -> Result<Self, ConfigError> {
        let brut = match std::fs::read_to_string(chemin) {
            Ok(contenu) => contenu,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(ConfigError::Introuvable(chemin.to_path_buf()))
            }
            Err(source) => {
                return Err(ConfigError::Illisible {
                    chemin: chemin.to_path_buf(),
                    source,
                })
            }
        };

        let config: Config = toml::from_str(&brut).map_err(|source| ConfigError::Invalide {
            chemin: chemin.to_path_buf(),
            source,
        })?;

        if config.token.is_some() {
            return Err(ConfigError::JetonEnClair(chemin.to_path_buf()));
        }

        Ok(config)
    }

    /// Les racines, avec le `~` remplace par le dossier personnel.
    pub fn racines(&self) -> Vec<PathBuf> {
        let maison = std::env::var("HOME").unwrap_or_default();
        self.roots
            .iter()
            .map(|brut| PathBuf::from(brut.replacen('~', &maison, 1)))
            .collect()
    }

    /// Emplacement par defaut : `~/.config/vibemap/config.toml`.
    pub fn chemin_par_defaut() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
            });
        base.join("vibemap").join("config.toml")
    }
}
