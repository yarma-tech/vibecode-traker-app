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

        self.poser_modules(&repo_id, modules).await;
        repo_id
    }

    /// Pose un repo par son identite, comme le ferait un scan (issue #28).
    pub async fn creer_repo_avec_identite(
        &self,
        machine_id: &str,
        identity: &str,
        modules: &[&str],
    ) -> String {
        let repos: Value = self
            .ecrire_service(
                "repos",
                json!([{
                    "user_id":    self.user_id,
                    "machine_id": machine_id,
                    "name":       "atelier",
                    "root_hash":  Uuid::new_v4().to_string(),
                    "identity":   identity,
                }]),
            )
            .await;

        let repo_id = repos[0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("pas d'id de repo dans {repos}"))
            .to_string();

        self.poser_modules(&repo_id, modules).await;
        repo_id
    }

    /// Pose les modules d'un repo deja cree, sans passer par un scan de disque.
    async fn poser_modules(&self, repo_id: &str, modules: &[&str]) {
        if modules.is_empty() {
            return;
        }

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

    /// L'apercu de l'accueil : tous les repos de l'appelant, tries par activite.
    ///
    /// Appele avec le jeton de l'utilisateur, comme le fait la page web : la RLS
    /// filtre alors la liste aux seuls repos de cet utilisateur, sans quoi la
    /// cle de service verrait ceux de toute la base de test.
    pub async fn apercu_repos(&self, fenetre_secondes: i64) -> Vec<Value> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/apercu_repos", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(&self.user_token)
            .json(&json!({ "p_fenetre_secondes": fenetre_secondes }))
            .send()
            .await
            .expect("appel de apercu_repos");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "apercu_repos a echoue ({code}) : {texte}");

        serde_json::from_str::<Value>(&texte)
            .expect("reponse JSON de apercu_repos")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Pose le proprietaire du remote d'un repo, comme le ferait un scan.
    pub async fn poser_remote_owner(&self, repo_id: &str, owner: &str) {
        self.ecrire(
            &format!("repos?id=eq.{repo_id}"),
            json!({ "remote_owner": owner }),
        )
        .await;
    }

    /// Regle une fois la correspondance proprietaire -> compte, comme le ferait
    /// l'ecran des reglages.
    pub async fn regler_compte(&self, owner: &str, label: &str) {
        self.ecrire_service(
            "account_mappings",
            json!([{ "user_id": self.user_id, "owner": owner, "label": label }]),
        )
        .await;
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

    /// Dit si la purge des evenements est bien programmee cote base (pg_cron).
    pub async fn purge_planifiee(&self) -> bool {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/purge_planifiee", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .json(&json!({}))
            .send()
            .await
            .expect("appel de purge_planifiee");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        assert!(code.is_success(), "purge_planifiee a echoue ({code}) : {texte}");
        texte.trim() == "true"
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

    /// Appel generique d'une fonction RPC avec un jeton choisi par
    /// l'appelant, tolerant a l'echec (rend `Err` plutot que de paniquer) :
    /// sert a eprouver un appel direct a une fonction de conversion PRD
    /// (#37) hors du chemin normal du daemon - par exemple pour forcer une
    /// erreur a mi-chemin et verifier que rien n'est reste a moitie ecrit.
    pub async fn appeler_rpc_avec_jeton(
        &self,
        nom: &str,
        jeton: &str,
        corps: Value,
    ) -> Result<Value, (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/{nom}", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .json(&corps)
            .send()
            .await
            .expect("appel de fonction RPC");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(serde_json::from_str(&texte).unwrap_or(Value::Null))
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

    /// Cree un bloc comme le fera la page web : par la fonction RPC, avec le
    /// jeton de l'utilisateur du contexte (jamais celui d'une machine).
    pub async fn creer_bloc(&self, repo_id: &str, titre: &str, type_: &str, chemin: &str) -> Value {
        self.creer_bloc_avec_jeton(&self.user_token, repo_id, titre, type_, chemin)
            .await
            .unwrap_or_else(|(code, texte)| panic!("creer_bloc a echoue ({code}) : {texte}"))
    }

    /// Meme appel, avec un jeton choisi par l'appelant : sert a eprouver la
    /// RLS avec le jeton d'un AUTRE compte, sans faire paniquer le test au
    /// premier refus attendu.
    pub async fn creer_bloc_avec_jeton(
        &self,
        jeton: &str,
        repo_id: &str,
        titre: &str,
        type_: &str,
        chemin: &str,
    ) -> Result<Value, (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/creer_bloc", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .json(&json!({
                "p_repo_id": repo_id,
                "p_titre": titre,
                "p_type": type_,
                "p_chemin": chemin,
            }))
            .send()
            .await
            .expect("appel de creer_bloc");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(serde_json::from_str(&texte).expect("reponse JSON de creer_bloc"))
    }

    /// Tente une insertion brute dans `blocs`, avec le jeton d'un utilisateur -
    /// jamais par `creer_bloc()`. Sert a eprouver #33 : la policy `insert`
    /// doit refuser `statut = 'done'` des la creation, pas seulement au fil
    /// des `update` qui suivent - une ligne ne doit jamais pouvoir NAITRE
    /// deja terminee.
    pub async fn inserer_bloc_brut_avec_jeton(
        &self,
        jeton: &str,
        corps: Value,
    ) -> Result<Value, (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/blocs", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .header("Prefer", "return=representation")
            .json(&corps)
            .send()
            .await
            .expect("tentative d'insertion brute de bloc");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(serde_json::from_str(&texte).expect("reponse JSON d'insertion brute"))
    }

    /// Le pendant de `inserer_bloc_brut_avec_jeton` pour `issues`.
    pub async fn inserer_issue_brute_avec_jeton(
        &self,
        jeton: &str,
        corps: Value,
    ) -> Result<Value, (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/issues", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .header("Prefer", "return=representation")
            .json(&corps)
            .send()
            .await
            .expect("tentative d'insertion brute d'issue");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(serde_json::from_str(&texte).expect("reponse JSON d'insertion brute"))
    }

    /// Supprime un bloc avec le jeton d'un utilisateur, exactement comme le
    /// fera le bouton de suppression du tableau (#38, FR-048) - jamais avec
    /// la cle de service, qui ne prouverait rien de la RLS traversee par ce
    /// geste web.
    pub async fn supprimer_bloc_avec_jeton(
        &self,
        jeton: &str,
        bloc_id: &str,
    ) -> Result<(), (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .delete(format!("{}/rest/v1/blocs?id=eq.{}", self.url, bloc_id))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .header("Prefer", "return=representation")
            .send()
            .await
            .expect("tentative de suppression de bloc");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        // Comme les PATCH ci-dessus : la RLS ne rend jamais d'erreur pour une
        // ligne qu'elle rend simplement invisible - `return=representation`
        // vide est la seule facon de distinguer « supprime » de « refuse en
        // silence, rien ne correspondait au filtre RLS ».
        if texte.trim() == "[]" {
            return Err((code, "la RLS n'a touche aucune ligne".to_string()));
        }
        Ok(())
    }

    /// Supprime un bloc en contournant la RLS, comme le fera un jour un
    /// bouton de suppression : sert ici a eprouver la non-reattribution.
    pub async fn supprimer_bloc(&self, bloc_id: &str) {
        let reponse = self
            .http
            .delete(format!("{}/rest/v1/blocs?id=eq.{}", self.url, bloc_id))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .send()
            .await
            .expect("suppression de bloc");

        assert!(
            reponse.status().is_success(),
            "suppression du bloc {bloc_id} a echoue ({})",
            reponse.status()
        );
    }

    /// Relit les blocs d'un repo en contournant la RLS.
    pub async fn lire_blocs(&self, repo_id: &str) -> Vec<Value> {
        self.lire(&format!("blocs?repo_id=eq.{repo_id}&select=*&order=ref"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Relit un seul bloc en contournant la RLS, pour verifier ce que les
    /// triggers (bloc_coherent, etat_bloc) lui ont fait subir.
    pub async fn lire_bloc(&self, bloc_id: &str) -> Value {
        self.lire(&format!("blocs?id=eq.{bloc_id}&select=*")).await[0].clone()
    }

    /// Relit une seule issue en contournant la RLS.
    pub async fn lire_issue(&self, issue_id: &str) -> Value {
        self.lire(&format!("issues?id=eq.{issue_id}&select=*")).await[0].clone()
    }

    /// Relit les commits d'un repo en contournant la RLS.
    pub async fn lire_commits(&self, repo_id: &str) -> Vec<Value> {
        self.lire(&format!("commits?repo_id=eq.{repo_id}&select=*&order=authored_at"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Relit les fermetures d'un bloc en contournant la RLS, dans l'ordre ou
    /// elles ont ete posees : c'est l'historique des versions d'un travail
    /// (FR-024).
    pub async fn lire_fermetures_bloc(&self, bloc_id: &str) -> Vec<Value> {
        self.lire(&format!("fermetures?bloc_id=eq.{bloc_id}&select=*&order=ferme_le"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Relit les fermetures d'une issue en contournant la RLS.
    pub async fn lire_fermetures_issue(&self, issue_id: &str) -> Vec<Value> {
        self.lire(&format!("fermetures?issue_id=eq.{issue_id}&select=*&order=ferme_le"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Pose une ligne `commits` directement, en contournant la RLS et le
    /// chemin normal (`ingerer_commit`). Sert a isoler `fermer_par_reference`
    /// de l'insertion qui la declenche d'ordinaire, pour eprouver la fonction
    /// seule - y compris contre un appelant qui n'a aucun droit sur ce repo.
    pub async fn poser_commit_brut(&self, repo_id: &str, sha: &str, message: &str) -> String {
        let lignes = self
            .ecrire_service(
                "commits",
                json!([{
                    "repo_id":     repo_id,
                    "sha":         sha,
                    "message":     message,
                    "authored_at": chrono::Utc::now(),
                }]),
            )
            .await;

        lignes[0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("pas d'id de commit dans {lignes}"))
            .to_string()
    }

    /// Cree une issue comme le fera la page web : par la fonction RPC, avec le
    /// jeton de l'utilisateur du contexte. `chemin: None` simule une saisie
    /// vide - le cas de la premiere issue d'un bloc simple, qui n'a pas besoin
    /// d'en fournir un puisqu'elle herite celui du bloc (FR-006).
    pub async fn creer_issue(&self, bloc_id: &str, titre: &str, chemin: Option<&str>) -> Value {
        self.creer_issue_avec_jeton(&self.user_token, bloc_id, titre, chemin)
            .await
            .unwrap_or_else(|(code, texte)| panic!("creer_issue a echoue ({code}) : {texte}"))
    }

    /// Meme appel, avec un jeton choisi par l'appelant : sert a eprouver la
    /// RLS avec le jeton d'un AUTRE compte, sans faire paniquer le test au
    /// premier refus attendu.
    pub async fn creer_issue_avec_jeton(
        &self,
        jeton: &str,
        bloc_id: &str,
        titre: &str,
        chemin: Option<&str>,
    ) -> Result<Value, (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/rpc/creer_issue", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .json(&json!({
                "p_bloc_id": bloc_id,
                "p_titre": titre,
                "p_chemin": chemin,
            }))
            .send()
            .await
            .expect("appel de creer_issue");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(serde_json::from_str(&texte).expect("reponse JSON de creer_issue"))
    }

    /// Tente une insertion directe dans `issues`, en contournant la RLS et le
    /// chemin normal de creation (`creer_issue`) : sert uniquement a
    /// eprouver que le schema lui-meme interdit une sous-issue (FR-008),
    /// independamment de la verification que fait deja la fonction RPC.
    pub async fn tenter_inserer_issue_brute(
        &self,
        repo_id: &str,
        bloc_id: &str,
        ref_: i64,
        chemin: &str,
    ) -> Result<Value, (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .post(format!("{}/rest/v1/issues", self.url))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .header("Prefer", "return=representation")
            .json(&json!([{
                "user_id": self.user_id,
                "repo_id": repo_id,
                "bloc_id": bloc_id,
                "ref": ref_,
                "titre": "detour interdit",
                "chemin": chemin,
            }]))
            .send()
            .await
            .expect("tentative d'insertion brute d'issue");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(serde_json::from_str(&texte).expect("reponse JSON d'insertion brute"))
    }

    /// Relit les issues d'un repo en contournant la RLS.
    pub async fn lire_issues(&self, repo_id: &str) -> Vec<Value> {
        self.lire(&format!("issues?repo_id=eq.{repo_id}&select=*&order=ref"))
            .await
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Lit les issues visibles avec un jeton donne : sert a eprouver que les
    /// issues d'un compte restent invisibles a un autre.
    pub async fn lire_issues_avec_jeton(&self, jeton: &str, repo_id: &str) -> Vec<Value> {
        let reponse = self
            .http
            .get(format!("{}/rest/v1/issues?repo_id=eq.{repo_id}&select=*", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .send()
            .await
            .expect("lecture d'issues");

        reponse
            .json::<Value>()
            .await
            .expect("reponse JSON de lecture d'issues")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Pose directement le statut d'une issue, en contournant la RLS : simule
    /// le signal qu'un futur automatisme (#31, #32) ecrira, sans avoir a
    /// l'implementer pour eprouver la derivation de l'etat d'un bloc (#30).
    ///
    /// Sert aussi, depuis #33, a eprouver que la cle `service_role` reste
    /// capable de poser `done` directement : c'est la meme requete que
    /// prendrait un automatisme cote serveur, jamais le navigateur.
    pub async fn poser_statut_issue(&self, issue_id: &str, statut: &str) {
        self.ecrire(&format!("issues?id=eq.{issue_id}"), json!({ "statut": statut }))
            .await;
    }

    /// Le pendant de `poser_statut_issue` pour un bloc : pose son statut avec
    /// la cle de service, en contournant la RLS.
    pub async fn poser_statut_bloc(&self, bloc_id: &str, statut: &str) {
        self.ecrire(&format!("blocs?id=eq.{bloc_id}"), json!({ "statut": statut }))
            .await;
    }

    /// Tente de poser `prd_priorite` avec la cle de SERVICE - qui echappe a
    /// la RLS et au trigger `prd_champs_proteges` (FR-042, tous deux
    /// contournables par un role `rolbypassrls`). Sert a eprouver que la
    /// contrainte `check` sur la colonne, elle, ne l'est par AUCUN role : la
    /// garantie de forme (`P<chiffres>`) tient par le schema, pas seulement
    /// par le parseur du daemon ni par un trigger applicatif.
    pub async fn tenter_poser_prd_priorite_service(
        &self,
        bloc_id: &str,
        valeur: &str,
    ) -> Result<(), (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .patch(format!("{}/rest/v1/blocs?id=eq.{}", self.url, bloc_id))
            .header("apikey", &self.service_key)
            .bearer_auth(&self.service_key)
            .header("Prefer", "return=representation")
            .json(&json!({ "prd_priorite": valeur }))
            .send()
            .await
            .expect("tentative de pose de prd_priorite avec la cle de service");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        Ok(())
    }

    /// Tente de deplacer un bloc directement, avec le jeton d'un utilisateur -
    /// comme le ferait un geste web s'il existait. Sert a eprouver FR-018 :
    /// un bloc decoupe doit refuser ce PATCH, un bloc simple doit l'accepter.
    pub async fn deplacer_bloc_avec_jeton(
        &self,
        jeton: &str,
        bloc_id: &str,
        statut: &str,
    ) -> Result<(), (reqwest::StatusCode, String)> {
        self.patch_bloc_avec_jeton(jeton, bloc_id, json!({ "statut": statut })).await
    }

    /// Meme geste, avec un corps de requete libre : sert a eprouver qu'un
    /// PATCH qui glisse `statut = 'done'` au milieu d'autres colonnes est
    /// refuse comme n'importe quel autre (#33, FR-026) - la policy ne regarde
    /// que la ligne resultante, jamais la forme de la requete qui l'a produite.
    pub async fn patch_bloc_avec_jeton(
        &self,
        jeton: &str,
        bloc_id: &str,
        corps: Value,
    ) -> Result<(), (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .patch(format!("{}/rest/v1/blocs?id=eq.{}", self.url, bloc_id))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .header("Prefer", "return=representation")
            .json(&corps)
            .send()
            .await
            .expect("requete de modification de bloc");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        if !(texte.trim_start().starts_with('[') && texte.trim() != "[]") {
            return Err((code, texte));
        }
        Ok(())
    }

    /// Le pendant de `deplacer_bloc_avec_jeton` pour une issue (#33, FR-025 et
    /// FR-026) : sert a eprouver la sortie de « Termine » comme le refus d'y
    /// entrer a la main, sur la table `issues` cette fois.
    pub async fn deplacer_issue_avec_jeton(
        &self,
        jeton: &str,
        issue_id: &str,
        statut: &str,
    ) -> Result<(), (reqwest::StatusCode, String)> {
        self.patch_issue_avec_jeton(jeton, issue_id, json!({ "statut": statut })).await
    }

    /// Le pendant de `patch_bloc_avec_jeton` pour une issue.
    pub async fn patch_issue_avec_jeton(
        &self,
        jeton: &str,
        issue_id: &str,
        corps: Value,
    ) -> Result<(), (reqwest::StatusCode, String)> {
        let reponse = self
            .http
            .patch(format!("{}/rest/v1/issues?id=eq.{}", self.url, issue_id))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .header("Prefer", "return=representation")
            .json(&corps)
            .send()
            .await
            .expect("requete de modification d'issue");

        let code = reponse.status();
        let texte = reponse.text().await.unwrap_or_default();
        if !code.is_success() {
            return Err((code, texte));
        }
        if !(texte.trim_start().starts_with('[') && texte.trim() != "[]") {
            return Err((code, texte));
        }
        Ok(())
    }

    /// Lit les blocs visibles avec un jeton donne : sert a eprouver que le
    /// tableau d'un compte reste invisible a un autre.
    pub async fn lire_blocs_avec_jeton(&self, jeton: &str, repo_id: &str) -> Vec<Value> {
        let reponse = self
            .http
            .get(format!("{}/rest/v1/blocs?repo_id=eq.{repo_id}&select=*", self.url))
            .header("apikey", &self.anon_key)
            .bearer_auth(jeton)
            .send()
            .await
            .expect("lecture de blocs");

        reponse
            .json::<Value>()
            .await
            .expect("reponse JSON de lecture de blocs")
            .as_array()
            .cloned()
            .unwrap_or_default()
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

    /// Pose la date du dernier battement d'une machine, sans attendre le daemon.
    ///
    /// Permet d'éprouver la bascule en état gelé : un battement vieux de plus de
    /// 90 s doit faire déclarer la machine injoignable côté écran.
    pub async fn poser_derniere_presence(&self, machine_id: &str, quand: &str) {
        self.ecrire(
            &format!("machines?id=eq.{machine_id}"),
            json!({ "last_seen_at": quand }),
        )
        .await;
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
