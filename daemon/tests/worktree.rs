//! Lecture des worktrees d'un repo (issue #6).
//!
//! Ces tests montent un vrai depot git dans un dossier temporaire, y ajoutent
//! un worktree, et verifient que `vibemap::worktrees` en tire la branche sans
//! jamais laisser fuiter un chemin absolu. C'est `git worktree list --porcelain`
//! qui decide, on ne reimplemente rien.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(racine: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(racine)
        .status()
        .expect("git");
    assert!(status.success(), "git {arguments:?} a echoue");
}

/// Un depot jetable, avec un premier commit : un worktree exige un historique.
fn depot_temporaire() -> PathBuf {
    let racine = std::env::temp_dir().join(format!("vibemap-wt-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&racine).unwrap();

    git(&racine, &["init", "--quiet"]);
    std::fs::write(racine.join("README.md"), "titre\n").unwrap();
    git(&racine, &["add", "."]);
    git(
        &racine,
        &[
            "-c",
            "user.email=test@vibemap.local",
            "-c",
            "user.name=Test",
            "commit",
            "--quiet",
            "-m",
            "premier",
        ],
    );

    racine
}

#[test]
fn un_worktree_ajoute_porte_sa_branche() {
    let racine = depot_temporaire();
    // Le worktree vit a cote du depot, sur une branche neuve.
    let ailleurs = racine.with_extension("hotfix");
    git(
        &racine,
        &[
            "worktree",
            "add",
            "-b",
            "hotfix",
            ailleurs.to_str().unwrap(),
        ],
    );

    let worktrees = vibemap::worktrees(&racine);

    // Le worktree principal (le depot lui-meme) ne compte pas : seul l'ajoute.
    assert_eq!(worktrees.len(), 1, "un seul worktree ajoute, le principal exclu");
    assert_eq!(worktrees[0].branch, "hotfix", "la branche du worktree, pas main");

    let _ = std::fs::remove_dir_all(&ailleurs);
    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn un_depot_sans_worktree_n_en_rend_aucun() {
    let racine = depot_temporaire();

    let worktrees = vibemap::worktrees(&racine);

    assert!(
        worktrees.is_empty(),
        "seul le worktree principal existe, et il ne compte pas : {worktrees:?}"
    );

    let _ = std::fs::remove_dir_all(&racine);
}

#[test]
fn aucun_chemin_absolu_ne_sort_du_worktree() {
    let racine = depot_temporaire();
    let ailleurs = racine.with_extension("hotfix");
    git(
        &racine,
        &["worktree", "add", "-b", "hotfix", ailleurs.to_str().unwrap()],
    );

    let worktrees = vibemap::worktrees(&racine);
    let serialise = format!("{worktrees:?}");
    let absolu = ailleurs.to_string_lossy().to_string();

    assert!(
        !serialise.contains(&absolu),
        "aucun chemin absolu ne doit figurer dans ce qui part vers Supabase : {serialise}"
    );

    let _ = std::fs::remove_dir_all(&ailleurs);
    let _ = std::fs::remove_dir_all(&racine);
}
