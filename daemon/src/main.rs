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
use std::time::{Duration, Instant};
use vibemap::journal::{self, Suivi};
use vibemap::reprise::{Backoff, Debordement, FileAttente};
use vibemap::{Activite, ApiError, Config, SessionCout, Supabase};

const URL_PAR_DEFAUT: &str = "http://127.0.0.1:54321";

/// Un envoi vers Supabase, mis de cote si le reseau tombe.
///
/// La file ne retient que ce qui doit vraiment survivre a une coupure : les
/// appels d'outils et la consommation. Le battement et les worktrees se
/// corrigent d'eux-memes au tour suivant, ils n'ont rien a faire ici.
enum Envoi {
    Activite {
        repo_id: String,
        branche: Option<String>,
        activites: Vec<Activite>,
    },
    Cout {
        repo_id: String,
        branche: Option<String>,
        sessions: Vec<SessionCout>,
    },
}

impl Envoi {
    async fn tenter(&self, client: &Supabase, machine_id: &str) -> Result<(), ApiError> {
        match self {
            Envoi::Activite { repo_id, branche, activites } => client
                .pousser_activite(machine_id, repo_id, branche.as_deref(), activites)
                .await
                .map(|_| ()),
            Envoi::Cout { repo_id, branche, sessions } => client
                .pousser_cout(machine_id, repo_id, branche.as_deref(), sessions)
                .await
                .map(|_| ()),
        }
    }
}

/// La file d'attente locale et sa temporisation de renvoi.
///
/// Ce qui echoue est mis de cote et retente plus tard, l'attente doublant a
/// chaque echec sans jamais depasser cinq minutes : on ne martele pas Supabase.
/// Un envoi qui part remet la temporisation a son pas de depart.
struct Tampon {
    file: FileAttente<Envoi>,
    backoff: Backoff,
    prochaine_tentative: Option<Instant>,
    enfiles: usize,
}

impl Tampon {
    fn new(plafond: usize) -> Self {
        Self {
            file: FileAttente::new(plafond),
            backoff: Backoff::new(Duration::from_secs(2), Duration::from_secs(300)),
            prochaine_tentative: None,
            enfiles: 0,
        }
    }

    /// Met un envoi en attente. Si la file deborde, le plus ancien tombe : il
    /// reste dans le journal sur disque, que le prochain redemarrage relira.
    fn mettre_de_cote(&mut self, envoi: Envoi) {
        self.enfiles += 1;
        if let Debordement::Jete(_) = self.file.pousser(envoi) {
            eprintln!(
                "file d'attente pleine : un envoi ancien est abandonne \
                 (il sera relu au prochain redemarrage)"
            );
        }
    }

