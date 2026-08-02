//! Point d'entree du daemon.
//!
//! Pour l'instant il ne fait qu'une chose : signaler que la machine est vivante.
//! Le scan des repos, la lecture des journaux et les worktrees viennent ensuite.

use std::path::PathBuf;
use std::process::ExitCode;
use vibemap::{Config, Supabase};

#[tokio::main]
async fn main() -> ExitCode {
    let chemin = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(Config::chemin_par_defaut);

    let config = match Config::load(&chemin) {
        Ok(config) => config,
        Err(erreur) => {
            eprintln!("{erreur}");
            return ExitCode::FAILURE;
        }
    };

    let client = Supabase::new(&config.supabase_url, &config.token);
    let mut horloge =
        tokio::time::interval(std::time::Duration::from_secs(config.interval_seconds));

    println!(
        "vibemap surveille depuis « {} », battement toutes les {} s",
        config.label, config.interval_seconds
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
            _ = tokio::signal::ctrl_c() => {
                println!("arret demande, au revoir");
                return ExitCode::SUCCESS;
            }
        }
    }
}
