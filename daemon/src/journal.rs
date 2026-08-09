//! Ce que les agents font, lu dans leurs propres journaux.
//!
//! Claude Code ecrit un `.jsonl` par session sous `~/.claude/projects`. Chaque
//! appel d'outil y laisse une ligne. On en retient le strict necessaire : quelle
//! session, quel outil, quel chemin, a quelle heure. Jamais le contenu d'un
//! fichier, jamais un prompt, jamais le resultat d'un outil.
//!
//! `Bash` ne produit rien : la commande peut toucher n'importe quoi, et deviner
//! serait pire que se taire.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Ce qu'un appel d'outil fait au disque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nature {
    Lecture,
    Ecriture,
}

impl Nature {
    /// Le nom que porte la colonne `kind` dans la base.
    pub fn code(self) -> &'static str {
        match self {
            Nature::Lecture => "read",
            Nature::Ecriture => "write",
        }
    }
}

/// Un appel d'outil, reduit a ce qui peut sortir de la machine sans risque.
#[derive(Debug, Clone)]
pub struct Evenement {
    pub session_id: String,
    pub tool_use_id: String,
    pub instant: DateTime<Utc>,
    /// Dossier courant de l'agent. Sert a retrouver le repo, reste ici.
    pub cwd: String,
    pub branche: Option<String>,
    /// Chemin absolu vise par l'outil. Il porte le nom de l'utilisateur du
    /// systeme : seule sa part relative au repo part vers Supabase.
    pub chemin: String,
    /// Un `Grep` vise un dossier entier, pas un fichier.
    pub dossier: bool,
    pub nature: Nature,
}

/// La consommation d'une ligne d'assistant : jetons et modele.
///
/// Chaque reponse de l'agent porte son propre bloc `usage`. Les additionner sur
/// une session en donne le total. Jamais un prompt, jamais un contenu : rien que
/// des nombres et un nom de modele.
#[derive(Debug, Clone)]
pub struct Usage {
    pub session_id: String,
    /// Dossier courant de l'agent. Sert a retrouver le repo, reste ici.
    pub cwd: String,
    pub branche: Option<String>,
    pub model: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub instant: DateTime<Utc>,
}

/// La correspondance de la spec, section 5.1. Tout le reste se tait.
fn nature(outil: &str) -> Option<Nature> {
    match outil {
        "Read" | "Grep" | "Glob" => Some(Nature::Lecture),
        "Edit" | "Write" | "NotebookEdit" => Some(Nature::Ecriture),
        _ => None,
    }
}

/// Le champ ou chaque outil range sa cible, et si cette cible est un dossier.
fn cible(outil: &str, entree: &Value) -> Option<(String, bool)> {
    let champ = |nom: &str| entree.get(nom).and_then(Value::as_str).map(str::to_string);

    match outil {
        "NotebookEdit" => champ("notebook_path").map(|c| (c, false)),
        // Sans chemin, une recherche porte sur le dossier courant. L'appelant
        // remplace la chaine vide par le `cwd`.
        "Grep" | "Glob" => Some((champ("path").unwrap_or_default(), true)),
        _ => champ("file_path").map(|c| (c, false)),
    }
}

/// Transforme un morceau de journal en evenements.
///
/// Une ligne illisible est sautee sans bruit : un journal en cours d'ecriture
/// se termine souvent par une ligne incomplete, et ce n'est pas une panne.
pub fn lire(contenu: &str) -> Vec<Evenement> {
    contenu.lines().flat_map(lire_une_ligne).collect()
}

fn lire_une_ligne(ligne: &str) -> Vec<Evenement> {
    let Ok(entree) = serde_json::from_str::<Value>(ligne) else {
        return Vec::new();
    };

    if entree.get("type").and_then(Value::as_str) != Some("assistant") {
        return Vec::new();
    }

    let (Some(session_id), Some(cwd), Some(instant)) = (
        entree.get("sessionId").and_then(Value::as_str),
        entree.get("cwd").and_then(Value::as_str),
        entree
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|t| t.parse::<DateTime<Utc>>().ok()),
    ) else {
        return Vec::new();
    };

    let branche = entree
        .get("gitBranch")
        .and_then(Value::as_str)
        .filter(|b| !b.is_empty())
        .map(str::to_string);

    let blocs = entree
        .pointer("/message/content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    blocs
        .iter()
        .filter(|bloc| bloc.get("type").and_then(Value::as_str) == Some("tool_use"))
        .filter_map(|bloc| {
            let outil = bloc.get("name").and_then(Value::as_str)?;
            let nature = nature(outil)?;
            let tool_use_id = bloc.get("id").and_then(Value::as_str)?;
            let (brut, dossier) = cible(outil, bloc.get("input").unwrap_or(&Value::Null))?;

            Some(Evenement {
                session_id: session_id.to_string(),
                tool_use_id: tool_use_id.to_string(),
                instant,
                cwd: cwd.to_string(),
                branche: branche.clone(),
                chemin: absolu(&brut, cwd),
                dossier,
                nature,
            })
        })
        .collect()
}

