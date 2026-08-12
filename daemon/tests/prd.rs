//! Lire un PRD et en tirer un bloc d'exploration (issue #36, premiere moitie
//! de F12 - FR-036 a FR-038, FR-043, FR-044).
//!
//! Deux familles, comme `commits.rs` :
//! - la premiere n'ouvre aucun fichier, ne parle a aucun reseau : elle
//!   eprouve `analyser` comme une fonction pure sur du texte fabrique a la
//!   main - c'est la ou vit la preuve de FR-043 (liste fermee) ;
//! - la seconde monte un vrai depot git temporaire et parle a la vraie pile
//!   Supabase locale (ADR 0001), en passant par `vibemap::prd::traiter`
//!   exactement comme la boucle de cartographie du daemon le fera.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use vibemap::prd::{analyser, nom_logique, traiter};
use vibemap::Plan;

// ---------------------------------------------------------------------------
// Fixtures : des documents PRD fabriques a la main, jamais lus sur disque
// dans cette section.
// ---------------------------------------------------------------------------

/// Un document minimal, en-tete + une section de features libre. Les tests
/// qui n'ont besoin que de l'en-tete passent une chaine vide pour `corps`.
fn document(id: &str, statut: &str, date: &str, repo: &str, titre: Option<&str>, corps: &str) -> String {
    let ligne_titre = titre.map(|t| format!("titre: {t}\n")).unwrap_or_default();
    format!("---\nid: {id}\nstatut: {statut}\ndate: {date}\nrepo: {repo}\n{ligne_titre}---\n\n{corps}\n")
}

/// Une section `### Fn` complete, avec un corps de section libre - c'est la
/// ou vivent user story, exigences et criteres dans un vrai PRD.
fn section_feature(numero: u32, titre: &str, priorite: &str, corps_section: &str) -> String {
    format!("### F{numero} — {titre} (Priorité : {priorite})\n\n{corps_section}\n\n")
}

// ---------------------------------------------------------------------------
// `analyser` : fonction pure, aucun reseau, aucun disque.
// ---------------------------------------------------------------------------

/// FR-036 : un en-tete qui porte les quatre champs requis est reconnu.
#[test]
fn un_entete_conforme_est_reconnu() {
    let texte = document("PRD-042", "draft", "2026-08-12", "atelier", Some("Un cadrage"), "");
    let doc = analyser(&texte).expect("l'en-tete est conforme, il doit etre reconnu");

    assert_eq!(doc.entete.id, "PRD-042");
    assert_eq!(doc.entete.statut, "draft");
    assert_eq!(doc.entete.date, "2026-08-12");
    assert_eq!(doc.entete.repo, "atelier");
    assert_eq!(doc.entete.titre, "Un cadrage");
}

/// FR-036 : un markdown ordinaire (README, ADR...) n'a pas d'en-tete YAML du
/// tout - ignore sans qu'`analyser` n'y voit meme un document a rejeter.
#[test]
fn un_document_sans_entete_est_ignore() {
    let texte = "# Juste un titre\n\nDu texte ordinaire, sans front-matter.\n";
    assert_eq!(analyser(texte), None);
}

/// Un en-tete commence mais le second `---` n'arrive jamais : un fichier a
/// moitie ecrit (sauvegarde en cours, edition interrompue) ne doit pas faire
/// paniquer le parseur, seulement le laisser de cote.
#[test]
fn un_entete_tronque_est_ignore() {
    let texte = "---\nid: PRD-1\nstatut: draft\ndate: 2026-08-12\nrepo: atelier\n\n# Pas de second delimiteur\n";
    assert_eq!(analyser(texte), None);
}

/// FR-036 : il manque `repo` - l'en-tete n'est pas conforme, meme si les
/// trois autres champs sont la.
#[test]
fn un_entete_incomplet_est_ignore() {
    let texte = "---\nid: PRD-1\nstatut: draft\ndate: 2026-08-12\n---\n\ncontenu\n";
    assert_eq!(analyser(texte), None);
}

/// Une date de front-matter qui n'est pas une date ISO rend l'en-tete non
/// conforme : mieux vaut ignorer un document mal ecrit que fabriquer une cle
/// `prd_cle` avec une date qui n'en est pas une.
#[test]
fn une_date_illisible_rend_lentete_non_conforme() {
    let texte = document("PRD-1", "draft", "hier", "atelier", None, "");
    assert_eq!(analyser(&texte), None);
}

/// `titre` n'est pas dans la liste de FR-036 : son absence ne doit pas faire
/// rejeter le document, seulement faire retomber le titre sur `id`.
#[test]
fn le_titre_manquant_retombe_sur_lid() {
    let texte = document("PRD-007", "draft", "2026-08-12", "atelier", None, "");
    let doc = analyser(&texte).expect("l'en-tete reste conforme sans titre");
    assert_eq!(doc.entete.titre, "PRD-007");
}

/// Un statut que le vocabulaire du produit ne connait pas (ni `draft`, ni
/// `validé`, ni `abandonné`) ne fait pas echouer l'analyse : il est lu tel
/// quel, et c'est a l'appelant (`traiter`) de decider qu'il n'en fait rien.
#[test]
fn un_statut_inattendu_est_lu_tel_quel() {
    let texte = document("PRD-1", "en_pause", "2026-08-12", "atelier", None, "");
    let doc = analyser(&texte).expect("l'en-tete reste conforme");
    assert_eq!(doc.entete.statut, "en_pause");
}

