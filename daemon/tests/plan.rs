//! Cartographie d'un repo (issue #3).
//!
//! Un dossier par ligne, avec ses lignes de code cumulees. Ces tests montent
//! un vrai depot git dans un dossier temporaire : c'est `git` lui-meme qui
//! decide ce qui est ignore, on ne reimplemente pas `.gitignore`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn ecrire(racine: &Path, chemin: &str, contenu: &str) {
    let complet = racine.join(chemin);
    std::fs::create_dir_all(complet.parent().unwrap()).unwrap();
    std::fs::write(complet, contenu).unwrap();
}

/// Un depot jetable, avec de quoi eprouver chaque regle.
fn depot_temporaire() -> PathBuf {
    let racine = std::env::temp_dir().join(format!("vibemap-plan-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&racine).unwrap();

    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&racine)
        .status()
        .expect("git init");

    ecrire(&racine, ".gitignore", "node_modules/\n*.log\n");
    ecrire(&racine, "README.md", "titre\n");
    ecrire(&racine, "src/a.rs", "une\ndeux\ntrois\n");
    ecrire(&racine, "src/core/b.rs", "une\ndeux\n");
    ecrire(&racine, "docs/x.md", "doc\n");

    // Ignore par git : ne doit jamais apparaitre.
    ecrire(&racine, "node_modules/paquet/index.js", &"x\n".repeat(500));
    ecrire(&racine, "bruit.log", "ligne\n");

    // Binaire : present pour git, mais ne compte pas en lignes de code.
    std::fs::write(racine.join("logo.png"), [0x89, 0x50, 0x4E, 0x47, 0x00, 0x01]).unwrap();

    racine
}

fn module<'a>(plan: &'a vibemap::Plan, chemin: &str) -> &'a vibemap::Module {
    plan.modules
        .iter()
        .find(|m| m.path == chemin)
        .unwrap_or_else(|| {
            panic!(
                "module « {chemin} » absent. Presents : {:?}",
                plan.modules.iter().map(|m| &m.path).collect::<Vec<_>>()
            )
        })
}

#[test]
fn cartographie_les_dossiers_avec_leurs_lignes() {
    let racine = depot_temporaire();
    let plan = vibemap::scanner(&racine).expect("le scan doit reussir");

    // Lignes cumulees : un dossier porte le total de ce qu'il contient.
    assert_eq!(module(&plan, "src").loc, 5, "src contient a.rs (3) et core/b.rs (2)");
    assert_eq!(module(&plan, "src/core").loc, 2);
    assert_eq!(module(&plan, "docs").loc, 1);

    // Les fichiers poses a la racine forment leur propre parcelle.
    // Le .gitignore en fait partie : c'est un fichier du repo, il pese.
    assert_eq!(
        module(&plan, ".").loc,
        3,
        "a la racine : README.md (1) et .gitignore (2)"
    );

    assert_eq!(plan.loc_total, 9, "src 5 + docs 1 + racine 3");
}

#[test]
fn ce_que_git_ignore_n_existe_pas() {
    let racine = depot_temporaire();
    let plan = vibemap::scanner(&racine).expect("le scan doit reussir");

    assert!(
        !plan.modules.iter().any(|m| m.path.starts_with("node_modules")),
        "node_modules est dans le .gitignore : il ne doit pas apparaitre"
    );
    assert!(
        plan.loc_total < 100,
        "les 500 lignes de node_modules ne doivent pas etre comptees, total obtenu : {}",
        plan.loc_total
    );
}

#[test]
fn les_binaires_ne_comptent_pas_en_lignes() {
    let racine = depot_temporaire();
    let plan = vibemap::scanner(&racine).expect("le scan doit reussir");

    // logo.png vit a la racine, aux cotes de README.md et .gitignore :
    // il ne doit rien ajouter aux trois lignes de ses voisins.
    assert_eq!(
        module(&plan, ".").loc,
        3,
        "un fichier binaire n'apporte aucune ligne"
    );
}

#[test]
fn chaque_module_connait_son_parent_et_sa_profondeur() {
    let racine = depot_temporaire();
    let plan = vibemap::scanner(&racine).expect("le scan doit reussir");

    let src = module(&plan, "src");
    assert_eq!(src.depth, 1);
    assert_eq!(src.parent_path.as_deref(), Some(""));

    let core = module(&plan, "src/core");
    assert_eq!(core.depth, 2);
    assert_eq!(core.parent_path.as_deref(), Some("src"));
}

#[test]
fn le_chemin_absolu_ne_sort_pas_du_plan() {
    let racine = depot_temporaire();
    let plan = vibemap::scanner(&racine).expect("le scan doit reussir");

    let absolu = racine.to_string_lossy().to_string();
    let serialise = serde_json::to_string(&plan).expect("le plan doit etre serialisable");

    assert!(
        !serialise.contains(&absolu),
        "aucun chemin absolu ne doit figurer dans ce qui part vers Supabase"
    );
    assert_eq!(plan.root_hash.len(), 64, "l'empreinte est un sha256 hexadecimal");
}

/// L'invariant qui rend la carte honnete : la somme des parcelles filles fait
/// exactement la parcelle mere. Sans lui, les surfaces racontent n'importe quoi.
#[test]
fn la_somme_des_filles_fait_la_mere() {
    let racine = depot_temporaire();
    let plan = vibemap::scanner(&racine).expect("le scan doit reussir");

    let filles = |parent: &str| -> u32 {
        plan.modules
            .iter()
            .filter(|m| m.parent_path.as_deref() == Some(parent))
            .map(|m| m.loc)
            .sum()
    };

    assert_eq!(
        filles(""),
        plan.loc_total,
        "les parcelles du premier niveau doivent couvrir tout le repo"
    );

    assert_eq!(
        filles("src"),
        module(&plan, "src").loc,
        "src/core et les fichiers de src doivent couvrir src"
    );

    // Les fichiers poses dans src ont leur propre parcelle.
    assert_eq!(module(&plan, "src/.").loc, 3, "a.rs seul est pose dans src");
}