/// Transforme un morceau de journal en releves de consommation.
///
/// Comme `lire`, une ligne illisible est sautee sans bruit. Une ligne sans bloc
/// `usage` n'apporte rien : ce n'est pas une reponse facturable.
pub fn lire_usage(contenu: &str) -> Vec<Usage> {
    contenu.lines().filter_map(lire_usage_une_ligne).collect()
}

fn lire_usage_une_ligne(ligne: &str) -> Option<Usage> {
    let entree = serde_json::from_str::<Value>(ligne).ok()?;

    if entree.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }

    let session_id = entree.get("sessionId").and_then(Value::as_str)?;
    let cwd = entree.get("cwd").and_then(Value::as_str)?;
    let instant = entree
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|t| t.parse::<DateTime<Utc>>().ok())?;

    let usage = entree.pointer("/message/usage")?;
    let jeton = |nom: &str| usage.get(nom).and_then(Value::as_i64).unwrap_or(0);

    let branche = entree
        .get("gitBranch")
        .and_then(Value::as_str)
        .filter(|b| !b.is_empty())
        .map(str::to_string);

    Some(Usage {
        session_id: session_id.to_string(),
        cwd: cwd.to_string(),
        branche,
        model: entree
            .pointer("/message/model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        input: jeton("input_tokens"),
        output: jeton("output_tokens"),
        cache_read: jeton("cache_read_input_tokens"),
        cache_creation: jeton("cache_creation_input_tokens"),
        instant,
    })
}

/// Un outil peut viser un chemin relatif : il se lit depuis le dossier courant.
fn absolu(brut: &str, cwd: &str) -> String {
    if brut.is_empty() {
        return cwd.to_string();
    }

    if brut.starts_with('/') {
        return brut.to_string();
    }

    Path::new(cwd).join(brut).to_string_lossy().to_string()
}

/// Range un chemin absolu dans le repo : rend le module et le chemin relatif.
///
/// Un chemin qui ne descend pas de la racine n'appartient a aucun module :
/// l'agent travaillait ailleurs, et la carte n'a rien a en dire.
pub fn localiser(chemin: &str, dossier: bool, racine: &Path) -> Option<(String, String)> {
    let relatif = Path::new(chemin)
        .strip_prefix(racine)
        .ok()?
        .to_string_lossy()
        .to_string();

    // La racine elle-meme : le module est le repo entier.
    if relatif.is_empty() {
        return Some((String::new(), ".".to_string()));
    }

    if dossier {
        return Some((relatif.clone(), relatif));
    }

    let module = match relatif.rfind('/') {
        Some(i) => relatif[..i].to_string(),
        None => String::new(),
    };

    Some((module, relatif))
}

/// Le meme evenement, mais tenu d'un hook `PostToolUse` au lieu du journal.
///
/// Le hook n'est jamais requis : il ne fait qu'arriver avant le journal. Comme
/// il porte le meme `tool_use_id`, la contrainte d'unicite absorbe celui des
/// deux qui arrive second.
pub fn depuis_hook(charge: &str) -> Option<Evenement> {
    let charge: Value = serde_json::from_str(charge).ok()?;

    let outil = charge.get("tool_name").and_then(Value::as_str)?;
    let nature = nature(outil)?;
    let cwd = charge.get("cwd").and_then(Value::as_str)?;
    let entree = charge.get("tool_input").unwrap_or(&Value::Null);
    let (brut, dossier) = cible(outil, entree)?;

    Some(Evenement {
        session_id: charge.get("session_id").and_then(Value::as_str)?.to_string(),
        tool_use_id: charge.get("tool_use_id").and_then(Value::as_str)?.to_string(),
        // Le hook parle au moment ou l'outil vient de rendre la main.
        instant: Utc::now(),
        cwd: cwd.to_string(),
        branche: None,
        chemin: absolu(&brut, cwd),
        dossier,
        nature,
    })
}

/// Le depot auquel appartient un dossier, en remontant jusqu'au premier `.git`.
///
/// Dans un worktree, `.git` est un fichier et non un dossier : d'ou le test
/// d'existence plutot que de type.
pub fn racine_git(depart: &Path) -> Option<PathBuf> {
    let mut courant = Some(depart);

    while let Some(dossier) = courant {
        if dossier.join(".git").exists() {
            return Some(dossier.to_path_buf());
        }
        courant = dossier.parent();
    }

    None
}