/// Les sections `### Fn` sont reconnues, avec leur cle complete
/// `<date>/<id>/Fn` (FR-040), leur titre et leur priorite.
#[test]
fn les_features_sont_reconnues_avec_cle_titre_priorite() {
    let corps = format!(
        "{}{}",
        section_feature(1, "Premiere feature", "P1", "Du texte."),
        section_feature(2, "Seconde feature", "P2", "D'autre texte."),
    );
    let texte = document("PRD-9", "validé", "2026-08-12", "atelier", None, &corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert_eq!(doc.features.len(), 2);
    assert_eq!(doc.features[0].cle, "2026-08-12/PRD-9/F1");
    assert_eq!(doc.features[0].titre, "Premiere feature");
    assert_eq!(doc.features[0].priorite.as_deref(), Some("P1"));
    assert_eq!(doc.features[1].cle, "2026-08-12/PRD-9/F2");
    assert_eq!(doc.features[1].priorite.as_deref(), Some("P2"));
}

/// FR-042 (releve en relecture de #37) : le PRD ecrit `P0` a `P2`, le vrai
/// PRD-001 va jusqu'a `P3` - `P` suivi d'un ou plusieurs chiffres est la
/// forme retenue. `P10` (deux chiffres) le prouve : la borne n'est pas
/// "un seul chiffre", elle est "des chiffres, rien d'autre".
#[test]
fn les_priorites_p0_a_p3_et_au_dela_sont_toutes_reconnues() {
    let corps = format!(
        "{}{}{}{}{}",
        section_feature(1, "F0", "P0", ""),
        section_feature(2, "F1", "P1", ""),
        section_feature(3, "F2", "P2", ""),
        section_feature(4, "F3", "P3", ""),
        section_feature(5, "F10", "P10", ""),
    );
    let texte = document("PRD-9", "validé", "2026-08-12", "atelier", None, &corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert_eq!(doc.features[0].priorite.as_deref(), Some("P0"));
    assert_eq!(doc.features[1].priorite.as_deref(), Some("P1"));
    assert_eq!(doc.features[2].priorite.as_deref(), Some("P2"));
    assert_eq!(doc.features[3].priorite.as_deref(), Some("P3"));
    assert_eq!(doc.features[4].priorite.as_deref(), Some("P10"));
}

/// FR-042, le canal de texte libre trouve en relecture de #37 : sans borne
/// de forme, tout ce qui suit `(Priorité :` voyageait tel quel, quelle que
/// soit sa longueur - exactement le passage que CONTRIBUTING.md interdit
/// d'ouvrir. Ce qui ne correspond pas a `P<chiffres>` est ECARTE, jamais
/// tronque ni devine (PRD, hypotheses : "on corrige le document, on ne
/// devine pas") - la feature reste creee, seule sa priorite retombe a
/// `None`.
#[test]
fn une_priorite_hors_de_la_forme_pn_nest_pas_transmise() {
    let corps = section_feature(1, "F", "Extremement important, a faire avant tout le reste", "");
    let texte = document("PRD-9", "validé", "2026-08-12", "atelier", None, &corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert_eq!(doc.features.len(), 1, "la feature reste creee");
    assert_eq!(doc.features[0].priorite, None, "mais sa priorite hors-forme n'est pas gardee");
}

/// Meme regle, cas limite : une priorite vide (`(Priorité : )`) n'est pas
/// plus transmise qu'une priorite bavarde - `P` suivi de rien n'est pas
/// `P<chiffres>` non plus.
#[test]
fn une_priorite_vide_reste_a_none() {
    let corps = "### F1 — Titre (Priorité : )\n\n";
    let texte = document("PRD-9", "validé", "2026-08-12", "atelier", None, corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert_eq!(doc.features[0].priorite, None);
}

/// Le marqueur `[À CLARIFIER]` dans le corps d'UNE section n'affecte que
/// cette section-la, pas ses voisines.
#[test]
fn une_feature_marquee_a_clarifier_est_distinguee_de_ses_voisines() {
    let corps = format!(
        "{}{}",
        section_feature(1, "Claire", "P1", "Rien a signaler."),
        section_feature(2, "A eclaircir", "P2", "- **[À CLARIFIER]** : on ne sait pas encore."),
    );
    let texte = document("PRD-9", "draft", "2026-08-12", "atelier", None, &corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert!(!doc.features[0].a_clarifier, "la premiere section ne porte pas le marqueur");
    assert!(doc.features[1].a_clarifier, "la seconde le porte");
}

/// FR-044 : un corps sans la moindre section `### Fn` ne fait reconnaitre
/// aucune feature - c'est ce cas precis que `traiter` doit signaler.
#[test]
fn aucune_feature_reconnue_quand_le_corps_nen_contient_pas() {
    let texte = document(
        "PRD-9",
        "validé",
        "2026-08-12",
        "atelier",
        None,
        "## Contexte\n\nCe document n'a jamais ete rempli au-dela de son en-tete.\n",
    );
    let doc = analyser(&texte).expect("en-tete conforme");
    assert!(doc.features.is_empty());
}

/// FR-043, la preuve : une section qui contient une vraie user story, de
/// vraies exigences numerotees et de vrais criteres d'acceptation ne laisse
/// AUCUNE trace de ce texte dans la feature qu'on en tire. Le type `Feature`
/// n'a nulle part ou le loger ; ce test le verifie a l'execution plutot que
/// de se fier a cette seule garantie de typage.
#[test]
fn la_feature_extraite_ne_transporte_ni_user_story_ni_exigences_ni_criteres() {
    let corps_section = "\
- **User story** : En tant que developpeur, je veux un secret bien cache afin de le retrouver ici si le parseur fuit.\n\
- **Exigences** :\n\
  - **FR-999** : Le systeme NE DOIT JAMAIS transmettre ce texte-la.\n\
- **Critères d'acceptation** :\n\
  - [ ] Etant donne ce texte, quand on analyse le document, alors il n'apparait nulle part ailleurs.\n\
- **Hors scope** : tout le reste.\n";
    let corps = section_feature(1, "Feature sensible", "P1", corps_section);
    let texte = document("PRD-9", "draft", "2026-08-12", "atelier", None, &corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert_eq!(doc.features.len(), 1);
    let feature = &doc.features[0];

    // Les seuls quatre champs que `Feature` peut porter, verifies un par un -
    // et le format Debug de l'ensemble, pour couvrir aussi le futur si le
    // type gagnait un champ sans que ce test soit mis a jour.
    for interdit in [
        "User story",
        "developpeur",
        "Exigences",
        "FR-999",
        "Critères d'acceptation",
        "Hors scope",
    ] {
        assert!(!feature.cle.contains(interdit), "{interdit} ne doit pas fuiter dans la cle");
        assert!(!feature.titre.contains(interdit), "{interdit} ne doit pas fuiter dans le titre");
        assert!(
            !feature.priorite.as_deref().unwrap_or_default().contains(interdit),
            "{interdit} ne doit pas fuiter dans la priorite"
        );
    }
    let debug = format!("{doc:?}");
    for interdit in ["User story", "Exigences", "FR-999", "Critères d'acceptation"] {
        assert!(
            !debug.contains(interdit),
            "{interdit} ne doit apparaitre nulle part dans le document analyse, meme en Debug"
        );
    }
}

/// Le titre d'une feature peut etre separe de sa priorite par le tiret
/// cadratin des PRD existants ou par un simple tiret : les deux sont lus.
#[test]
fn le_titre_cadratin_et_le_tiret_simple_sont_tous_deux_acceptes() {
    let corps = "### F1 — Avec cadratin (Priorité : P1)\n\n### F2 - Avec tiret simple (Priorité : P2)\n\n";
    let texte = document("PRD-9", "draft", "2026-08-12", "atelier", None, corps);
    let doc = analyser(&texte).expect("en-tete conforme");

    assert_eq!(doc.features[0].titre, "Avec cadratin");
    assert_eq!(doc.features[1].titre, "Avec tiret simple");
}

/// Un titre qui contient lui-meme un `:` (comme dans un vrai frontmatter
/// YAML, `titre: Kanban: le retour`) doit etre lu en entier, pas coupe au
/// premier `:` rencontre.
#[test]
fn un_titre_avec_deux_points_est_lu_en_entier() {
    let texte = document("PRD-1", "draft", "2026-08-12", "atelier", Some("Kanban: le retour"), "");
    let doc = analyser(&texte).expect("en-tete conforme");
    assert_eq!(doc.entete.titre, "Kanban: le retour");
}

/// Un fichier de plusieurs megaoctets (un vrai risque : un PRD colle depuis
/// un export, avec toute une annexe) doit s'analyser sans probleme - aucune
/// recursion, aucun regex a retour arriere catastrophique dans ce parseur.
#[test]
fn un_tres_gros_document_sanalyse_sans_probleme() {
    let paragraphe = "Du texte de remplissage qui ne ressemble a aucun en-tete de feature.\n".repeat(20_000);
    let corps = format!("{paragraphe}{}", section_feature(1, "La seule vraie feature", "P1", "Corps."));
    let texte = document("PRD-1", "draft", "2026-08-12", "atelier", None, &corps);

    assert!(texte.len() > 1_000_000, "le document doit vraiment etre gros pour ce test");

    let doc = analyser(&texte).expect("en-tete conforme");
    assert_eq!(doc.features.len(), 1);
    assert_eq!(doc.features[0].titre, "La seule vraie feature");
}

/// `nom_logique` : un depot avec distant se reconnait par le dernier segment
/// de son identite normalisee, pas par le nom (renommable) de son dossier.
#[test]
fn nom_logique_avec_distant_prend_le_dernier_segment_de_lidentite() {
    let plan = plan_simple("dossier-renomme", "github.com/yarma-tech/vibecode-traker-app");
    assert_eq!(nom_logique(&plan), "vibecode-traker-app");
}

/// Sans distant, l'identite est `local:<empreinte>` : le seul nom qu'un tel
/// depot possede est celui de son dossier.
#[test]
fn nom_logique_sans_distant_replie_sur_le_nom_du_dossier() {
    let plan = plan_simple("atelier", "local:abc123");
    assert_eq!(nom_logique(&plan), "atelier");
}

fn plan_simple(nom: &str, identity: &str) -> Plan {
    Plan {
        name: nom.to_string(),
        root_hash: String::new(),
        identity: identity.to_string(),
        remote_owner: None,
        remote_url: None,
        current_branch: None,
        loc_total: 0,
        file_count: 0,
        modules: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// `traiter` : un vrai depot git temporaire, la vraie pile Supabase locale.
// ---------------------------------------------------------------------------

/// Un depot git jetable, sans meme un premier commit : `git ls-files
/// --others --exclude-standard` voit les fichiers non suivis tant qu'ils ne
/// sont pas ignores, un `git init` suffit donc ici.
fn depot_git_temporaire(nom: &str) -> PathBuf {
    let racine = std::env::temp_dir().join(format!("vibemap-prd-{nom}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&racine).unwrap();
    Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .current_dir(&racine)
        .status()
        .expect("git init");
    racine
}

fn ecrire(racine: &Path, chemin_relatif: &str, contenu: &str) {
    let chemin = racine.join(chemin_relatif);
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(chemin, contenu).unwrap();
}

/// Appaire une machine et rend son identite, comme le ferait le daemon
/// (meme helper que `commits.rs`).
async fn machine_reliee(ctx: &common::TestContext) -> vibemap::Identite {
    let code = ctx.creer_code().await;
    vibemap::appairer(&ctx.url, &ctx.anon_key, &code, "MacBook Pro", Some("darwin"))
        .await
        .expect("appairage")
}

/// Un repo Supabase et le client daemon qui parle avec le jeton de sa
/// machine - exactement comme `traiter` sera appele en vrai.
async fn repo_de_test(ctx: &common::TestContext) -> (vibemap::Supabase, String) {
    let machine = machine_reliee(ctx).await;
    let repo_id = ctx.creer_repo(&machine.machine_id, &[]).await;
    (vibemap::Supabase::new(&ctx.url, &machine.token), repo_id)
}

/// Le critere d'acceptation central de l'issue : un PRD `draft` de douze
/// features laisse un bloc d'exploration et zero feature.
#[tokio::test]
async fn un_prd_draft_de_douze_features_pose_un_bloc_exploration_et_zero_feature() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("douze-features");
    let plan = plan_simple("atelier", "local:x");

    let corps: String = (1..=12)
        .map(|n| section_feature(n, &format!("Feature {n}"), "P1", "Un peu de texte."))
        .collect();
    let texte = document("PRD-100", "draft", "2026-08-12", "atelier", Some("Grand chantier"), &corps);
    ecrire(&racine, "docs/prd/PRD-100-grand-chantier.md", &texte);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 1);
    assert!(resume.sans_feature_reconnue.is_empty());

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "un seul bloc, quel que soit le nombre de features du document");
    assert_eq!(blocs[0]["type"], serde_json::json!("exploration"));
    assert_eq!(blocs[0]["titre"], serde_json::json!("cadrage Grand chantier"));
    assert_eq!(blocs[0]["chemin"], serde_json::json!("docs/prd/PRD-100-grand-chantier.md"));
    assert_eq!(blocs[0]["prd_cle"], serde_json::json!("2026-08-12/PRD-100"));
    assert_eq!(blocs[0]["prd_statut"], serde_json::json!("draft"));
    assert_eq!(blocs[0]["statut"], serde_json::json!("todo"));
    assert!(blocs[0]["ref"].as_i64().is_some(), "le bloc d'exploration porte une reference VM-n");
}

/// Idempotence : relire le meme document ne cree rien de neuf (l'unique
/// `(repo_id, prd_cle)` en est la garantie cote base, ce test l'eprouve
/// depuis le daemon).
#[tokio::test]
async fn relire_le_meme_document_ne_cree_rien_de_neuf() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("idempotence");
    let plan = plan_simple("atelier", "local:x");

    let texte = document(
        "PRD-1",
        "draft",
        "2026-08-12",
        "atelier",
        Some("Un cadrage"),
        &section_feature(1, "Une feature", "P1", "Texte."),
    );
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);

    let premiere = traiter(&client, &racine, &repo_id, &plan).await;
    let seconde = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(premiere.blocs_poses, 1);
    assert_eq!(seconde.blocs_poses, 1, "le second passage retrouve le meme bloc, il n'en cree pas un autre");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "toujours un seul bloc apres deux lectures du meme document");
}

/// FR-036 : un markdown sans en-tete conforme ne peuple rien.
#[tokio::test]
async fn un_document_sans_entete_conforme_ne_peuple_rien() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("sans-entete");
    let plan = plan_simple("atelier", "local:x");

    ecrire(&racine, "NOTES.md", "# Des notes de reunion\n\nRien a voir avec un PRD.\n");

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 0);
    assert!(ctx.lire_blocs(&repo_id).await.is_empty());
}

/// FR-037 : un document ecrit pour un autre depot ne peuple pas le tableau
/// courant, meme s'il est physiquement present dans ce depot-ci (un
/// monorepo qui referencerait un PRD d'un autre projet, par exemple).
#[tokio::test]
async fn un_repo_different_ne_peuple_pas_le_tableau_courant() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("autre-depot");
    let plan = plan_simple("atelier", "local:x");

    let texte = document("PRD-1", "draft", "2026-08-12", "un-autre-projet", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 0);
    assert!(ctx.lire_blocs(&repo_id).await.is_empty());
}

/// FR-038, cote negatif, MIS A JOUR par #37 : un PRD `validé` ne laisse plus
/// zero bloc - c'est desormais tout le sujet de cette issue. Le test d'origine
/// de #36 affirmait le contraire par construction (la conversion n'existait
/// pas encore) ; il est remplace ici plutot que laisse a mentir sur le
/// comportement actuel.
///
/// Cas le plus simple : un document `validé` des sa toute premiere lecture,
/// sans phase `draft` prealable (aucune exploration n'a donc jamais existe -
/// conception §6 le permet, l'exploration n'est qu'une PHASE, pas un passage
/// oblige).
#[tokio::test]
async fn un_prd_valide_des_sa_premiere_lecture_cree_ses_features_sans_exploration_prealable() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("valide-direct");
    let plan = plan_simple("atelier", "local:x");

    let texte = document(
        "PRD-1",
        "validé",
        "2026-08-12",
        "atelier",
        None,
        &section_feature(1, "Une feature", "P1", ""),
    );
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 0, "aucune exploration n'a jamais existe pour ce document");
    assert_eq!(resume.features_creees, 1);

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1);
    assert_eq!(blocs[0]["type"], serde_json::json!("feature"));
    assert_eq!(blocs[0]["statut"], serde_json::json!("todo"), "une feature arrive toujours en A faire");
    assert_eq!(blocs[0]["prd_cle"], serde_json::json!("2026-08-12/PRD-1/F1"));
    assert_eq!(blocs[0]["prd_priorite"], serde_json::json!("P1"));
    assert_eq!(blocs[0]["prd_absent"], serde_json::json!(false));
    assert!(blocs[0]["ref"].as_i64().is_some(), "une feature porte une reference VM-n comme tout travail suivi");
}

