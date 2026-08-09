//! Point d'entree du daemon.
//!
//!   vibemap pair <code>   relie cette machine au compte qui a affiche le code
//!   vibemap               bat, cartographie et suit les agents
//!   vibemap hook          poste un appel d'outil recu sur l'entree standard
//!
//! Les worktrees et le cout viennent ensuite.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use vibemap::journal::{self, Suivi};
use vibemap::{Config, Supabase};

const URL_PAR_DEFAUT: &str = "http://127.0.0.1:54321";

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    match arguments.first().map(String::as_str) {
        Some("pair") => appairer(arguments.get(1).map(String::as_str)).await,
        Some("hook") => hook().await,
        Some("--help" | "-h") => {
            aide();
            ExitCode::SUCCESS
        }
        _ => battre(arguments.first().map(PathBuf::from)).await,
    }
}

fn aide() {
    println!(
        "vibemap\n\n\
           vibemap pair <code>   relie cette machine au compte qui a affiche le code\n\
           vibemap [config]      bat, cartographie et suit les agents\n\
           vibemap hook          poste l'appel d'outil recu sur l'entree standard\n\n\
         Variables d'environnement :\n\
         \x20 VIBEMAP_SUPABASE_URL       racine de l'API (defaut : {URL_PAR_DEFAUT})\n\
         \x20 VIBEMAP_SUPABASE_ANON_KEY  cle publique du projet\n\
         \x20 VIBEMAP_TOKEN              jeton machine, a defaut du trousseau"
    );
}

/// `vibemap pair <code>` : echange le code, range le jeton, ecrit la config.
async fn appairer(code: Option<&str>) -> ExitCode {
    let Some(code) = code else {
        eprintln!("il manque le code : vibemap pair 7K4-M2Q");
        return ExitCode::FAILURE;
    };

    let url = std::env::var("VIBEMAP_SUPABASE_URL")
        .unwrap_or_else(|_| URL_PAR_DEFAUT.to_string());

    let Ok(anon_key) = std::env::var("VIBEMAP_SUPABASE_ANON_KEY") else {
        eprintln!(
            "VIBEMAP_SUPABASE_ANON_KEY manquante. C'est la cle publique du projet, \
             affichee par l'application web sur la page d'appairage."
        );
        return ExitCode::FAILURE;
    };

    let label = nom_de_la_machine();
    let plateforme = std::env::consts::OS;

    let identite =
        match vibemap::appairer(&url, &anon_key, code, &label, Some(plateforme)).await {
            Ok(identite) => identite,
            Err(erreur) => {
                eprintln!("{erreur}");
                return ExitCode::FAILURE;
            }
        };

    if let Err(erreur) = vibemap::trousseau::ranger(&identite.machine_id, &identite.token) {
        eprintln!("{erreur}");
        return ExitCode::FAILURE;
    }

    let chemin = Config::chemin_par_defaut();
    if let Err(erreur) = ecrire_config(&chemin, &url, &identite.machine_id, &identite.label) {
        eprintln!("impossible d'ecrire {} : {erreur}", chemin.display());
        return ExitCode::FAILURE;
    }

    println!(
        "« {} » est reliee. Le jeton est au trousseau, la configuration dans {}.\n\
         Lance `vibemap` pour commencer a battre.",
        identite.label,
        chemin.display()
    );
    ExitCode::SUCCESS
}

