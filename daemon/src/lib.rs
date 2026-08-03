//! Daemon local de Vibe Map.
//!
//! Il observe ce que font les agents sur le poste et pousse des metadonnees
//! vers Supabase. Aucun contenu de fichier ne sort jamais d'ici.

mod appairage;
mod config;
mod plan;
pub mod trousseau;

pub use appairage::{appairer, AppairageError, Identite};
pub use config::{Config, ConfigError};
pub use plan::{scanner, Module, Plan, ScanError};

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

    /// Envoie le plan d'un repo et rend son identifiant.
    ///
    /// Le repo est reconnu par l'empreinte de sa racine : deux scans de suite
    /// mettent a jour la meme ligne. Les modules sont remplaces en entier, ce
    /// qui fait disparaitre de la carte les dossiers disparus du disque.
    pub async fn pousser_plan(
        &self,
        machine_id: &str,
        plan: &crate::Plan,
    ) -> Result<String, ApiError> {
        let repo = self
            .envoyer(
                "repos?on_conflict=machine_id,root_hash",
                Some("resolution=merge-duplicates,return=representation"),
                json!([{
                    "machine_id":     machine_id,
                    "name":           plan.name,
                    "root_hash":      plan.root_hash,
                    "remote_owner":   plan.remote_owner,
                    "remote_url":     plan.remote_url,
                    "current_branch": plan.current_branch,
                    "loc_total":      plan.loc_total,
                    "file_count":     plan.file_count,
                    "scanned_at":     chrono::Utc::now(),
                }]),
            )
            .await?;

        let repo_id = repo[0]["id"]
            .as_str()
            .ok_or_else(|| ApiError::Refuse {
                code: 500,
                corps: format!("pas d'identifiant de repo dans {repo}"),
            })?
            .to_string();

        // On remplace la carte entiere plutot que de calculer un differentiel :
        // un repo compte quelques centaines de dossiers, et un remplacement ne
        // peut pas laisser de dossier fantome derriere lui.
        let effacement = self
            .http
            .delete(format!("{}/rest/v1/modules?repo_id=eq.{}", self.url, repo_id))
            .header("apikey", &self.token)
            .bearer_auth(&self.token)
            .send()
            .await?;

        if !effacement.status().is_success() {
            let code = effacement.status().as_u16();
            let corps = effacement.text().await.unwrap_or_default();
            return Err(ApiError::Refuse { code, corps });
        }

        if !plan.modules.is_empty() {
            let lignes: Vec<_> = plan
                .modules
                .iter()
                .map(|m| {
                    json!({
                        "repo_id":     repo_id,
                        "path":        m.path,
                        "parent_path": m.parent_path,
                        "depth":       m.depth,
                        "loc":         m.loc,
                        "file_count":  m.file_count,
                    })
                })
                .collect();

            self.envoyer("modules", None, json!(lignes)).await?;
        }

        Ok(repo_id)
    }

    async fn envoyer(
        &self,
        chemin: &str,
        prefer: Option<&str>,
        corps: serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/{}", self.url, chemin))
            .header("apikey", &self.token)
            .bearer_auth(&self.token)
            .header("Content-Type", "application/json")
            .header("Prefer", prefer.unwrap_or("return=representation"))
            .json(&corps)
            .send()
            .await?;

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();

        if !code.is_success() {
            return Err(ApiError::Refuse { code: code.as_u16(), corps: texte });
        }

        Ok(serde_json::from_str(&texte).unwrap_or(serde_json::Value::Null))
    }
}