/// Le critere d'acceptation central de l'issue #37 : un PRD `draft` de douze
/// features passe en `validé` fait apparaitre douze features en « A faire »
/// et fait disparaitre le bloc d'exploration - dans cet ordre, comme le vivra
/// vraiment le daemon (300 s de cadence, deux passages successifs).
#[tokio::test]
async fn un_prd_draft_de_douze_features_passe_en_valide_peuple_douze_features_et_retire_lexploration() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("conversion-douze");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-100-grand-chantier.md";

    let corps: String = (1..=12)
        .map(|n| section_feature(n, &format!("Feature {n}"), "P1", "Un peu de texte."))
        .collect();

    // Premier passage : brouillon, l'exploration seule (comme #36 le prouve
    // deja) - on part d'un etat reel plutot que de fabriquer l'exploration a
    // la main, pour eprouver le VRAI enchainement des deux lectures.
    let brouillon = document("PRD-100", "draft", "2026-08-12", "atelier", Some("Grand chantier"), &corps);
    ecrire(&racine, chemin_fichier, &brouillon);
    let premier = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(premier.blocs_poses, 1);
    assert_eq!(ctx.lire_blocs(&repo_id).await.len(), 1);

    // Second passage : le meme document, valide.
    let valide = document("PRD-100", "validé", "2026-08-12", "atelier", Some("Grand chantier"), &corps);
    ecrire(&racine, chemin_fichier, &valide);
    let second = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(second.features_creees, 12);
    assert_eq!(second.blocs_poses, 0, "aucune nouvelle exploration ne se pose sur un document valide");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 12, "douze features, plus aucun bloc d'exploration");
    assert!(
        blocs.iter().all(|b| b["type"] == serde_json::json!("feature")),
        "le bloc d'exploration vierge a bien ete retire"
    );
    assert!(blocs.iter().all(|b| b["statut"] == serde_json::json!("todo")));
    assert!(blocs.iter().all(|b| b["prd_priorite"] == serde_json::json!("P1")));
}