/// `vibemap` : la boucle.
async fn battre(chemin: Option<PathBuf>) -> ExitCode {
    let chemin = chemin.unwrap_or_else(Config::chemin_par_defaut);

    let config = match Config::load(&chemin) {
        Ok(config) => config,
        Err(erreur) => {
            eprintln!("{erreur}");
            return ExitCode::FAILURE;
        }
    };

    // Dit avant d'agir : l'acces au trousseau peut ouvrir une boite de dialogue
    // du systeme, et un daemon bloque sans avoir rien affiche est indiagnosable.
    // macOS redemande l'autorisation a chaque fois que le binaire change, donc
    // apres chaque recompilation pendant le developpement.
    println!("lecture du jeton au trousseau…");

    let token = match std::env::var("VIBEMAP_TOKEN") {
        // Echappatoire pour le developpement et les machines sans trousseau
        // (conteneurs, serveurs sans session graphique).
        Ok(depuis_l_environnement) => depuis_l_environnement,
        Err(_) => match vibemap::trousseau::lire(&config.machine_id) {
            Ok(token) => token,
            Err(erreur) => {
                eprintln!(
                    "{erreur}\n\
                     Si le systeme a demande une autorisation et qu'elle a ete refusee, \
                     relance apres avoir accepte, ou passe le jeton par la variable \
                     VIBEMAP_TOKEN."
                );
                return ExitCode::FAILURE;
            }
        },
    };

    let client = Supabase::new(&config.supabase_url, &token);
    let mut horloge =
        tokio::time::interval(std::time::Duration::from_secs(config.interval_seconds));
    let mut arpentage =
        tokio::time::interval(std::time::Duration::from_secs(config.scan_seconds));
    let mut veille =
        tokio::time::interval(std::time::Duration::from_secs(config.journal_seconds));
    let mut chantier =
        tokio::time::interval(std::time::Duration::from_secs(config.worktree_seconds));

    println!(
        "vibemap surveille depuis « {} », battement toutes les {} s, \
         cartographie toutes les {} s, journaux toutes les {} s, \
         worktrees toutes les {} s",
        config.label,
        config.interval_seconds,
        config.scan_seconds,
        config.journal_seconds,
        config.worktree_seconds
    );

    // La carte se dresse avant la premiere lecture des journaux : sans elle,
    // aucun evenement ne saurait a quel repo se rattacher.
    let mut carte = BTreeMap::new();
    cartographier(&client, &config, &mut carte).await;
    let mut suivi = Suivi::new();

    loop {
        tokio::select! {
            _ = horloge.tick() => {
                let instant = chrono::Utc::now();
                match client.announce(&config.machine_id, instant).await {
                    // Un battement perdu n'arrete pas le daemon : la machine
                    // apparaitra figee a l'ecran, ce qui est le comportement
                    // voulu. La file d'attente et le reessai arrivent en #10.
                    Err(erreur) => eprintln!("battement perdu : {erreur}"),
                    Ok(()) => println!("{} battement", instant.format("%H:%M:%S")),
                }
            }
            _ = arpentage.tick() => {
                cartographier(&client, &config, &mut carte).await;
            }
            _ = veille.tick() => {
                suivre(&client, &config, &carte, &mut suivi).await;
            }
            _ = chantier.tick() => {
                relever_worktrees(&client, &carte).await;
            }
            _ = tokio::signal::ctrl_c() => {
                println!("arret demande, au revoir");
                return ExitCode::SUCCESS;
            }
        }
    }
}

/// Parcourt les racines, cartographie chaque repo trouve, envoie les plans.
///
/// Un repo qui echoue n'arrete pas les autres : mieux vaut une carte partielle
/// qu'un ecran vide parce qu'un seul dossier posait probleme.
async fn cartographier(
    client: &Supabase,
    config: &Config,
    carte: &mut BTreeMap<PathBuf, String>,
) {
    let mut trouves = 0;
    let mut envoyes = 0;

    for racine in config.racines() {
        let Ok(entrees) = std::fs::read_dir(&racine) else {
            eprintln!("racine illisible, ignoree : {}", racine.display());
            continue;
        };

        for entree in entrees.flatten() {
            let chemin = entree.path();
            if !chemin.join(".git").exists() {
                continue;
            }
            trouves += 1;

            let plan = match vibemap::scanner(&chemin) {
                Ok(plan) => plan,
                Err(erreur) => {
                    eprintln!("{erreur}");
                    continue;
                }
            };

            match client.pousser_plan(&config.machine_id, &plan).await {
                Ok(repo_id) => {
                    envoyes += 1;
                    carte.insert(chemin, repo_id);
                }
                Err(erreur) => eprintln!("plan de {} non envoye : {erreur}", plan.name),
            }
        }
    }

    println!(
        "{} cartographie : {envoyes} repo(s) sur {trouves}",
        chrono::Utc::now().format("%H:%M:%S")
    );
}

/// Lit ce que les agents ont fait depuis le dernier tour, et l'envoie.
///
/// Un evenement dont le repo n'est pas encore cartographie est perdu : le
/// journal a deja avance. Un repo tout neuf perd donc au plus une periode de
/// cartographie d'activite, ce qui vaut mieux que de relire les journaux
/// depuis le debut a chaque tour.
async fn suivre(
    client: &Supabase,
    config: &Config,
    carte: &BTreeMap<PathBuf, String>,
    suivi: &mut Suivi,
) {
    let horizon = chrono::Utc::now()
        - chrono::Duration::seconds(config.journal_lookback_seconds as i64);
    let lecture = suivi.nouveaux(&config.journaux(), horizon);
    if lecture.evenements.is_empty() && lecture.usages.is_empty() {
        return;
    }

    for lot in journal::rattacher(&lecture.evenements, carte) {
        let resultat = client
            .pousser_activite(
                &config.machine_id,
                &lot.repo_id,
                lot.branche.as_deref(),
                &lot.activites,
            )
            .await;

        match resultat {
            Ok(0) => {}
            Ok(poses) => println!(
                "{} activite : {poses} appel(s)",
                chrono::Utc::now().format("%H:%M:%S")
            ),
            Err(erreur) => eprintln!("activite non envoyee : {erreur}"),
        }
    }

    for lot in journal::rattacher_usage(&lecture.usages, carte) {
        let resultat = client
            .pousser_cout(
                &config.machine_id,
                &lot.repo_id,
                lot.branche.as_deref(),
                &lot.sessions,
            )
            .await;

        match resultat {
            Ok(0) => {}
            Ok(sessions) => println!(
                "{} cout : {sessions} session(s)",
                chrono::Utc::now().format("%H:%M:%S")
            ),
            Err(erreur) => eprintln!("cout non envoye : {erreur}"),
        }
    }
}