    /// Tente d'ecouler la file, dans l'ordre. Rend `true` si tout est parti.
    ///
    /// Tant que la temporisation court, on ne retente rien. Au premier echec, on
    /// arme la prochaine tentative et on s'arrete : l'ordre est preserve, et
    /// Supabase n'est pas martele.
    async fn ecouler(&mut self, client: &Supabase, machine_id: &str) -> bool {
        if let Some(quand) = self.prochaine_tentative {
            if Instant::now() < quand {
                return self.file.est_vide();
            }
        }

        while !self.file.est_vide() {
            let resultat = {
                let envoi = self.file.premier().expect("la file n'est pas vide");
                envoi.tenter(client, machine_id).await
            };
            match resultat {
                Ok(()) => {
                    self.file.tirer();
                    self.backoff.reset();
                    self.prochaine_tentative = None;
                }
                Err(erreur) => {
                    let attente = self.backoff.prochain();
                    self.prochaine_tentative = Some(Instant::now() + attente);
                    eprintln!(
                        "reseau indisponible, renvoi dans {} s ({} en attente) : {erreur}",
                        attente.as_secs(),
                        self.file.len()
                    );
                    return false;
                }
            }
        }
        true
    }
}

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
           vibemap pair <code>   relie cette machine au compte qui a affiche le code\n\
           vibemap [config]      bat, cartographie et suit les agents\n\
           vibemap hook          poste l'appel d'outil recu sur l'entree standard\n\
           vibemap --version     affiche la version du paquet\n\n\
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
    let mut commits =
        tokio::time::interval(std::time::Duration::from_secs(config.commit_seconds));

    println!(
        "vibemap surveille depuis « {} », battement toutes les {} s, \
         cartographie toutes les {} s, journaux toutes les {} s, \
         worktrees toutes les {} s, commits toutes les {} s",
        config.label,
        config.interval_seconds,
        config.scan_seconds,
        config.journal_seconds,
        config.worktree_seconds,
        config.commit_seconds
    );

    // La carte se dresse avant la premiere lecture des journaux : sans elle,
    // aucun evenement ne saurait a quel repo se rattacher.
    let mut carte = BTreeMap::new();
    cartographier(&client, &config, &mut carte).await;

    // La position de lecture se recharge depuis le disque : un redemarrage
    // reprend exactement ou il s'etait arrete, sans recompter la fenetre de
    // rattrapage. Elle vit a cote de la configuration.
    let chemin_offsets = chemin.with_file_name("offsets.json");
    let mut suivi = Suivi::charger(&chemin_offsets);
    let mut tampon = Tampon::new(config.file_plafond);

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
                suivre(&client, &config, &carte, &mut suivi, &mut tampon).await;
            }
            _ = chantier.tick() => {
                relever_worktrees(&client, &carte).await;
            }
            _ = commits.tick() => {
                ingerer_commits(&client, &carte).await;
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
/// Le nouveau travail est d'abord mis en file, puis la file est ecoulee dans
/// l'ordre. La position de lecture n'est persistee QUE lorsque la file est vide,
/// c'est-a-dire quand tout a bien ete accepte par Supabase. Une coupure laisse
/// donc la position en arriere : au redemarrage, le daemon relit ce qui n'etait
/// pas parti, et l'idempotence (unicite des evenements, cle des jetons) absorbe
/// les doublons. Aucun evenement perdu, aucun jeton recompte.
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
    tampon: &mut Tampon,
) {
    let horizon = chrono::Utc::now()
        - chrono::Duration::seconds(config.journal_lookback_seconds as i64);
    let lecture = suivi.nouveaux(&config.journaux(), horizon);

    let retard = !tampon.file.est_vide();
    tampon.enfiles = 0;

    for lot in journal::rattacher(&lecture.evenements, carte) {
        tampon.mettre_de_cote(Envoi::Activite {
            repo_id: lot.repo_id,
            branche: lot.branche,
            activites: lot.activites,
        });
    }
    for lot in journal::rattacher_usage(&lecture.usages, carte) {
        tampon.mettre_de_cote(Envoi::Cout {
            repo_id: lot.repo_id,
            branche: lot.branche,
            sessions: lot.sessions,
        });
    }

    let du_travail = tampon.enfiles > 0 || retard;
    let vide = tampon.ecouler(client, &config.machine_id).await;

    if du_travail && vide {
        println!("{} activite ecoulee", chrono::Utc::now().format("%H:%M:%S"));
        // Tout est parti : on peut avancer la position de lecture sur disque.
        if let Err(erreur) = suivi.enregistrer() {
            eprintln!("position de lecture non ecrite : {erreur}");
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

/// Ferme les travaux que les commits locaux nomment, par depot cartographie.
///
/// Cadence propre de 30 s (conception §5.2), entre les journaux (2 s) et la
/// cartographie (300 s) : celle-ci ne pouvait pas tenir la promesse d'une
/// fermeture en moins d'une minute (PRD FR-029). Un depot qui echoue
/// n'arrete pas les autres, comme pour la cartographie et les worktrees.
async fn ingerer_commits(client: &Supabase, carte: &BTreeMap<PathBuf, String>) {
    let mut ingeres = 0;
    let mut en_defaut = 0;

    for (chemin, repo_id) in carte {
        match traiter_commits_du_repo(client, chemin, repo_id).await {
            Ok(n) => ingeres += n,
            Err(erreur) => {
                en_defaut += 1;
                eprintln!("commits de {} non ingeres : {erreur}", chemin.display());
            }
        }
    }

    println!(
        "{} commits : {ingeres} ingere(s), {en_defaut} depot(s) en defaut sur {}",
        chrono::Utc::now().format("%H:%M:%S"),
        carte.len()
    );
}

/// Rejoue les commits nouveaux d'un seul depot, du plus ancien au plus
/// recent, et avance sa position de lecture.
///
/// Rien n'est avance tant que tout n'est pas parti : une coupure en cours de
/// route laisse `last_commit_sha` en arriere, et le prochain tour rejoue la
/// plage entiere depuis ce point. C'est sans risque : l'insertion est
/// idempotente (`unique (repo_id, sha)`) et ne referme jamais ce qui l'est
/// deja (seule une insertion qui cree reellement une ligne appelle la
/// fermeture, cote base).
async fn traiter_commits_du_repo(
    client: &Supabase,
    racine: &Path,
    repo_id: &str,
) -> Result<usize, ApiError> {
    let Some(dernier) = client.dernier_commit_connu(repo_id).await? else {
        // Premier passage sur ce depot : on pose HEAD sans rien fermer, le
        // passe n'est jamais rejoue (PRD, risques).
        if let Some(tete) = vibemap::head(racine) {
            client.poser_dernier_commit(repo_id, &tete).await?;
        }
        return Ok(0);
    };

    let nouveaux = vibemap::commits_depuis(racine, &dernier);
    if nouveaux.is_empty() {
        return Ok(0);
    }

    let branche = vibemap::branche_courante(racine);

    for commit in &nouveaux {
        client.ingerer_commit(repo_id, branche.as_deref(), commit).await?;
    }

    let plus_recent = &nouveaux
        .last()
        .expect("la liste vient d'etre verifiee non vide")
        .sha;
    client.poser_dernier_commit(repo_id, plus_recent).await?;

    Ok(nouveaux.len())
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