/// Idempotence, cote `validé` cette fois (le cas `draft` est deja eprouve
/// plus haut) : relire le meme document sans changement ne cree rien de neuf.
#[tokio::test]
async fn relire_le_meme_prd_valide_sans_changement_ne_cree_rien_de_neuf() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("idempotence-valide");
    let plan = plan_simple("atelier", "local:x");

    let corps = format!(
        "{}{}",
        section_feature(1, "Premiere", "P1", "Texte."),
        section_feature(2, "Seconde", "P2", "Texte."),
    );
    let texte = document("PRD-1", "validé", "2026-08-12", "atelier", None, &corps);
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);

    let premiere = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(premiere.features_creees, 2);

    let blocs_apres_premiere = ctx.lire_blocs(&repo_id).await;
    let ids_avant: std::collections::BTreeSet<_> =
        blocs_apres_premiere.iter().map(|b| b["id"].as_str().unwrap().to_string()).collect();

    let seconde = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(seconde.features_creees, 0, "les deux features existent deja, rien de neuf a creer");
    assert_eq!(seconde.features_absentes, 0, "rien n'a disparu du document entre les deux lectures");

    let blocs_apres_seconde = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_apres_seconde.len(), 2, "toujours deux blocs, jamais quatre");
    let ids_apres: std::collections::BTreeSet<_> =
        blocs_apres_seconde.iter().map(|b| b["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(ids_avant, ids_apres, "ce sont exactement les memes lignes, pas des doublons");
}

/// FR-039, la moitie qui protege du travail humain : un bloc d'exploration
/// qui porte une issue ajoutee a la main n'est PAS vierge - il est conserve
/// et marque converti, jamais supprime, meme quand le PRD passe `validé`.
#[tokio::test]
async fn un_bloc_dexploration_portant_une_issue_ajoutee_a_la_main_est_conserve_et_marque_converti() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("exploration-non-vierge");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-7.md";

    let brouillon = document(
        "PRD-7",
        "draft",
        "2026-08-12",
        "atelier",
        Some("Chantier suivi de pres"),
        &section_feature(1, "Une feature", "P1", ""),
    );
    ecrire(&racine, chemin_fichier, &brouillon);
    traiter(&client, &racine, &repo_id, &plan).await;

    let blocs_apres_brouillon = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_apres_brouillon.len(), 1);
    let id_exploration = blocs_apres_brouillon[0]["id"].as_str().unwrap().to_string();

    // Une issue ajoutee a la main : l'exploration n'est plus vierge.
    ctx.creer_issue(&id_exploration, "Un detail note pendant le cadrage", None).await;

    let valide = document(
        "PRD-7",
        "validé",
        "2026-08-12",
        "atelier",
        Some("Chantier suivi de pres"),
        &section_feature(1, "Une feature", "P1", ""),
    );
    ecrire(&racine, chemin_fichier, &valide);
    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_creees, 1);

    let exploration = ctx.lire_bloc(&id_exploration).await;
    assert_eq!(exploration["type"], serde_json::json!("exploration"), "elle n'est jamais retypee de force");
    assert_eq!(exploration["prd_converti"], serde_json::json!(true));

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 2, "l'exploration conservee, plus la feature creee - jamais fusionnees ni supprimees");
}