/// Releve les worktrees de chaque repo cartographie et les pousse.
///
/// Canal a part, jamais mele a l'activite : on envoie la liste COMPLETE des
/// worktrees ouverts d'un repo, la base ferme d'elle-meme ceux qui ont disparu.
/// Un repo sans worktree envoie une liste vide, ce qui ferme les siens : c'est
/// ce qui fait disparaitre un worktree du plan quand on le supprime.
async fn relever_worktrees(client: &Supabase, carte: &BTreeMap<PathBuf, String>) {
    let mut ouverts = 0;

    for (chemin, repo_id) in carte {
        let worktrees = vibemap::worktrees(chemin);
        match client.pousser_worktrees(repo_id, &worktrees).await {
            Ok(n) => ouverts += n,
            Err(erreur) => eprintln!("worktrees non envoyes : {erreur}"),
        }
    }

    println!(
        "{} worktrees : {ouverts} ouvert(s) sur {} repo(s)",
        chrono::Utc::now().format("%H:%M:%S"),
        carte.len()
    );
}

/// `vibemap hook` : un appel d'outil, poste sans attendre le prochain tour.
///
/// Le hook est facultatif et ne doit jamais gener l'agent qui l'appelle : quoi
/// qu'il arrive, il sort en succes et ne dit rien sur la sortie standard.
async fn hook() -> ExitCode {
    let mut charge = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut charge).is_err() {
        return ExitCode::SUCCESS;
    }

    if let Err(erreur) = poster_le_hook(&charge).await {
        eprintln!("vibemap hook : {erreur}");
    }

    ExitCode::SUCCESS
}

async fn poster_le_hook(charge: &str) -> Result<(), String> {
    let Some(evenement) = journal::depuis_hook(charge) else {
        // Un outil hors correspondance, `Bash` en tete : rien a dire.
        return Ok(());
    };

    let config = Config::load(&Config::chemin_par_defaut()).map_err(|e| e.to_string())?;

    let token = match std::env::var("VIBEMAP_TOKEN") {
        Ok(depuis_l_environnement) => depuis_l_environnement,
        Err(_) => vibemap::trousseau::lire(&config.machine_id).map_err(|e| e.to_string())?,
    };

    let racine = journal::racine_git(Path::new(&evenement.cwd))
        .ok_or_else(|| format!("{} n'est pas dans un depot git", evenement.cwd))?;

    let client = Supabase::new(&config.supabase_url, &token);
    let empreinte = vibemap::empreinte(&racine);
    let Some(repo_id) = client
        .repo_par_empreinte(&config.machine_id, &empreinte)
        .await
        .map_err(|e| e.to_string())?
    else {
        // Repo pas encore cartographie : la prochaine lecture des journaux le
        // rattrapera, une fois le plan envoye.
        return Ok(());
    };

    let lot = journal::rattacher(
        std::slice::from_ref(&evenement),
        &BTreeMap::from([(racine, repo_id.clone())]),
    );

    for lot in lot {
        client
            .pousser_activite(
                &config.machine_id,
                &lot.repo_id,
                lot.branche.as_deref(),
                &lot.activites,
            )
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn nom_de_la_machine() -> String {
    std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .filter(|sortie| sortie.status.success())
        .map(|sortie| String::from_utf8_lossy(&sortie.stdout).trim().to_string())
        .filter(|nom| !nom.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "machine sans nom".to_string())
}

fn ecrire_config(
    chemin: &PathBuf,
    url: &str,
    machine_id: &str,
    label: &str,
) -> std::io::Result<()> {
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }

    // Aucun secret ici : le jeton est au trousseau.
    std::fs::write(
        chemin,
        format!(
            "supabase_url = \"{url}\"\n\
             machine_id = \"{machine_id}\"\n\
             label = \"{label}\"\n\
             interval_seconds = 30\n\
             scan_seconds = 300\n\
             roots = [\"~/Developer\"]\n"
        ),
    )
}
