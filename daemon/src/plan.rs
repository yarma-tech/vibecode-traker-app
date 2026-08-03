//! Cartographie d'un repo : un dossier par parcelle, avec ses lignes cumulees.
//!
//! C'est `git` qui decide ce qui existe. On lui demande la liste des fichiers
//! suivis et des fichiers non ignores, ce qui applique le `.gitignore` du repo
//! sans avoir a le reimplementer.
//!
//! Rien de ce qui sort d'ici ne contient de chemin absolu : la racine devient
//! une empreinte, et tous les chemins de modules sont relatifs.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Dossiers qu'on ecarte meme quand le repo a oublie de les ignorer.
const TOUJOURS_EXCLUS: &[&str] = &[
    "node_modules", ".git", "dist", "build", "target", ".next", "vendor",
];

/// Extensions qu'on ne compte jamais en lignes de code.
const BINAIRES: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "icns", "pdf", "zip",
    "gz", "tar", "bz2", "7z", "mp3", "mp4", "mov", "wav", "woff", "woff2",
    "ttf", "otf", "eot", "so", "dylib", "dll", "a", "o", "bin", "wasm",
    "class", "jar", "pyc", "db", "sqlite", "lock",
];

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("{0} n'est pas un depot git")]
    PasUnDepot(String),

    #[error("git a echoue dans {chemin} : {details}")]
    GitEnEchec { chemin: String, details: String },

    #[error("lecture impossible dans {0} : {1}")]
    Lecture(String, std::io::Error),
}

#[derive(Debug, Serialize)]
pub struct Module {
    pub path: String,
    pub parent_path: Option<String>,
    pub depth: i32,
    pub loc: u32,
    pub file_count: u32,
}

#[derive(Debug, Serialize)]
pub struct Plan {
    pub name: String,
    pub root_hash: String,
    pub remote_owner: Option<String>,
    pub remote_url: Option<String>,
    pub current_branch: Option<String>,
    pub loc_total: u32,
    pub file_count: u32,
    pub modules: Vec<Module>,
}

pub fn scanner(racine: &Path) -> Result<Plan, ScanError> {
    if !racine.join(".git").exists() {
        return Err(ScanError::PasUnDepot(nom(racine)));
    }

    let fichiers = fichiers_du_depot(racine)?;

    // Deux comptes par dossier : ce qu'il contient en tout, et ce qui est pose
    // directement dedans. Sans le second, la somme des parcelles filles ne
    // ferait pas la parcelle mere, et la carte mentirait sur les surfaces.
    let mut cumul: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let mut direct: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let mut sous_dossiers: BTreeMap<String, bool> = BTreeMap::new();
    let mut loc_total = 0u32;
    let mut file_count = 0u32;

    for relatif in fichiers {
        if exclu(&relatif) {
            continue;
        }

        // `git ls-files` annonce aussi les sous-modules, qui sont des dossiers.
        // Les lire comme des fichiers echoue, et les compter comme tels
        // fausserait le nombre de fichiers.
        let absolu = racine.join(&relatif);
        if absolu.is_dir() {
            continue;
        }

        let lignes = compter_lignes(&absolu)?;
        if lignes == 0 && est_binaire(&relatif) {
            continue;
        }

        loc_total += lignes;
        file_count += 1;

        // Le dossier qui contient directement ce fichier, « » pour la racine.
        let contenant = match relatif.rfind('/') {
            Some(i) => relatif[..i].to_string(),
            None => String::new(),
        };
        let entree = direct.entry(contenant).or_insert((0, 0));
        entree.0 += lignes;
        entree.1 += 1;

        for dossier in ancetres(&relatif) {
            sous_dossiers.insert(parent(&dossier), true);
            let entree = cumul.entry(dossier).or_insert((0, 0));
            entree.0 += lignes;
            entree.1 += 1;
        }
    }

    let mut modules: Vec<Module> = cumul
        .into_iter()
        .map(|(path, (loc, file_count))| Module {
            depth: profondeur(&path),
            parent_path: Some(parent(&path)),
            path,
            loc,
            file_count,
        })
        .collect();

    // Un dossier qui melange sous-dossiers et fichiers gagne une parcelle pour
    // ses fichiers a lui. Un dossier sans sous-dossier n'en a pas besoin : il
    // est deja sa propre feuille.
    for (contenant, (loc, file_count)) in direct {
        if !sous_dossiers.get(&contenant).copied().unwrap_or(false) {
            continue;
        }

        let path = if contenant.is_empty() {
            ".".to_string()
        } else {
            format!("{contenant}/.")
        };

        modules.push(Module {
            depth: profondeur(&path),
            parent_path: Some(contenant),
            path,
            loc,
            file_count,
        });
    }

    modules.sort_by(|a, b| b.loc.cmp(&a.loc));

    Ok(Plan {
        name: nom(racine),
        root_hash: empreinte(racine),
        remote_owner: remote(racine).as_deref().and_then(proprietaire),
        remote_url: remote(racine),
        current_branch: branche(racine),
        loc_total,
        file_count,
        modules,
    })
}