/// FR-040, la preuve : un titre qui change SANS que la cle change (meme
/// date, meme id, meme numero de section) met a jour le meme bloc - jamais
/// un doublon. Le rattachement se fait par la cle, jamais par le titre.
#[tokio::test]
async fn un_titre_de_feature_qui_change_sans_que_la_cle_change_met_a_jour_le_meme_bloc() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("titre-change");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-1.md";

    let v1 = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "Ancien titre", "P1", ""));
    ecrire(&racine, chemin_fichier, &v1);
    traiter(&client, &racine, &repo_id, &plan).await;

    let blocs_v1 = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_v1.len(), 1);
    let id_bloc = blocs_v1[0]["id"].as_str().unwrap().to_string();

    let v2 = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "Nouveau titre", "P1", ""));
    ecrire(&racine, chemin_fichier, &v2);
    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_creees, 0, "la cle n'a pas change, rien de neuf");

    let blocs_v2 = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_v2.len(), 1, "toujours un seul bloc, pas un doublon");
    assert_eq!(blocs_v2[0]["id"], serde_json::json!(id_bloc), "c'est bien le meme bloc");
    assert_eq!(blocs_v2[0]["titre"], serde_json::json!("Nouveau titre"));
}

/// FR-041 : une feature retiree du document est marquee `prd_absent`, jamais
/// supprimee. Les features restantes ne sont pas affectees.
#[tokio::test]
async fn une_feature_retiree_du_document_est_marquee_prd_absent_jamais_supprimee() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("feature-retiree");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-1.md";

    let corps_complet = format!(
        "{}{}",
        section_feature(1, "Reste", "P1", "Texte."),
        section_feature(2, "Disparait", "P2", "Texte."),
    );
    let v1 = document("PRD-1", "validé", "2026-08-12", "atelier", None, &corps_complet);
    ecrire(&racine, chemin_fichier, &v1);
    traiter(&client, &racine, &repo_id, &plan).await;

    let blocs_v1 = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_v1.len(), 2);
    let id_f2 = blocs_v1
        .iter()
        .find(|b| b["prd_cle"] == serde_json::json!("2026-08-12/PRD-1/F2"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // F2 disparait du document, F1 reste.
    let v2 = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "Reste", "P1", "Texte."));
    ecrire(&racine, chemin_fichier, &v2);
    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_absentes, 1);
    assert_eq!(resume.features_creees, 0);

    let blocs_v2 = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_v2.len(), 2, "F2 est CONSERVE, jamais supprime");
    let f2_apres = blocs_v2.iter().find(|b| b["id"] == serde_json::json!(id_f2)).unwrap();
    assert_eq!(f2_apres["prd_absent"], serde_json::json!(true));
    let f1_apres = blocs_v2
        .iter()
        .find(|b| b["prd_cle"] == serde_json::json!("2026-08-12/PRD-1/F1"))
        .unwrap();
    assert_eq!(f1_apres["prd_absent"], serde_json::json!(false), "F1 n'est pas touchee par la disparition de F2");
}

/// FR-042, cote base : la priorite lue dans le document ne se modifie pas
/// depuis le tableau. Ni depuis un vrai bouton (qui n'existe pas dans
/// l'interface, F10/#34 ne retype QUE `type`), ni depuis un PATCH direct qui
/// contournerait l'absence de bouton - la meme demonstration que #33 avait
/// deja faite pour `statut = 'done'` (FR-026).
#[tokio::test]
async fn la_priorite_dune_feature_ne_peut_pas_etre_modifiee_par_un_patch_direct() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("priorite-protegee");
    let plan = plan_simple("atelier", "local:x");

    let texte = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);
    traiter(&client, &racine, &repo_id, &plan).await;

    let bloc = ctx.lire_blocs(&repo_id).await.into_iter().next().unwrap();
    let id_bloc = bloc["id"].as_str().unwrap().to_string();
    assert_eq!(bloc["prd_priorite"], serde_json::json!("P1"));

    let resultat = ctx
        .patch_bloc_avec_jeton(&ctx.user_token, &id_bloc, serde_json::json!({ "prd_priorite": "P4" }))
        .await;
    assert!(resultat.is_err(), "un PATCH direct sur prd_priorite doit etre refuse (FR-042)");

    let bloc_apres = ctx.lire_bloc(&id_bloc).await;
    assert_eq!(bloc_apres["prd_priorite"], serde_json::json!("P1"), "la priorite n'a pas bouge");
}

/// FR-042, le schema plutot que le seul parseur (lecon de #30, releve en
/// relecture de #37) : meme la cle de SERVICE - qui echappe a la RLS et au
/// trigger `prd_champs_proteges` juste au-dessus - ne peut pas poser une
/// `prd_priorite` hors de la forme `P<chiffres>`. Une contrainte `check`
/// protege la colonne pour TOUT chemin d'ecriture, y compris ceux que ce
/// depot n'a pas encore imagines.
#[tokio::test]
async fn la_colonne_prd_priorite_refuse_toute_valeur_hors_de_la_forme_pn_meme_pour_la_cle_de_service() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("priorite-check-schema");
    let plan = plan_simple("atelier", "local:x");

    let texte = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);
    traiter(&client, &racine, &repo_id, &plan).await;

    let bloc = ctx.lire_blocs(&repo_id).await.into_iter().next().unwrap();
    let id_bloc = bloc["id"].as_str().unwrap().to_string();

    let refuse = ctx.tenter_poser_prd_priorite_service(&id_bloc, "Tres urgent").await;
    assert!(refuse.is_err(), "la contrainte check doit refuser une priorite hors forme, meme pour service_role");

    let accepte = ctx.tenter_poser_prd_priorite_service(&id_bloc, "P4").await;
    assert!(accepte.is_ok(), "P4 respecte la forme P<chiffres> meme si le PRD ne va aujourd'hui qu'a P3");

    let vide_ok = ctx.tenter_poser_prd_priorite_service(&id_bloc, "").await;
    // Une chaine vide n'est ni `null` ni de la forme `P<chiffres>` : elle
    // n'a jamais ete produite par le parseur (`nullif` la transforme en
    // `null` avant d'ecrire), mais un appelant direct pourrait la tenter -
    // la contrainte doit la refuser au meme titre que tout texte hors forme.
    assert!(vide_ok.is_err(), "une chaine vide n'est pas P<chiffres> non plus");
}

