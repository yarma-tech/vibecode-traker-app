//! Contexte de test partage : parle a la pile Supabase locale, pas a un simulacre.
//!
//! C'est deliberé. L'ADR 0001 exige un test d'integration qui verifie le contrat
//! entre les structures Rust et le schema SQL. Un mock ne verifierait rien.

use serde_json::{json, Value};
use uuid::Uuid;

pub struct TestContext {
    pub url: String,
    pub service_key: String,
    pub user_token: String,
    pub user_id: String,
    http: reqwest::Client,
}

impl TestContext {
    /// Cree un utilisateur neuf et ouvre une session pour lui.
    ///
    /// Chaque test a son propre utilisateur : deux tests ne peuvent pas se
    /// marcher dessus, et la RLS se teste pour de vrai.
    pub async fn new() -> Self {
        let url = std::env::var("VIBEMAP_TEST_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:54321".to_string());
        let service_key = std::env::var("VIBEMAP_TEST_SERVICE_KEY").expect(
            "VIBEMAP_TEST_SERVICE_KEY manquante. Lance `supabase status` et exporte la cle service_role.",
        );
        let http = reqwest::Client::new();

        let email = format!("{}@test.vibemap.local", Uuid::new_v4());
        let password = "motdepasse-de-test-1234";

        let created: Value = http
            .post(format!("{url}/auth/v1/admin/users"))
            .header("apikey", &service_key)
            .bearer_auth(&service_key)
            .json(&json!({ "email": email, "password": password, "email_confirm": true }))
            .send()
            .await
            .expect("creation de l'utilisateur de test")
            .json()
            .await
            .expect("reponse JSON de creation");

        let user_id = created["id"]
            .as_str()
            .unwrap_or_else(|| panic!("pas d'id utilisateur dans {created}"))
            .to_string();

        let session: Value = http
            .post(format!("{url}/auth/v1/token?grant_type=password"))
            .header("apikey", &service_key)
            .json(&json!({ "email": email, "password": password }))
            .send()
            .await
            .expect("ouverture de session")
            .json()
            .await
            .expect("reponse JSON de session");

        let user_token = session["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("pas de jeton dans {session}"))
            .to_string();

        Self { url, service_key, user_token, user_id, http }
    }

    /// Insere une machine appartenant a l'utilisateur du contexte.
    pub async fn create_machine(&self, label: &str) -> String {
        let rows: Value = self
            .http
            .post(format!("{}/rest/v1/machines", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .header("Prefer", "return=representation")
            .json(&json!({ "user_id": self.user_id, "label": label }))
            .send()
            .await
            .expect("insertion de la machine")
            .json()
            .await
            .expect("reponse JSON d'insertion");

        rows[0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("pas d'id machine dans {rows}"))
            .to_string()
    }

    /// Relit `last_seen_at` en contournant la RLS, pour verifier ce qui a ete ecrit.
    pub async fn last_seen_at(&self, machine_id: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        let rows: Value = self
            .http
            .get(format!(
                "{}/rest/v1/machines?id=eq.{}&select=last_seen_at",
                self.url, machine_id
            ))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .send()
            .await
            .expect("lecture de la machine")
            .json()
            .await
            .expect("reponse JSON de lecture");

        rows[0]["last_seen_at"].as_str().map(|s| {
            s.parse::<chrono::DateTime<chrono::Utc>>()
                .expect("last_seen_at doit etre une date valide")
        })
    }
}
