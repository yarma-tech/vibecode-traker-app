// Chaque binaire de test compile ce module en entier mais n'en utilise qu'une
// partie : le reste n'est pas du code mort, il sert ailleurs.
#![allow(dead_code)]

//! Contexte de test partage : parle a la pile Supabase locale, pas a un simulacre.
//!
//! C'est deliberé. L'ADR 0001 exige un test d'integration qui verifie le contrat
//! entre les structures Rust et le schema SQL. Un mock ne verifierait rien.

use serde_json::{json, Value};
use uuid::Uuid;

pub struct TestContext {
    pub url: String,
    pub service_key: String,
    pub anon_key: String,
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
        let anon_key = std::env::var("VIBEMAP_TEST_ANON_KEY").expect(
            "VIBEMAP_TEST_ANON_KEY manquante. Lance `supabase status` et exporte la cle anon.",
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

        Self { url, service_key, anon_key, user_token, user_id, http }
    }

    /// Demande un code d'appairage, comme le ferait la page web.
    pub async fn creer_code(&self) -> String {
        let ligne: Value = self
            .http
            .post(format!("{}/rest/v1/rpc/creer_code_appairage", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(&self.user_token)
            .json(&json!({}))
            .send()
            .await
            .expect("creation du code d'appairage")
            .json()
            .await
            .expect("reponse JSON de creation de code");

        ligne["code"]
            .as_str()
            .unwrap_or_else(|| panic!("pas de code dans {ligne}"))
            .to_string()
    }

    /// Force la date d'expiration d'un code, pour tester le refus sans attendre.
    pub async fn perimer_code(&self, code: &str) {
        self.ecrire(
            &format!("pairing_codes?code=eq.{code}"),
            json!({ "expires_at": "2020-01-01T00:00:00Z" }),
        )
        .await;
    }

    /// Revoque une machine, comme le ferait le bouton des reglages.
    pub async fn revoquer(&self, machine_id: &str) {
        self.ecrire(
            &format!("machines?id=eq.{machine_id}"),
            json!({ "revoked_at": "now()" }),
        )
        .await;
    }

    /// Pose un repo et ses modules sans passer par un scan de disque.
    ///
    /// Les tests d'activite n'ont pas besoin d'un vrai depot : ils ont besoin
    /// d'une carte sur laquelle poser des couleurs.
    pub async fn creer_repo(&self, machine_id: &str, modules: &[&str]) -> String {
        self.creer_repo_avec_empreinte(machine_id, &Uuid::new_v4().to_string(), modules)
            .await
    }

    pub async fn creer_repo_avec_empreinte(
        &self,
        machine_id: &str,
        empreinte: &str,
        modules: &[&str],
    ) -> String {
        let repos: Value = self
            .ecrire_service(
                "repos",
                json!([{
                    "user_id":    self.user_id,
                    "machine_id": machine_id,
                    "name":       "atelier",
                    "root_hash":  empreinte,
                }]),
            )
            .await;

        let repo_id = repos[0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("pas d'id de repo dans {repos}"))
            .to_string();

        let lignes: Vec<Value> = modules
            .iter()
            .map(|path| {
                let parent = match path.rfind('/') {
                    Some(i) => path[..i].to_string(),
                    None => String::new(),
                };
                json!({
                    "repo_id":     repo_id,
                    "path":        path,
                    "parent_path": parent,
                    "depth":       path.split('/').count(),
                    "loc":         100,
                    "file_count":  1,
                })
            })
            .collect();

        self.ecrire_service("modules", json!(lignes)).await;
        repo_id
    }

    /// Relit les evenements d'activite d'un repo en contournant la RLS.
    pub async fn lire_evenements(&self, repo_id: &str) -> Vec<Value> {
        self.lire(&format!(
            "activity_events?repo_id=eq.{repo_id}&select=*&order=occurred_at"
        ))
        .await
        .as_array()
        .cloned()
        .unwrap_or_default()
    }

    /// Relit une session en contournant la RLS.
    pub async fn lire_session(&self, session_id: &str) -> Value {
        self.lire(&format!("sessions?id=eq.{session_id}&select=*")).await[0].clone()
    }

    /// Demande a la base l'etat de chaque module, comme le fera l'ecran.
    pub async fn etat_modules(&self, repo_id: &str, fenetre_secondes: i64) -> Vec<Value> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/etat_modules", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .json(&json!({
                "p_repo_id": repo_id,
                "p_fenetre_secondes": fenetre_secondes,
            }))
            .send()
            .await
            .expect("appel de etat_modules");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "etat_modules a echoue ({code}) : {texte}");

        serde_json::from_str::<Value>(&texte)
            .expect("reponse JSON de etat_modules")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Les conflits ouverts d'un repo, tels que l'ecran les comptera.
    pub async fn conflits(&self, repo_id: &str, fenetre_secondes: i64) -> Vec<Value> {
        self.appeler(
            "conflits",
            json!({ "p_repo_id": repo_id, "p_fenetre_secondes": fenetre_secondes }),
        )
        .await
    }

    /// Les conflits de tous les repos, ramenes a ceux d'une machine.
    ///
    /// La cle de service ignore la RLS et verrait donc les repos de tous les
    /// utilisateurs de la base de test : le filtre remet le test dans son bac.
    pub async fn conflits_de_la_machine(
        &self,
        machine_id: &str,
        fenetre_secondes: i64,
    ) -> Vec<Value> {
        let repos: Vec<String> = self
            .lire(&format!("repos?machine_id=eq.{machine_id}&select=id"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|r| r["id"].as_str().map(str::to_string))
            .collect();

        self.appeler(
            "conflits",
            json!({ "p_repo_id": null, "p_fenetre_secondes": fenetre_secondes }),
        )
        .await
        .into_iter()
        .filter(|ligne| {
            ligne["repo_id"]
                .as_str()
                .is_some_and(|id| repos.iter().any(|r| r == id))
        })
        .collect()
    }

    /// Le nombre d'agents presents sur un repo pendant la fenetre.
    pub async fn agents_actifs(&self, repo_id: &str, fenetre_secondes: i64) -> i64 {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/agents_actifs", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .json(&json!({
                "p_repo_id": repo_id,
                "p_fenetre_secondes": fenetre_secondes,
            }))
            .send()
            .await
            .expect("appel de agents_actifs");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "agents_actifs a echoue ({code}) : {texte}");

        texte.trim().parse().unwrap_or_else(|_| panic!("nombre attendu, recu : {texte}"))
    }

    /// Les worktrees ouverts d'un repo, tels que le plan les lira.
    pub async fn worktrees_ouverts(&self, repo_id: &str) -> Vec<Value> {
        self.appeler("worktrees_ouverts", json!({ "p_repo_id": repo_id }))
            .await
    }

    /// Le releve de consommation d'un repo, tel que le bandeau le lira.
    pub async fn releve_repo(&self, repo_id: &str) -> Value {
        self.appeler("releve_repo", json!({ "p_repo_id": repo_id }))
            .await
            .into_iter()
            .next()
            .unwrap_or(Value::Null)
    }

    /// Lance la purge des evenements et rend le nombre de lignes effacees.
    pub async fn purger(&self) -> i64 {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/purger_activite", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .json(&json!({}))
            .send()
            .await
            .expect("appel de purger_activite");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "purger_activite a echoue ({code}) : {texte}");
        texte.trim().parse().unwrap_or_else(|_| panic!("nombre attendu, recu : {texte}"))
    }

    /// Pose un evenement daté à la main, pour éprouver la purge sans attendre.
    pub async fn poser_evenement_ancien(
        &self,
        repo_id: &str,
        session_id: &str,
        occurred_at: &str,
    ) {
        self.ecrire_service(
            "activity_events",
            json!([{
                "user_id":     self.user_id,
                "session_id":  session_id,
                "repo_id":     repo_id,
                "module_path": "",
                "file_path":   "vieux.ts",
                "kind":        "write",
                "occurred_at": occurred_at,
                "tool_use_id": Uuid::new_v4().to_string(),
            }]),
        )
        .await;
    }

    /// Appel d'une fonction de la base avec la cle de service.
    async fn appeler(&self, nom: &str, corps: Value) -> Vec<Value> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/{nom}", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .json(&corps)
            .send()
            .await
            .expect("appel de fonction");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "{nom} a echoue ({code}) : {texte}");

        serde_json::from_str::<Value>(&texte)
            .expect("reponse JSON de fonction")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Insertion de service qui refuse d'echouer en silence.
    async fn ecrire_service(&self, chemin: &str, corps: Value) -> Value {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/{}", self.url, chemin))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .header("Prefer", "return=representation")
            .json(&corps)
            .send()
            .await
            .expect("requete de service");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "POST {chemin} a echoue ({code}) : {texte}");

        serde_json::from_str(&texte).expect("reponse JSON d'insertion")
    }

    /// Relit un repo en contournant la RLS.
    pub async fn lire_repo(&self, repo_id: &str) -> Value {
        self.lire(&format!("repos?id=eq.{repo_id}&select=*")).await[0].clone()
    }

    /// Relit les modules d'un repo en contournant la RLS.
    pub async fn lire_modules(&self, repo_id: &str) -> Vec<Value> {
        self.lire(&format!("modules?repo_id=eq.{repo_id}&select=*"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    async fn lire(&self, chemin: &str) -> Value {
        self.http
            .get(format!("{}/rest/v1/{}", self.url, chemin))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .send()
            .await
            .expect("lecture de service")
            .json()
            .await
            .expect("reponse JSON de lecture")
    }

    /// Ecriture de service qui refuse d'echouer en silence.
    ///
    /// Une aide de test muette ne rate pas seulement son travail : elle fait
    /// passer au vert des tests qui devraient etre rouges.
    async fn ecrire(&self, chemin: &str, corps: Value) {
        let reponse = self
            .http
            .patch(format!("{}/rest/v1/{}", self.url, chemin))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .header("Prefer", "return=representation")
            .json(&corps)
            .send()
            .await
            .expect("requete de service");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();

        assert!(code.is_success(), "PATCH {chemin} a echoue ({code}) : {texte}");
        assert!(
            texte.trim_start().starts_with('[') && texte.trim() != "[]",
            "PATCH {chemin} n'a touche aucune ligne : {texte}"
        );
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