/// Le pendant positif du test precedent : le nouveau garde-fou de FR-042 ne
/// doit PAS sur-bloquer ce que #33/#34 garantissaient deja sur une carte
/// ordinaire. Une feature issue d'un PRD reste retypable (FR-032, "a tout
/// moment") et sortable de Termine (FR-025) comme n'importe quel autre bloc -
/// aucune de ces deux ecritures ne touche une colonne `prd_*`, le trigger ne
/// doit donc jamais les voir.
#[tokio::test]
async fn une_feature_issue_dun_prd_reste_retypable_et_sortable_de_termine_normalement() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("prd-feature-retypable");
    let plan = plan_simple("atelier", "local:x");

    let texte = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "docs/prd/PRD-1.md", &texte);
    traiter(&client, &racine, &repo_id, &plan).await;

    let bloc = ctx.lire_blocs(&repo_id).await.into_iter().next().unwrap();
    let id_bloc = bloc["id"].as_str().unwrap().to_string();

    ctx.patch_bloc_avec_jeton(&ctx.user_token, &id_bloc, serde_json::json!({ "type": "correction" }))
        .await
        .expect("retyper une feature issue d'un PRD doit rester possible (FR-032)");

    ctx.poser_statut_bloc(&id_bloc, "done").await;
    ctx.deplacer_bloc_avec_jeton(&ctx.user_token, &id_bloc, "doing")
        .await
        .expect("sortir de Termine une feature issue d'un PRD doit rester possible (FR-025)");

    let bloc_apres = ctx.lire_bloc(&id_bloc).await;
    assert_eq!(bloc_apres["type"], serde_json::json!("correction"));
    assert_eq!(bloc_apres["statut"], serde_json::json!("doing"));
    assert_eq!(bloc_apres["prd_priorite"], serde_json::json!("P1"), "inchangee par ces deux ecritures");
}

/// `statut: abandonné` : plus de creation, les blocs deja issus de ce
/// document (exploration si elle n'a jamais ete convertie, chaque feature)
/// passent `prd_absent = true`.
#[tokio::test]
async fn un_prd_abandonne_ne_cree_rien_et_marque_les_blocs_existants_prd_absent() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("abandonne");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-1.md";

    let corps = format!(
        "{}{}",
        section_feature(1, "Une", "P1", "Texte."),
        section_feature(2, "Deux", "P2", "Texte."),
    );
    let valide = document("PRD-1", "validé", "2026-08-12", "atelier", None, &corps);
    ecrire(&racine, chemin_fichier, &valide);
    traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(ctx.lire_blocs(&repo_id).await.len(), 2);

    let abandonne = document("PRD-1", "abandonné", "2026-08-12", "atelier", None, &corps);
    ecrire(&racine, chemin_fichier, &abandonne);
    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_creees, 0, "un document abandonne ne cree jamais rien");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 2, "rien n'est supprime");
    assert!(
        blocs.iter().all(|b| b["prd_absent"] == serde_json::json!(true)),
        "les deux features existantes sont marquees absentes"
    );
}

/// Casse (dirigee par l'issue) : deux documents `validé` partagent le meme
/// `id` et la meme `date` - leurs features naitraient sous les MEMES cles.
/// Contrairement au cas `draft` (ou l'absorption silencieuse est acceptable,
/// un seul bloc de toute facon), laisser le second ecraser ou s'absorber
/// dans le premier ferait mentir un des deux documents en silence : c'est la
/// decision de #37, prise et testee ici. Seul le premier document rencontre
/// (ordre alphabetique stable de `markdowns_du_depot`, verifie par #29-#30
/// -> `a-premier.md` avant `b-second.md`) est converti ; le second est
/// signale a l'ecran plutot qu'absorbe - "on corrige le document, on ne
/// devine pas" (PRD, hypotheses).
#[tokio::test]
async fn deux_prd_valides_avec_le_meme_id_et_la_meme_date_ne_convertissent_que_le_premier() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("id-duplique-valide");
    let plan = plan_simple("atelier", "local:x");

    let premier = document(
        "PRD-1",
        "validé",
        "2026-08-12",
        "atelier",
        Some("Premier document"),
        &section_feature(1, "Titre du premier", "P1", ""),
    );
    let second = document(
        "PRD-1",
        "validé",
        "2026-08-12",
        "atelier",
        Some("Second document"),
        &section_feature(1, "Titre du second", "P2", ""),
    );
    ecrire(&racine, "docs/prd/a-premier.md", &premier);
    ecrire(&racine, "docs/prd/b-second.md", &second);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_creees, 1, "un seul document converti, jamais deux ecritures sur la meme cle");
    assert_eq!(
        resume.cles_dupliquees,
        vec![PathBuf::from("docs/prd/b-second.md")],
        "le second document est signale, jamais absorbe en silence"
    );

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "une seule feature, pas deux qui se marcheraient dessus");
    assert_eq!(
        blocs[0]["titre"],
        serde_json::json!("Titre du premier"),
        "c'est le PREMIER document (ordre alphabetique stable) qui fait foi"
    );
}

/// Atomicite (dirigee par l'issue - "une conversion interrompue a mi-chemin")
/// : un appel direct a `convertir_prd_valide` dont une feature est malformee
/// (titre absent, viole NOT NULL) doit echouer ENTIEREMENT - aucune des
/// features valides qui la precedaient dans le lot ne doit rester ecrite.
/// Une fonction plpgsql s'execute dans la transaction de son appelant, sans
/// COMMIT intermediaire : c'est ce qui garantit ce tout-ou-rien, teste ici au
/// niveau le plus bas, sans passer par le parseur (qui ne produirait jamais
/// une feature sans titre - ce test simule un appelant hostile ou un futur
/// bug d'assemblage du JSON, pas un vrai document).
#[tokio::test]
async fn une_conversion_qui_echoue_a_mi_chemin_ne_laisse_rien() {
    let ctx = common::TestContext::new().await;
    // Un appel direct a la fonction SQL, pas via `traiter` : ce test eprouve
    // l'atomicite de `convertir_prd_valide` elle-meme, independamment du
    // parseur (qui ne produirait jamais une feature sans titre).
    let machine_id = ctx.create_machine("MacBook Pro").await;
    let repo_id = ctx.creer_repo(&machine_id, &[]).await;

    let features = serde_json::json!([
        { "cle": "2026-08-12/PRD-1/F1", "titre": "Celle-ci est valide", "priorite": "P1", "a_clarifier": false },
        { "cle": "2026-08-12/PRD-1/F2", "titre": null, "priorite": "P1", "a_clarifier": false },
    ]);

    let resultat = ctx
        .appeler_rpc_avec_jeton(
            "convertir_prd_valide",
            &ctx.user_token,
            serde_json::json!({
                "p_repo_id": repo_id,
                "p_doc_prd_cle": "2026-08-12/PRD-1",
                "p_prd_statut": "validé",
                "p_prd_maj": null,
                "p_prd_valide_le": null,
                "p_features": features,
            }),
        )
        .await;

    assert!(resultat.is_err(), "un titre manquant doit faire echouer l'appel");
    assert!(
        ctx.lire_blocs(&repo_id).await.is_empty(),
        "la premiere feature, pourtant valide, ne doit pas etre restee ecrite : tout ou rien"
    );
}

