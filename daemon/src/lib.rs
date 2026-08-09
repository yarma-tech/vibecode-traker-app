//! Daemon local de Vibe Map.
//!
//! Il observe ce que font les agents sur le poste et pousse des metadonnees
//! vers Supabase. Aucun contenu de fichier ne sort jamais d'ici.

mod appairage;
mod config;
pub mod journal;
mod plan;
pub mod trousseau;

pub use appairage::{appairer, AppairageError, Identite};
pub use config::{Config, ConfigError};
pub use plan::{empreinte, scanner, Module, Plan, ScanError};

use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::BTreeMap;

/// Un appel d'outil, pret a partir : plus de chemin absolu, plus de contenu.
#[derive(Debug, Clone)]
pub struct Activite {
    pub session_id: String,
    pub tool_use_id: String,
    /// Dossier contenant le fichier, relatif a la racine du repo.
    pub module_path: String,
    pub file_path: String,
    /// « read » ou « write ».
    pub kind: &'static str,
    pub occurred_at: DateTime<Utc>,
}

/// La consommation d'une session, agregee et prete a partir.
///
/// Les jetons s'additionnent au fil des reponses de l'agent ; le modele et les
/// bornes de temps viennent de ce que ce lot a vu. Le cout se calcule cote base,
/// a partir du modele : un modele inconnu doit montrer les jetons sans cout,
/// jamais un cout faux.
#[derive(Debug, Clone)]
pub struct SessionCout {
    pub session_id: String,
    pub model: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

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

    /// Retrouve un repo deja cartographie par l'empreinte de sa racine.
    ///
    /// Le hook n'a pas la carte du daemon en memoire : il ne connait que le
    /// dossier ou l'outil vient de travailler.
    pub async fn repo_par_empreinte(
        &self,
        machine_id: &str,
        root_hash: &str,
    ) -> Result<Option<String>, ApiError> {
        let reponse = self
            .http
            .get(format!(
                "{}/rest/v1/repos?machine_id=eq.{}&root_hash=eq.{}&select=id",
                self.url, machine_id, root_hash
            ))
            .header("apikey", &self.token)
            .bearer_auth(&self.token)
            .send()
            .await?;

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();

        if !code.is_success() {
            return Err(ApiError::Refuse { code: code.as_u16(), corps: texte });
        }

        let lignes: serde_json::Value = serde_json::from_str(&texte).unwrap_or_default();
        Ok(lignes[0]["id"].as_str().map(str::to_string))
    }

    /// Envoie des appels d'outils, et rend le nombre de lignes reellement posees.
    ///
    /// Les sessions sont annoncees avant leurs evenements : sans leur ligne, la
    /// cle etrangere refuserait tout. Un appel deja connu ne cree rien et ne
    /// gene rien : c'est la contrainte d'unicite qui absorbe le doublon, et
    /// c'est ce qui rend le hook optionnel.
    pub async fn pousser_activite(
        &self,
        machine_id: &str,
        repo_id: &str,
        branche: Option<&str>,
        evenements: &[Activite],
    ) -> Result<usize, ApiError> {
        if evenements.is_empty() {
            return Ok(0);
        }

        // Une session peut arriver en plusieurs lots : on annonce l'etendue vue
        // dans ce lot-ci, la base garde la plus large.
        let mut etendues: BTreeMap<&str, (DateTime<Utc>, DateTime<Utc>)> = BTreeMap::new();
        for evenement in evenements {
            let etendue = etendues
                .entry(&evenement.session_id)
                .or_insert((evenement.occurred_at, evenement.occurred_at));
            etendue.0 = etendue.0.min(evenement.occurred_at);
            etendue.1 = etendue.1.max(evenement.occurred_at);
        }

        for (session_id, (debut, fin)) in etendues {
            self.envoyer(
                "rpc/noter_session",
                Some("return=minimal"),
                json!({
                    "p_id":            session_id,
                    "p_repo_id":       repo_id,
                    "p_machine_id":    machine_id,
                    "p_branch":        branche,
                    "p_started_at":    debut,
                    "p_last_event_at": fin,
                }),
            )
            .await?;
        }

        let lignes: Vec<_> = evenements
            .iter()
            .map(|e| {
                json!({
                    "session_id":  e.session_id,
                    "repo_id":     repo_id,
                    "module_path": e.module_path,
                    "file_path":   e.file_path,
                    "kind":        e.kind,
                    "occurred_at": e.occurred_at,
                    "tool_use_id": e.tool_use_id,
                })
            })
            .collect();

        let poses = self
            .envoyer(
                "activity_events?on_conflict=session_id,tool_use_id",
                Some("resolution=ignore-duplicates,return=representation"),
                json!(lignes),
            )
            .await?;

        Ok(poses.as_array().map(Vec::len).unwrap_or(0))
    }

    /// Annonce la consommation de chaque session, jetons et modele compris.
    ///
    /// Un lot ne transporte que ce qu'il a vu : `noter_session` ajoute ces jetons
    /// au total deja connu. Le cout ne part pas d'ici, il se calcule a la lecture
    /// depuis le modele. Rend le nombre de sessions annoncees.
    pub async fn pousser_cout(
        &self,
        machine_id: &str,
        repo_id: &str,
        branche: Option<&str>,
        sessions: &[SessionCout],
    ) -> Result<usize, ApiError> {
        for session in sessions {
            self.envoyer(
                "rpc/noter_session",
                Some("return=minimal"),
                json!({
                    "p_id":             session.session_id,
                    "p_repo_id":        repo_id,
                    "p_machine_id":     machine_id,
                    "p_branch":         branche,
                    "p_started_at":     session.debut,
                    "p_last_event_at":  session.fin,
                    "p_input":          session.input,
                    "p_output":         session.output,
                    "p_cache_read":     session.cache_read,
                    "p_cache_creation": session.cache_creation,
                    "p_model":          session.model,
                }),
            )
            .await?;
        }

        Ok(sessions.len())
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
