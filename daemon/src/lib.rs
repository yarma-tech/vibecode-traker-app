//! Daemon local de Vibe Map.
//!
//! Il observe ce que font les agents sur le poste et pousse des metadonnees
//! vers Supabase. Aucun contenu de fichier ne sort jamais d'ici.

mod appairage;
mod config;
pub mod trousseau;

pub use appairage::{appairer, AppairageError, Identite};
pub use config::{Config, ConfigError};

use chrono::{DateTime, Utc};
use serde_json::json;

/// Ce qui peut mal se passer en parlant a Supabase.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Supabase est injoignable : {0}")]
    Injoignable(#[from] reqwest::Error),

    #[error("Supabase a refuse la requete (code {code}) : {corps}")]
    Refuse { code: u16, corps: String },

    /// Aucune ligne touchee. Trois causes possibles, indiscernables d'ici :
    /// la machine a ete revoquee, supprimee, ou le jeton ne lui correspond pas.
    /// Le message les nomme toutes plutot que d'en deviner une.
    #[error(
        "la machine {0} n'a pas accepte l'ecriture. Elle a peut-etre ete revoquee \
         ou supprimee depuis l'application web, ou ce jeton ne lui correspond plus. \
         Relance `vibemap pair <code>` avec un nouveau code pour la relier a nouveau."
    )]
    MachineInconnue(String),
}

/// Client d'ecriture vers Supabase.
///
/// Volontairement mince : des requetes HTTP et du JSON, rien de plus.
/// L'ADR 0001 interdit d'empiler une abstraction sur la base.
pub struct Supabase {
    url: String,
    token: String,
    http: reqwest::Client,
}

impl Supabase {
    pub fn new(url: &str, token: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Signale que la machine est vivante a l'instant donne.
    pub async fn announce(
        &self,
        machine_id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let reponse = self
            .http
            .patch(format!("{}/rest/v1/machines?id=eq.{}", self.url, machine_id))
            .header("apikey", &self.token)
            .bearer_auth(&self.token)
            .header("Content-Type", "application/json")
            // On reclame la ligne modifiee : sans cela, une mise a jour ecartee
            // par la RLS ne touche aucune ligne et repond quand meme 204.
            // Le daemon croirait battre alors que rien n'est ecrit.
            .header("Prefer", "return=representation")
            .json(&json!({ "last_seen_at": at }))
            .send()
            .await?;

        let code = reponse.status();
        if !code.is_success() {
            let corps = reponse.text().await.unwrap_or_default();
            return Err(ApiError::Refuse { code: code.as_u16(), corps });
        }

        let lignes: serde_json::Value = reponse.json().await?;
        if lignes.as_array().is_none_or(|l| l.is_empty()) {
            return Err(ApiError::MachineInconnue(machine_id.to_string()));
        }

        Ok(())
    }
}