/// Casse (suggeree par l'issue) : un PRD `validé` repasse a `draft` (une
/// correction malheureuse, ou un chantier rouvert a la main). L'exploration
/// ne doit pas ressusciter a cote des features deja converties - il n'existe
/// aucun troisieme etat "exploration ET features actives" dans le
/// vocabulaire du produit (PRD, "Les quatre types").
#[tokio::test]
async fn un_prd_qui_repasse_de_valide_a_draft_ne_ressuscite_pas_lexploration() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("regression-valide-draft");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-1.md";

    let valide = document("PRD-1", "validé", "2026-08-12", "atelier", Some("Chantier"), &section_feature(1, "F", "P1", ""));
    ecrire(&racine, chemin_fichier, &valide);
    traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(ctx.lire_blocs(&repo_id).await.len(), 1, "une feature, l'exploration vierge a ete retiree");

    let brouillon = document("PRD-1", "draft", "2026-08-12", "atelier", Some("Chantier"), &section_feature(1, "F", "P1", ""));
    ecrire(&racine, chemin_fichier, &brouillon);
    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.blocs_poses, 0, "aucune exploration ne ressuscite a cote d'une feature deja convertie");
    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "toujours la seule feature, jamais un second bloc d'exploration a cote");
    assert_eq!(blocs[0]["type"], serde_json::json!("feature"));
}

/// Casse (suggeree par l'issue) : un bloc de feature supprime a la main
/// (hors produit - aucun bouton de suppression n'existe encore) reapparait a
/// la prochaine lecture du meme document, avec un nouvel identifiant. Ce
/// n'est pas un defaut : la base de verite reste le document, et le systeme
/// ne DOIT jamais supprimer (FR-041) - il ne peut pas non plus savoir qu'une
/// ligne disparue l'a ete par une main humaine plutot que par un accident
/// d'infrastructure. Ce test documente ce choix plutot que de le laisser
/// implicite.
#[tokio::test]
async fn un_bloc_de_feature_supprime_a_la_main_reapparait_a_la_prochaine_lecture() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("suppression-manuelle");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-1.md";

    let texte = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, chemin_fichier, &texte);
    traiter(&client, &racine, &repo_id, &plan).await;

    let blocs_avant = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_avant.len(), 1);
    let ancien_id = blocs_avant[0]["id"].as_str().unwrap().to_string();
    ctx.supprimer_bloc(&ancien_id).await;
    assert!(ctx.lire_blocs(&repo_id).await.is_empty());

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.features_creees, 1, "le document decrit toujours cette feature, elle est recreee");

    let blocs_apres = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_apres.len(), 1);
    assert_ne!(blocs_apres[0]["id"], serde_json::json!(ancien_id), "un nouvel identifiant, la ligne precedente est bien partie");
}

/// Casse (suggeree par l'issue), documentee plutot que corrigee : corriger
/// la date de l'en-tete change la cle de TOUTES les features du document
/// (`<date>/<id>/Fn`). Les anciens blocs restent actifs, `prd_absent` non
/// pose - la detection de disparition ne regarde que les cles du document
/// COURANT, jamais un id seul, parce que deux documents differents peuvent
/// legitimement partager un `id` a des dates differentes (un gabarit reutilise
/// des mois plus tard). Deviner que la date a ete "corrigee" plutot que le
/// document "change" serait inventer une intention que le fichier ne dit pas
/// (PRD, hypotheses : "on corrige le document, on ne devine pas"). Une vraie
/// correction de date laisse donc un doublon actif a nettoyer a la main -
/// une limite connue, pas un defaut cache.
#[tokio::test]
async fn corriger_la_date_de_len_tete_cree_de_nouveaux_blocs_sans_signaler_les_anciens() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("date-corrigee");
    let plan = plan_simple("atelier", "local:x");
    let chemin_fichier = "docs/prd/PRD-1.md";

    let mauvaise_date = document("PRD-1", "validé", "2026-08-12", "atelier", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, chemin_fichier, &mauvaise_date);
    traiter(&client, &racine, &repo_id, &plan).await;
    let blocs_avant = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_avant.len(), 1);
    let id_ancien_bloc = blocs_avant[0]["id"].as_str().unwrap().to_string();

    let date_corrigee = document("PRD-1", "validé", "2026-08-11", "atelier", None, &section_feature(1, "F", "P1", ""));
    ecrire(&racine, chemin_fichier, &date_corrigee);
    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_creees, 1, "une nouvelle cle, donc un nouveau bloc");
    let blocs_apres = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs_apres.len(), 2, "l'ancien bloc reste actif a cote du nouveau - limite connue");
    let ancien = blocs_apres.iter().find(|b| b["id"] == serde_json::json!(id_ancien_bloc)).unwrap();
    assert_eq!(ancien["prd_absent"], serde_json::json!(false), "il n'est pas signale : sa cle ne fait plus partie du document courant");
}



/// FR-044 : un document dont l'en-tete est reconnu mais qui ne recense
/// aucune feature est signale - `traiter` le rend dans son resume plutot que
/// de le taire.
#[tokio::test]
async fn un_document_sans_feature_reconnue_est_signale_dans_le_resume() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("sans-feature");
    let plan = plan_simple("atelier", "local:x");

    let texte = document("PRD-1", "validé", "2026-08-12", "atelier", None, "## Contexte\n\nRien de plus.\n");
    ecrire(&racine, "docs/prd/PRD-vide.md", &texte);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(
        resume.sans_feature_reconnue,
        vec![PathBuf::from("docs/prd/PRD-vide.md")]
    );
}

/// Casse : un PRD `draft` place a la racine du depot, pas sous `docs/`.
/// FR-036 reconnait un document a son en-tete, jamais a son emplacement -
/// ce test le prouve plutot que de le supposer.
#[tokio::test]
async fn un_prd_hors_de_docs_est_tout_de_meme_lu() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("hors-docs");
    let plan = plan_simple("atelier", "local:x");

    let texte = document("PRD-1", "draft", "2026-08-12", "atelier", Some("A la racine"), &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "BROUILLON.md", &texte);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 1, "l'emplacement du fichier ne conditionne pas sa lecture");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs[0]["chemin"], serde_json::json!("BROUILLON.md"));
}

