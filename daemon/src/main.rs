//! Point d'entree du daemon.
//!
//!   vibemap pair <code>   relie cette machine au compte qui a affiche le code
//!   vibemap               bat, tant qu'on ne l'arrete pas
//!
//! Pour l'instant il ne fait que signaler que la machine est vivante. Le scan
//! des repos, la lecture des journaux et les worktrees viennent ensuite.

use std::path::PathBuf;
use std::process::ExitCode;
use vibemap::{Config, Supabase};

const URL_PAR_DEFAUT: &str = "http://127.0.0.1:54321";

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    match arguments.first().map(String::as_str) {
        Some("pair") => appairer(arguments.get(1).map(String::as_str)).await,
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
           vibemap [config]      bat, tant qu'on ne l'arrete pas\n\n\
         Variables d'environnement :\n\
         \x20 VIBEMAP_SUPABASE_URL       racine de l'API (defaut : {URL_PAR_DEFAUT})\n\
         \x20 VIBEMAP_SUPABASE_ANON_KEY  cle publique du projet"
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

    println!(
        "vibemap surveille depuis « {} », battement toutes les {} s, \
         cartographie toutes les {} s",
        config.label, config.interval_seconds, config.scan_seconds
    );

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
                cartographier(&client, &config).await;
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
async fn cartographier(client: &Supabase, config: &Config) {
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
                Ok(_) => envoyes += 1,
                Err(erreur) => eprintln!("plan de {} non envoye : {erreur}", plan.name),
            }
        }
    }

    println!(
        "{} cartographie : {envoyes} repo(s) sur {trouves}",
        chrono::Utc::now().format("%H:%M:%S")
    );
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
