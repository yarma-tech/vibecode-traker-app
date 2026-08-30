//! Point d'entree du daemon.
//!
//!   vibemap               bat, cartographie et suit les agents
//!   vibemap hook          poste un appel d'outil recu sur l'entree standard
//!
//! Ce binaire est le compagnon de l'application de bureau sur le meme Mac, et
//! non plus une porte d'entree autonome (FR-082) : il ne relie aucune machine
//! par lui-meme, il lit la configuration et le jeton que l'application a ecrits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use vibemap::journal;
use vibemap::lecteur::{Arret, Lecteur};
use vibemap::{Config, Supabase, Verrou};

const URL_PAR_DEFAUT: &str = "http://127.0.0.1:54321";

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    match arguments.first().map(String::as_str) {
        Some("pair") => pair_retire(),
        Some("hook") => hook().await,
        Some("--help" | "-h") => {
            aide();
            ExitCode::SUCCESS
        }
        Some("--version" | "-V") => {
            version();
            ExitCode::SUCCESS
        }
        _ => battre(arguments.first().map(PathBuf::from)).await,
    }
}

/// `vibemap --version` : la version du paquet, prise de `Cargo.toml`.
///
/// Une seule source de verite pour le numero, celui que la release et la formule
/// Homebrew publient. On l'affiche pour que l'utilisateur sache ce qu'il a.
fn version() {
    println!("vibemap {}", env!("CARGO_PKG_VERSION"));
}

fn aide() {
    println!(
        "vibemap\n\n\
           vibemap [config]      bat, cartographie et suit les agents\n\
           vibemap hook          poste l'appel d'outil recu sur l'entree standard\n\
           vibemap --version     affiche la version du paquet\n\n\
         Compagnon de l'application de bureau Vibe Map sur ce Mac : c'est elle qui\n\
         relie la machine et ecrit sa configuration, ce binaire ne fait que la lire.\n\n\
         Variables d'environnement :\n\
         \x20 VIBEMAP_SUPABASE_URL       racine de l'API (defaut : {URL_PAR_DEFAUT})\n\
         \x20 VIBEMAP_SUPABASE_ANON_KEY  cle publique du projet\n\
         \x20 VIBEMAP_TOKEN              jeton machine, a defaut du trousseau"
    );
}

/// L'ancienne sous-commande d'appairage, retiree (FR-082).
///
/// POURQUOI un aiguillage plutot qu'un simple retrait : sans lui, `vibemap pair
/// 7K4-M2Q` retomberait sur la boucle, qui prendrait « pair » pour un chemin de
/// configuration. L'utilisateur lirait une erreur sur un fichier introuvable au
/// lieu de la raison. Cet aiguillage ne relie rien : il nomme ce qui a change et
/// dit ou aller.
fn pair_retire() -> ExitCode {
    eprintln!(
        "`vibemap pair` n'existe plus. C'est l'application de bureau Vibe Map qui relie \
         cette machine et ecrit sa configuration : installe-la, ouvre-la sur ce Mac, \
         puis relance `vibemap`."
    );
    ExitCode::FAILURE
}

/// `vibemap` : la boucle.
///
/// Elle vit dans la bibliotheque, pas ici : l'application de bureau embarque le
/// meme lecteur, et le PRD-002 interdit d'en recopier le code. Ce qui reste au
/// binaire, c'est ce qui lui appartient - son code de sortie et Ctrl-C.
async fn battre(chemin: Option<PathBuf>) -> ExitCode {
    let chemin = chemin.unwrap_or_else(Config::chemin_par_defaut);

    // Un seul lecteur par machine (FR-008, FR-054) : le verrou est pris ici,
    // par le lecteur lui-meme, et tient jusqu'a la fin de la boucle ou jusqu'a
    // la disparition du processus - le noyau le relache alors seul.
    let lecteur = match Lecteur::preparer(&chemin, &Verrou::chemin_par_defaut(), "vibemap") {
        Ok(lecteur) => lecteur,
        Err(erreur) => {
            eprintln!("{erreur}");
            return ExitCode::FAILURE;
        }
    };

    // Ctrl-C n'a de sens que pour un vehicule de terminal : une bibliotheque
    // qui s'emparerait des signaux les prendrait aussi a l'application de
    // bureau, qui les traite autrement.
    let arret = Arret::new();
    let demande = arret.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("arret demande, au revoir");
            demande.demander();
        }
    });

    lecteur.tourner(arret).await;
    ExitCode::SUCCESS
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
    // Meme identite que celle qu'un scan calculerait pour cette racine : le
    // hook retrouve ainsi la ligne quelle que soit la machine qui a scanne en
    // dernier (issue #28).
    let identite = vibemap::identite(&racine, &vibemap::empreinte(&racine));
    let Some(repo_id) = client
        .repo_par_identite(&identite)
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