/// Les fichiers suivis, plus les fichiers non suivis que git n'ignore pas.
/// Autrement dit : exactement ce que `git status` considere comme le depot.
fn fichiers_du_depot(racine: &Path) -> Result<Vec<String>, ScanError> {
    let sortie = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard", "-z"])
        .current_dir(racine)
        .output()
        .map_err(|e| ScanError::GitEnEchec {
            chemin: nom(racine),
            details: e.to_string(),
        })?;

    if !sortie.status.success() {
        return Err(ScanError::GitEnEchec {
            chemin: nom(racine),
            details: String::from_utf8_lossy(&sortie.stderr).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&sortie.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

/// Filet de securite : certains repos oublient d'ignorer leurs dossiers lourds.
fn exclu(relatif: &str) -> bool {
    relatif
        .split('/')
        .any(|segment| TOUJOURS_EXCLUS.contains(&segment))
}

fn est_binaire(relatif: &str) -> bool {
    Path::new(relatif)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| BINAIRES.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Compte les lignes d'un fichier texte. Un binaire vaut zero.
fn compter_lignes(chemin: &Path) -> Result<u32, ScanError> {
    if est_binaire(&chemin.to_string_lossy()) {
        return Ok(0);
    }

    match std::fs::read(chemin) {
        // Un octet nul trahit un binaire que l'extension n'annoncait pas.
        Ok(octets) if octets.contains(&0) => Ok(0),
        Ok(octets) => Ok(octets.iter().filter(|o| **o == b'\n').count() as u32),
        // Un lien casse ou un fichier disparu pendant le scan n'arrete rien.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(ScanError::Lecture(chemin.display().to_string(), e)),
    }
}

/// Les dossiers auxquels un fichier appartient. Un fichier pose a la racine
/// n'en a aucun : sa parcelle est synthetisee plus loin.
fn ancetres(relatif: &str) -> Vec<String> {
    let segments: Vec<&str> = relatif.split('/').collect();

    (1..segments.len())
        .map(|fin| segments[..fin].join("/"))
        .collect()
}

fn profondeur(path: &str) -> i32 {
    path.split('/').count() as i32
}

fn parent(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

fn nom(racine: &Path) -> String {
    racine
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo sans nom".to_string())
}

/// Le chemin absolu contient le nom de l'utilisateur du systeme : il reste ici.
fn empreinte(racine: &Path) -> String {
    let mut hachoir = Sha256::new();
    hachoir.update(racine.to_string_lossy().as_bytes());
    format!("{:x}", hachoir.finalize())
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

fn remote(racine: &Path) -> Option<String> {
    git(racine, &["remote", "get-url", "origin"])
}

fn branche(racine: &Path) -> Option<String> {
    git(racine, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// « git@github.com:yarma-tech/vibemap.git » et
/// « https://github.com/yarma-tech/vibemap.git » donnent tous deux yarma-tech.
fn proprietaire(url: &str) -> Option<String> {
    let sans_protocole = url
        .rsplit_once("://")
        .map(|(_, reste)| reste)
        .unwrap_or(url);
    let sans_hote = sans_protocole
        .rsplit_once(':')
        .map(|(_, reste)| reste)
        .or_else(|| sans_protocole.split_once('/').map(|(_, reste)| reste))?;

    sans_hote.split('/').next().map(str::to_string)
}
