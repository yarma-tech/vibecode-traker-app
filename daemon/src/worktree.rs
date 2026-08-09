//! Les worktrees d'un repo, lus par `git worktree list --porcelain` (issue #6).
//!
//! Un worktree est une copie de travail parallele du meme depot, sur une autre
//! branche. On ne garde que les worktrees LIES : le worktree principal est le
//! depot lui-meme, deja cartographie, et n'a rien a ajouter en surimpression.
//!
//! Rien de ce qui sort d'ici ne contient de chemin absolu : le chemin absolu
//! porte le nom de l'utilisateur du systeme (spec, section 7). Seul le nom du
//! dossier du worktree voyage, comme le nom d'un repo voyage deja.

use std::path::Path;
use std::process::Command;

/// Un worktree lie, pret a partir : sa branche et le nom de son dossier.
#[derive(Debug, Clone)]
pub struct Worktree {
    /// Le nom du dossier du worktree, sans son chemin. Sert de cle stable pour
    /// rapprocher deux releves : c'est ce qui distingue deux worktrees du repo.
    pub path: String,
    /// La branche extraite dans ce worktree. Un worktree detache, sans branche,
    /// porte l'abrege de son HEAD : il existe, on doit pouvoir le montrer.
    pub branch: String,
}

/// Les worktrees lies d'un repo. Le principal est ecarte, les erreurs de git
/// rendent une liste vide : un repo sans worktree et un repo illisible se
/// traitent pareil, la surimpression reste simplement absente.
pub fn worktrees(racine: &Path) -> Vec<Worktree> {
    let sortie = Command::new("git")
        .args(["worktree", "list", "--porcelain", "-z"])
        .current_dir(racine)
        .output()
        .ok();

    let Some(sortie) = sortie else {
        return Vec::new();
    };
    if !sortie.status.success() {
        return Vec::new();
    }

    analyser(&String::from_utf8_lossy(&sortie.stdout))
}

/// Decoupe la sortie porcelain en blocs et en tire les worktrees lies.
///
/// Chaque worktree est un bloc de lignes ; les blocs sont separes par un octet
/// nul en mode `-z`. La premiere ligne d'un bloc est toujours `worktree <chemin>`.
/// Le premier bloc est le worktree principal : on le saute.
fn analyser(porcelain: &str) -> Vec<Worktree> {
    // En `-z`, chaque attribut finit par un NUL, et un NUL vide separe les blocs.
    // On segmente donc sur `worktree ` en tete d'attribut.
    let mut worktrees = Vec::new();
    let mut chemin: Option<String> = None;
    let mut branche: Option<String> = None;
    let mut tete: Option<String> = None;
    let mut principal = true;

    let clore = |chemin: &mut Option<String>,
                     branche: &mut Option<String>,
                     tete: &mut Option<String>,
                     principal: &mut bool,
                     worktrees: &mut Vec<Worktree>| {
        if let Some(chemin) = chemin.take() {
            if *principal {
                // Le premier worktree est le depot lui-meme : deja sur la carte.
                *principal = false;
            } else {
                let nom = chemin
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&chemin)
                    .to_string();
                let branch = branche
                    .take()
                    .unwrap_or_else(|| detache(tete.as_deref()));
                worktrees.push(Worktree { path: nom, branch });
            }
        }
        *branche = None;
        *tete = None;
    };

    for attribut in porcelain.split('\0') {
        let attribut = attribut.trim_matches(['\n', '\r']);
        if let Some(valeur) = attribut.strip_prefix("worktree ") {
            // Nouveau bloc : on clot le precedent avant d'ouvrir celui-ci.
            clore(&mut chemin, &mut branche, &mut tete, &mut principal, &mut worktrees);
            chemin = Some(valeur.to_string());
        } else if let Some(valeur) = attribut.strip_prefix("branch ") {
            branche = Some(valeur.trim_start_matches("refs/heads/").to_string());
        } else if let Some(valeur) = attribut.strip_prefix("HEAD ") {
            tete = Some(valeur.to_string());
        }
    }
    clore(&mut chemin, &mut branche, &mut tete, &mut principal, &mut worktrees);

    worktrees
}

/// Un worktree detache n'a pas de branche : on montre l'abrege de son commit,
/// prefixe pour que l'ecran ne le prenne pas pour un nom de branche.
fn detache(tete: Option<&str>) -> String {
    match tete {
        Some(sha) if sha.len() >= 7 => format!("détaché {}", &sha[..7]),
        _ => "détaché".to_string(),
    }
}
