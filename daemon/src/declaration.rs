//! La machine se declare elle-meme, sans code d'appairage (issue #65).
//!
//! Soeur d'`appairage.rs`, et volontairement de la meme forme : toute la
//! logique vit dans la base, et ce module ne fait qu'appeler ses fonctions et
//! traduire leurs refus en messages lisibles. Ce qui change tient en une ligne :
//! l'appelant presente la SESSION de l'utilisateur au lieu d'un code court.
//!
//! ## Deux identites, et elles ne se melangent jamais
//!
//! Le jeton de session est celui de l'utilisateur : il ouvre la porte de son
//! compte, et il sert ici a declarer une machine ou a redemander le jeton d'une
//! machine deja declaree. Le jeton de MACHINE, lui, ne vaut que pour une ligne
//! de `machines`, ne represente personne d'autre, et va directement au trousseau
//! du systeme. Rien ici ne les fait se croiser : les fonctions qui recoivent un
//! jeton de session ne rendent jamais celui d'une machine sans le dire, et
//! l'appelant du cote du poste ne le laisse jamais remonter jusqu'a la fenetre.
//!
//! ## Ce qui sort de la machine (FR-022)
//!
//! Le nom de la machine et sa plateforme. Rien d'autre : ni chemin, ni racine
//! surveillee, ni nom de depot. `charge_de_declaration` est le seul endroit ou
//! cette liste s'ecrit, pour qu'un champ de plus ne puisse pas s'y glisser sans
//! qu'un test tombe.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::appairage::Identite;

#[derive(Debug, thiserror::Error)]
pub enum DeclarationError {
    #[error("Supabase est injoignable : {0}")]
    Injoignable(#[from] reqwest::Error),

    /// Message venu de la base : il dit deja quoi faire, on le laisse passer
    /// tel quel.
    #[error("{0}")]
    Refuse(String),

    #[error("reponse de la base incomprehensible : {0}")]
    Incomprehensible(String),
}

/// Ce qui sort de la machine au moment de la declaration, et rien d'autre
/// (FR-022).
///
/// Une fonction plutot qu'un litteral pose au milieu de l'appel : c'est ce qui
/// rend la liste eprouvable. Un champ de plus - un chemin, une racine, un
/// compte de depots - se verrait ici, et le test qui compare cette charge a
/// l'attendu tomberait avant que quoi que ce soit ne parte.
pub fn charge_de_declaration(label: &str, platform: Option<&str>) -> Value {
    json!({
        "p_label": label,
        "p_platform": platform,
    })
}

/// Declare cette machine sur le compte de la session presentee (FR-017).
///
/// Rend l'identite complete, jeton de machine compris : c'est l'appelant qui le
/// range au trousseau, sous le compte que forme l'identifiant de la machine
/// (FR-018).
pub async fn declarer(
    url: &str,
    jeton_de_session: &str,
    label: &str,
    platform: Option<&str>,
) -> Result<Identite, DeclarationError> {
    let corps = appeler(
        url,
        jeton_de_session,
        "declarer_machine",
        charge_de_declaration(label, platform),
    )
    .await?;

    serde_json::from_str(&corps).map_err(|_| DeclarationError::Incomprehensible(corps))
}

/// Un jeton neuf pour une machine DEJA declaree.
///
/// POURQUOI cela existe : le jeton vit au trousseau du systeme, et un trousseau
/// peut etre vide - poste reinstalle, session refaite. L'identifiant conserve a
/// cote (FR-055) permet alors de redemander un jeton pour LA MEME machine,
/// plutot que d'en declarer une seconde. C'est exactement ce qui evite qu'une
/// liste de machines gagne un doublon a chaque reinstallation.
///
/// A ne PAS confondre avec le refus du trousseau (FR-080) : un acces refuse
/// n'est pas un jeton absent, et il ne passe pas par ici.
pub async fn jeton_de_machine(
    url: &str,
    jeton_de_session: &str,
    machine_id: &str,
) -> Result<String, DeclarationError> {
    let corps = appeler(
        url,
        jeton_de_session,
        "jeton_de_machine",
        json!({ "p_machine": machine_id }),
    )
    .await?;

    serde_json::from_str::<String>(&corps).map_err(|_| DeclarationError::Incomprehensible(corps))
}

/// Ce que la base repond quand on lui represente l'identifiant conserve
/// (FR-055).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DansLaBase {
    /// La machine est la, et elle emet.
    Presente { label: String },
    /// La machine est la, et elle a ete revoquee depuis le web (FR-021).
    Revoquee { label: String },
    /// L'identifiant conserve ne designe plus rien sur ce compte : machine
    /// supprimee, ou base remise a zero.
    Inconnue,
}

#[derive(Debug, Deserialize)]
struct LigneMachine {
    label: String,
    revoked_at: Option<String>,
}

/// Represente l'identifiant conserve, et dit ce que la base en fait (FR-019).
///
/// C'est LA question que l'application pose a chaque lancement, et c'est ce qui
/// tient la promesse « pas de doublon » : tant que cet identifiant designe une
/// machine du compte, il n'y a rien a declarer.
///
/// La RLS fait le tri toute seule : cette lecture ne peut rendre que les
/// machines de la session presentee, meme si le filtre ne le precise pas.
pub async fn representer(
    url: &str,
    jeton_de_session: &str,
    machine_id: &str,
) -> Result<DansLaBase, DeclarationError> {
    let url_propre = url.trim_end_matches('/');

    let reponse = reqwest::Client::new()
        .get(format!(
            "{url_propre}/rest/v1/machines?id=eq.{machine_id}&select=label,revoked_at"
        ))
        .header("apikey", jeton_de_session)
        .bearer_auth(jeton_de_session)
        .send()
        .await?;

    let code = reponse.status();
    let corps = reponse.text().await?;

    if !code.is_success() {
        return Err(DeclarationError::Refuse(message_de_postgres(&corps)));
    }

    let lignes: Vec<LigneMachine> =
        serde_json::from_str(&corps).map_err(|_| DeclarationError::Incomprehensible(corps))?;

    let Some(ligne) = lignes.into_iter().next() else {
        return Ok(DansLaBase::Inconnue);
    };

    Ok(match ligne.revoked_at {
        Some(_) => DansLaBase::Revoquee { label: ligne.label },
        None => DansLaBase::Presente { label: ligne.label },
    })
}

/// L'appel d'une fonction de la base avec le jeton de session, et rien d'autre.
///
/// Le jeton sert aussi de cle d'API, comme partout ailleurs dans le lecteur :
/// l'application n'a pas de cle publique a connaitre pour se declarer.
async fn appeler(
    url: &str,
    jeton_de_session: &str,
    fonction: &str,
    charge: Value,
) -> Result<String, DeclarationError> {
    let url = url.trim_end_matches('/');

    let reponse = reqwest::Client::new()
        .post(format!("{url}/rest/v1/rpc/{fonction}"))
        .header("apikey", jeton_de_session)
        .bearer_auth(jeton_de_session)
        .json(&charge)
        .send()
        .await?;

    let code = reponse.status();
    let corps = reponse.text().await?;

    if !code.is_success() {
        return Err(DeclarationError::Refuse(message_de_postgres(&corps)));
    }

    Ok(corps)
}

/// PostgREST enveloppe l'erreur SQL dans un objet JSON. On en extrait le
/// message ecrit dans la migration, qui est deja redige pour un humain.
fn message_de_postgres(corps: &str) -> String {
    serde_json::from_str::<Value>(corps)
        .ok()
        .and_then(|v| {
            v["message"]
                .as_str()
                .or_else(|| v["hint"].as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| corps.to_string())
}