/// Casse : deux documents `draft` portent le meme `id` et la meme `date`
/// (copier-coller d'un gabarit, par exemple) - leur `prd_cle` collisionne
/// donc volontairement. Le second ne doit ni faire echouer l'ingestion du
/// premier, ni creer un second bloc : il est absorbe par l'idempotence,
/// silencieusement, comme une relecture du meme document.
#[tokio::test]
async fn deux_prd_avec_le_meme_id_et_la_meme_date_ne_creent_quun_seul_bloc() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("id-duplique");
    let plan = plan_simple("atelier", "local:x");

    let premier = document("PRD-1", "draft", "2026-08-12", "atelier", Some("Premier document"), &section_feature(1, "F", "P1", ""));
    let second = document("PRD-1", "draft", "2026-08-12", "atelier", Some("Second document"), &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "docs/prd/a-premier.md", &premier);
    ecrire(&racine, "docs/prd/b-second.md", &second);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 2, "les deux appels reussissent (idempotence), aucun n'echoue");

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 1, "un seul bloc au final, la cle prd_cle est partagee par construction");
}

/// Casse : un titre qui porte des guillemets et un antislash ne doit pas
/// casser la requete - ni le JSON qui la transporte (echappe par serde_json
/// comme partout ailleurs), ni la fonction SQL (parametree, jamais
/// concatenee).
#[tokio::test]
async fn un_titre_avec_guillemets_et_antislash_traverse_sans_encombre() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("titre-hostile");
    let plan = plan_simple("atelier", "local:x");

    let titre = "Kanban \"v2\" \\ le retour";
    let texte = document("PRD-1", "draft", "2026-08-12", "atelier", Some(titre), &section_feature(1, "F", "P1", ""));
    ecrire(&racine, "docs/prd/hostile.md", &texte);

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 1);

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs[0]["titre"], serde_json::json!(format!("cadrage {titre}")));
}

/// Casse : un fichier `.md` dont le contenu n'est pas de l'UTF-8 valide (un
/// export colle depuis un encodage different, par exemple) ne doit pas faire
/// paniquer `traiter` - il est simplement illisible comme texte, donc ignore
/// comme le serait un en-tete non conforme.
#[tokio::test]
async fn un_fichier_non_utf8_ne_fait_pas_paniquer_traiter() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("non-utf8");
    let plan = plan_simple("atelier", "local:x");

    std::fs::create_dir_all(racine.join("docs/prd")).unwrap();
    std::fs::write(racine.join("docs/prd/casse.md"), [0xFF, 0xFE, 0x00, 0xC3, 0x28]).unwrap();

    let resume = traiter(&client, &racine, &repo_id, &plan).await;
    assert_eq!(resume.blocs_poses, 0);
    assert!(ctx.lire_blocs(&repo_id).await.is_empty());
}

/// Le PRD-001 reel de ce depot : `statut: validé`, comme le note son propre
/// `[À CLARIFIER]` de F12 ("ce PRD-ci ne suit pas encore le gabarit qu'il
/// decrit ; il sera le premier cas de test du parseur"). Mis a jour par #37 :
/// ce document est desormais LE cas de test de la conversion elle-meme, pas
/// seulement du parseur - passe a `traiter`, ses seize features doivent
/// apparaitre en « A faire », sans bloc d'exploration (aucun n'a jamais ete
/// pose, ce depot n'a jamais vu ce document au stade `draft`).
///
/// Les chiffres verifies ici sont ceux du document tel qu'il existe au
/// moment ou ce test est ecrit (2026-08-12) : onze features P1 (F1-F11),
/// quatre P2 (F12-F15), une P3 (F16), et un seul marqueur `[À CLARIFIER]`,
/// porte par F12 - celui-la meme qui annonce que "ce PRD-ci ne suit pas
/// encore le gabarit qu'il decrit". Si PRD-001 est retouche plus tard (une
/// feature ajoutee, une priorite corrigee), ce test doit etre mis a jour
/// avec lui : c'est le prix d'avoir choisi un document reel comme fixture
/// plutot qu'un texte fabrique.
#[tokio::test]
async fn le_vrai_prd_001_du_depot_est_converti_en_seize_features() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    // La racine du depot lui-meme : `daemon/` est le repertoire du crate, son
    // parent est la racine ou vit `docs/prd/PRD-001-espace-projet-kanban.md`.
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let plan = plan_simple("vibecode-traker-app", "github.com/yarma-tech/vibecode-traker-app");

    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.features_creees, 16, "PRD-001 porte seize sections ### Fn");
    assert_eq!(resume.blocs_poses, 0, "aucune exploration n'a jamais ete posee pour ce document");
    assert!(
        resume.sans_feature_reconnue.iter().all(|c| !c.ends_with("PRD-001-espace-projet-kanban.md")),
        "PRD-001 porte 16 features reconnues, il ne doit pas etre signale comme vide"
    );
    assert!(
        resume.cles_dupliquees.is_empty(),
        "PRD-001 est seul dans ce depot de test, aucune collision de cle possible"
    );

    let blocs = ctx.lire_blocs(&repo_id).await;
    assert_eq!(blocs.len(), 16);
    assert!(blocs.iter().all(|b| b["type"] == serde_json::json!("feature")));
    assert!(blocs.iter().all(|b| b["statut"] == serde_json::json!("todo")), "toutes en A faire, aucune n'est nee ailleurs");
    assert!(blocs.iter().all(|b| b["prd_statut"] == serde_json::json!("validé")));
    assert!(
        blocs.iter().all(|b| b["prd_cle"]
            .as_str()
            .is_some_and(|c| c.starts_with("2026-08-10/PRD-001/F"))),
        "toutes les cles portent la date et l'id du document, jamais son titre"
    );

    let compte_priorite = |p: &str| {
        blocs.iter().filter(|b| b["prd_priorite"] == serde_json::json!(p)).count()
    };
    assert_eq!(compte_priorite("P1"), 11, "F1 a F11");
    assert_eq!(compte_priorite("P2"), 4, "F12 a F15");
    assert_eq!(compte_priorite("P3"), 1, "F16 seule");

    let a_clarifier: Vec<_> = blocs
        .iter()
        .filter(|b| b["prd_a_clarifier"] == serde_json::json!(true))
        .collect();
    assert_eq!(a_clarifier.len(), 1, "un seul marqueur [À CLARIFIER] dans tout le document, porte par F12");
    assert_eq!(a_clarifier[0]["prd_cle"], serde_json::json!("2026-08-10/PRD-001/F12"));
}
