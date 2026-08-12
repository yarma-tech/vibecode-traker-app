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

/// FR-038, cote negatif : un PRD deja `validé` ne laisse AUCUN bloc
/// d'exploration - la conversion en features est #37, pas cette issue.
#[tokio::test]
async fn un_prd_valide_ne_pose_aucun_bloc_pour_linstant() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    let racine = depot_git_temporaire("valide");
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
    assert_eq!(resume.blocs_poses, 0, "#37 s'occupera de la conversion, pas cette issue-ci");
    assert!(ctx.lire_blocs(&repo_id).await.is_empty());
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
/// decrit ; il sera le premier cas de test du parseur"). Ce test EST ce cas
/// de test : passe a `traiter`, il ne doit creer ni bloc d'exploration
/// (reserve aux brouillons, FR-038) ni feature (conversion = #37).
#[tokio::test]
async fn le_vrai_prd_001_du_depot_ne_cree_ni_exploration_ni_feature() {
    let ctx = common::TestContext::new().await;
    let (client, repo_id) = repo_de_test(&ctx).await;
    // La racine du depot lui-meme : `daemon/` est le repertoire du crate, son
    // parent est la racine ou vit `docs/prd/PRD-001-espace-projet-kanban.md`.
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let plan = plan_simple("vibecode-traker-app", "github.com/yarma-tech/vibecode-traker-app");

    let resume = traiter(&client, &racine, &repo_id, &plan).await;

    assert_eq!(resume.blocs_poses, 0, "PRD-001 est valide, pas draft : aucun bloc d'exploration");
    assert!(
        resume.sans_feature_reconnue.iter().all(|c| !c.ends_with("PRD-001-espace-projet-kanban.md")),
        "PRD-001 porte 16 features reconnues, il ne doit pas etre signale comme vide"
    );
    assert!(
        ctx.lire_blocs(&repo_id).await.is_empty(),
        "aucune ligne de blocs ne doit exister pour ce depot apres la lecture de PRD-001"
    );
}
