//! Rangement du jeton de machine dans le trousseau du systeme.
//!
//! Le jeton ne transite jamais par un fichier de configuration : il est ecrit
//! ici a l'appairage, et relu a chaque demarrage. Sur macOS c'est le trousseau
//! d'ouverture de session, sur Linux le service de secrets du bureau.

const SERVICE: &str = "fr.yarma.vibemap";

#[derive(Debug, thiserror::Error)]
pub enum TrousseauError {
    #[error("le trousseau du systeme est inaccessible : {0}")]
    Inaccessible(#[from] keyring::Error),

    #[error(
        "aucun jeton au trousseau pour la machine {0}. \
         Lance `vibemap pair <code>` avec un code affiche par l'application web."
    )]
    Absent(String),
}

pub fn ranger(machine_id: &str, token: &str) -> Result<(), TrousseauError> {
    keyring::Entry::new(SERVICE, machine_id)?.set_password(token)?;
    Ok(())
}

pub fn lire(machine_id: &str) -> Result<String, TrousseauError> {
    match keyring::Entry::new(SERVICE, machine_id)?.get_password() {
        Ok(jeton) => Ok(jeton),
        Err(keyring::Error::NoEntry) => Err(TrousseauError::Absent(machine_id.to_string())),
        Err(autre) => Err(TrousseauError::Inaccessible(autre)),
    }
}

/// Efface le jeton d'une machine desappairee.
pub fn oublier(machine_id: &str) -> Result<(), TrousseauError> {
    match keyring::Entry::new(SERVICE, machine_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(autre) => Err(TrousseauError::Inaccessible(autre)),
    }
}