/// Ce qu'un repo recoit d'un tour de lecture.
#[derive(Debug)]
pub struct Lot {
    pub repo_id: String,
    pub branche: Option<String>,
    pub activites: Vec<crate::Activite>,
}

/// Range les evenements par repo, et les traduit en chemins relatifs.
///
/// Le repo se deduit du dossier courant de l'agent : la racine connue la plus
/// profonde qui le contient. Un travail hors de tout repo cartographie, ou un
/// fichier hors du repo courant, est ecarte sans bruit. C'est la seule barriere
/// entre le disque entier et ce qui part vers Supabase : un chemin qui ne
/// descend pas d'une racine connue ne franchit pas cette fonction.
pub fn rattacher(
    evenements: &[Evenement],
    repos: &std::collections::BTreeMap<PathBuf, String>,
) -> Vec<Lot> {
    let mut lots: Vec<Lot> = Vec::new();

    for evenement in evenements {
        let Some((racine, repo_id)) = repo_de(&evenement.cwd, repos) else {
            continue;
        };
        let Some((module_path, file_path)) =
            localiser(&evenement.chemin, evenement.dossier, racine)
        else {
            continue;
        };

        let activite = crate::Activite {
            session_id: evenement.session_id.clone(),
            tool_use_id: evenement.tool_use_id.clone(),
            module_path,
            file_path,
            kind: evenement.nature.code(),
            occurred_at: evenement.instant,
        };

        match lots.iter_mut().find(|lot| lot.repo_id == *repo_id) {
            Some(lot) => {
                // La branche la plus recente gagne : un agent peut changer de
                // branche en cours de session.
                if evenement.branche.is_some() {
                    lot.branche = evenement.branche.clone();
                }
                lot.activites.push(activite);
            }
            None => lots.push(Lot {
                repo_id: repo_id.clone(),
                branche: evenement.branche.clone(),
                activites: vec![activite],
            }),
        }
    }

    lots
}

/// Ce qu'un repo recoit d'un tour de lecture, cote consommation.
#[derive(Debug)]
pub struct LotUsage {
    pub repo_id: String,
    pub branche: Option<String>,
    pub sessions: Vec<crate::SessionCout>,
}

/// Range la consommation par repo, puis par session.
///
/// Meme barriere que `rattacher` : un usage dont le dossier courant ne descend
/// d'aucune racine connue ne franchit pas cette fonction. Les jetons d'une
/// session s'additionnent ; le modele retenu est celui de la reponse la plus
/// recente, car un agent peut changer de modele en cours de session.
pub fn rattacher_usage(
    usages: &[Usage],
    repos: &std::collections::BTreeMap<PathBuf, String>,
) -> Vec<LotUsage> {
    let mut lots: Vec<LotUsage> = Vec::new();

    for usage in usages {
        let Some((_, repo_id)) = repo_de(&usage.cwd, repos) else {
            continue;
        };

        let idx = match lots.iter().position(|lot| lot.repo_id == *repo_id) {
            Some(i) => i,
            None => {
                lots.push(LotUsage {
                    repo_id: repo_id.clone(),
                    branche: None,
                    sessions: Vec::new(),
                });
                lots.len() - 1
            }
        };
        let lot = &mut lots[idx];

        if usage.branche.is_some() {
            lot.branche = usage.branche.clone();
        }

        match lot
            .sessions
            .iter_mut()
            .find(|s| s.session_id == usage.session_id)
        {
            Some(s) => {
                s.input += usage.input;
                s.output += usage.output;
                s.cache_read += usage.cache_read;
                s.cache_creation += usage.cache_creation;
                s.debut = s.debut.min(usage.instant);
                // La reponse la plus recente fixe le modele et la borne haute.
                if usage.instant >= s.fin {
                    s.fin = usage.instant;
                    s.model = usage.model.clone();
                }
            }
            None => lot.sessions.push(crate::SessionCout {
                session_id: usage.session_id.clone(),
                model: usage.model.clone(),
                input: usage.input,
                output: usage.output,
                cache_read: usage.cache_read,
                cache_creation: usage.cache_creation,
                debut: usage.instant,
                fin: usage.instant,
            }),
        }
    }

    lots
}

