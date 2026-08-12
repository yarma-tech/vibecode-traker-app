//! Les commits locaux qui ferment un travail par reference (issue #32).
//!
//! Rien ici ne sait ce qu'est une reference ni ce que fermer veut dire : ce
//! module se contente de lire `git log`, comme `plan.rs` lit l'arbre des
//! fichiers et `worktree.rs` la liste des copies de travail. La fermeture
//! elle-meme vit en SQL (`fermer_par_reference`), a l'abri du reseau et
//! rejouable sans effet de bord (conception, §5).

use chrono::{DateTime, Utc};
use std::path::Path;
use std::process::Command;

/// Un commit local, pret a partir. Le message part en clair vers Supabase
/// (PRD §7.1, liste fermee) : c'est deja le cas pour les commits affiches
/// ailleurs, et c'est lui qui porte la reference qui ferme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitLocal {
    pub sha: String,
    pub message: String,
    pub authored_at: DateTime<Utc>,
}

/// `HEAD` d'un depot, ou `None` s'il n'a encore aucun commit.
///
/// C'est ce que le daemon pose comme position de depart au premier passage
/// sur un depot, sans rien fermer : le passe n'est jamais rejoue (PRD,
/// risques - « mieux vaut rater une fermeture que d'en inventer une »).
pub fn head(racine: &Path) -> Option<String> {
    git(racine, &["rev-parse", "HEAD"])
}

/// La branche courante, comme `plan.rs` la lit pour la carte.
pub fn branche_courante(racine: &Path) -> Option<String> {
    git(racine, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Separateurs qu'aucun message de commit legitime ne porte, contrairement a
/// une virgule ou un saut de ligne qu'un message ordinaire contient deja.
/// Git les insere lui-meme via `%x1f`/`%x1e` dans le format demande : ce ne
/// sont donc jamais des octets qu'un message pourrait injecter.
const SEP_CHAMP: char = '\u{1f}';
const SEP_ENREGISTREMENT: char = '\u{1e}';

/// Les commits poses depuis `depuis` (exclusif) jusqu'a `HEAD`, du plus
/// ancien au plus recent - l'ordre dans lequel ils doivent fermer (§5.2 de
/// la conception). Les fusions sont ecartees : elles n'ont jamais nomme un
/// travail par elles-memes, seuls les commits qu'elles integrent l'ont deja
/// fait, un par un.
///
/// `depuis` doit toujours designer un commit qui existe deja dans le depot :
/// c'est `head()`, pose au premier passage, qui garantit ça.
pub fn commits_depuis(racine: &Path, depuis: &str) -> Vec<CommitLocal> {
    let plage = format!("{depuis}..HEAD");

    let sortie = Command::new("git")
        .args([
            "log",
            &plage,
            "--no-merges",
            "--reverse",
            "--format=%H%x1f%aI%x1f%B%x1e",
        ])
        .current_dir(racine)
        .output();

    let Ok(sortie) = sortie else {
        return Vec::new();
    };
    if !sortie.status.success() {
        return Vec::new();
    }

    analyser(&String::from_utf8_lossy(&sortie.stdout))
}

/// Decoupe la sortie en enregistrements, puis chaque enregistrement en ses
/// trois champs : sha, date d'auteur, message (corps compris).
fn analyser(sortie: &str) -> Vec<CommitLocal> {
    sortie
        .split(SEP_ENREGISTREMENT)
        .map(str::trim)
        .filter(|bloc| !bloc.is_empty())
        .filter_map(|bloc| {
            let mut champs = bloc.splitn(3, SEP_CHAMP);
            let sha = champs.next()?.to_string();
            let date = champs.next()?;
            let message = champs.next().unwrap_or_default().trim().to_string();
            let authored_at = DateTime::parse_from_rfc3339(date).ok()?.with_timezone(&Utc);
            Some(CommitLocal { sha, message, authored_at })
        })
        .collect()
}

fn git(racine: &Path, arguments: &[&str]) -> Option<String> {
    let sortie = Command::new("git")
        .args(arguments)
        .current_dir(racine)
        .output()
        .ok()?;

    if !sortie.status.success() {
        return None;
    }

    let valeur = String::from_utf8_lossy(&sortie.stdout).trim().to_string();
    (!valeur.is_empty()).then_some(valeur)
}
