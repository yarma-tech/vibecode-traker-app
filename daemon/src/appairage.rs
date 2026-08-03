//! Echange d'un code d'appairage contre un jeton de machine.
//!
//! Toute la logique vit dans la base : cette fonction ne fait qu'appeler
//! `appairer_machine` et traduire son refus en message lisible. Le daemon
//! n'a donc aucun secret a connaitre pour se relier, seulement le code court
//! affiche par l'application web.

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum AppairageError {
    #[error("Supabase est injoignable : {0}")]
    Injoignable(#[from] reqwest::Error),

    /// Message venu de la base : il dit deja quoi faire, on le laisse passer tel quel.
    #[error("{0}")]
    Refuse(String),

    #[error("reponse d'appairage incomprehensible : {0}")]
    Incomprehensible(String),
}

/// Ce qu'une machine sait d'elle-meme apres l'appairage.
#[derive(Debug, Deserialize)]
pub struct Identite {
    pub machine_id: String,
    pub label: String,
    /// Ne finit jamais dans un fichier : le daemon le range dans le trousseau.
    pub token: String,
}

pub async fn appairer(
    url: &str,
    anon_key: &str,
    code: &str,
    label: &str,
    platform: Option<&str>,
) -> Result<Identite, AppairageError> {
    let url = url.trim_end_matches('/');

    let reponse = reqwest::Client::new()
        .post(format!("{url}/rest/v1/rpc/appairer_machine"))
        .header("apikey", anon_key)
        .bearer_auth(anon_key)
        .json(&serde_json::json!({
            "p_code": code,
            "p_label": label,
            "p_platform": platform,
        }))
        .send()
        .await?;

    let code_http = reponse.status();
    let corps = reponse.text().await?;

    if !code_http.is_success() {
        return Err(AppairageError::Refuse(message_de_postgres(&corps)));
    }

    serde_json::from_str(&corps).map_err(|_| AppairageError::Incomprehensible(corps))
}

/// PostgREST enveloppe l'erreur SQL dans un objet JSON. On en extrait le
/// message ecrit dans la migration, qui est deja redige pour un humain.
fn message_de_postgres(corps: &str) -> String {
    serde_json::from_str::<serde_json::Value>(corps)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| corps.to_string())
}