/// La racine connue la plus profonde qui contient ce dossier.
///
/// Compare segment par segment : « atelier-bis » ne descend pas de « atelier »,
/// meme si la chaine commence pareil.
fn repo_de<'a>(
    cwd: &str,
    repos: &'a std::collections::BTreeMap<PathBuf, String>,
) -> Option<(&'a Path, &'a String)> {
    repos
        .iter()
        .filter(|(racine, _)| Path::new(cwd).starts_with(racine))
        .max_by_key(|(racine, _)| racine.components().count())
        .map(|(racine, id)| (racine.as_path(), id))
}

/// Ou en est la lecture de chaque journal.
///
/// Les journaux ne font que grandir : retenir un decalage par fichier evite de
/// relire des megaoctets a chaque tour. Un fichier vu pour la premiere fois
/// n'apporte que ce qui est plus recent que l'horizon, sans quoi le premier
/// demarrage rejouerait des semaines d'activite.
#[derive(Debug, Default)]
pub struct Suivi {
    decalages: HashMap<PathBuf, u64>,
}

/// Ce qu'un tour de lecture tire des journaux : ce que les agents ont fait, et
/// ce qu'ils ont consomme. Les deux sortent d'une seule passe sur chaque
/// fichier, pour que le decalage n'avance qu'une fois.
#[derive(Debug, Default)]
pub struct Lecture {
    pub evenements: Vec<Evenement>,
    pub usages: Vec<Usage>,
}

impl Suivi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nouveaux(&mut self, racine: &Path, horizon: DateTime<Utc>) -> Lecture {
        let mut lecture = Lecture::default();

        for chemin in journaux(racine) {
            let Ok(infos) = std::fs::metadata(&chemin) else {
                continue;
            };
            let taille = infos.len();

            let premiere_vue = !self.decalages.contains_key(&chemin);
            if premiere_vue && !touche_depuis(&infos, horizon) {
                // Journal dormant : rien a en tirer, on se place a sa fin.
                self.decalages.insert(chemin, taille);
                continue;
            }

            // Un fichier plus court qu'avant a ete remplace : on repart de zero.
            let depart = match self.decalages.get(&chemin) {
                Some(&decalage) if decalage <= taille => decalage,
                _ => 0,
            };

            if depart == taille {
                continue;
            }

            let Some((texte, avance)) = morceau(&chemin, depart) else {
                continue;
            };
            self.decalages.insert(chemin, depart + avance);

            lecture.evenements.extend(
                lire(&texte)
                    .into_iter()
                    .filter(|e| !premiere_vue || e.instant >= horizon),
            );
            lecture.usages.extend(
                lire_usage(&texte)
                    .into_iter()
                    .filter(|u| !premiere_vue || u.instant >= horizon),
            );
        }

        lecture.evenements.sort_by_key(|e| e.instant);
        lecture.usages.sort_by_key(|u| u.instant);
        lecture
    }
}

fn touche_depuis(infos: &std::fs::Metadata, horizon: DateTime<Utc>) -> bool {
    infos
        .modified()
        .map(|instant| DateTime::<Utc>::from(instant) >= horizon)
        .unwrap_or(true)
}

/// Lit a partir d'un decalage, en s'arretant a la derniere ligne complete.
///
/// Un journal peut etre en cours d'ecriture : lire sa derniere ligne tronquee
/// la perdrait, puisque le decalage aurait deja avance.
fn morceau(chemin: &Path, depart: u64) -> Option<(String, u64)> {
    let mut fichier = std::fs::File::open(chemin).ok()?;
    fichier.seek(SeekFrom::Start(depart)).ok()?;

    let mut octets = Vec::new();
    fichier.read_to_end(&mut octets).ok()?;

    let fin = octets.iter().rposition(|o| *o == b'\n')? + 1;
    let texte = String::from_utf8_lossy(&octets[..fin]).to_string();

    Some((texte, fin as u64))
}

/// Tous les `.jsonl` sous la racine, quelle que soit leur profondeur.
fn journaux(racine: &Path) -> Vec<PathBuf> {
    let mut trouves = Vec::new();
    let mut a_visiter = vec![racine.to_path_buf()];

    while let Some(dossier) = a_visiter.pop() {
        let Ok(entrees) = std::fs::read_dir(&dossier) else {
            continue;
        };

        for entree in entrees.flatten() {
            let chemin = entree.path();
            match entree.file_type() {
                Ok(sorte) if sorte.is_dir() => a_visiter.push(chemin),
                Ok(_) if chemin.extension().is_some_and(|e| e == "jsonl") => {
                    trouves.push(chemin)
                }
                _ => {}
            }
        }
    }

    trouves.sort();
    trouves
}

/// L'emplacement des journaux de Claude Code.
pub fn dossier_par_defaut() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".claude")
        .join("projects")
}
