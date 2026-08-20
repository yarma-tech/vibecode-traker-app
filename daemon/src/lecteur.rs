//! Le lecteur, tel qu'un vehicule l'embarque.
//!
//! Ce module tient la sequence de demarrage et la boucle du lecteur : la
//! configuration, le verrou du poste, le jeton, puis les cinq cadences qui
//! battent, cartographient, lisent les journaux, relevent les worktrees et
//! ingerent les commits.
//!
//! POURQUOI ici et pas dans `main.rs` : deux vehicules embarquent le meme
//! lecteur - le binaire en ligne de commande et l'application de bureau -, et
//! le PRD-002 interdit d'en recopier le code (decisions d'implementation, « une
//! seule implementation, partagee »). Ce qui reste au binaire, c'est ce qui lui
//! appartient en propre : ses arguments, ses codes de sortie, et Ctrl-C. Une
//! bibliotheque qui s'emparerait des signaux les prendrait aussi a
//! l'application.
//!
//! Le verrou vit dans la valeur `Lecteur` : il est pris au moment de la
//! preparer, et relache quand elle disparait - fin de la boucle, arret demande,
//! ou disparition du processus, le noyau s'en chargeant alors seul (voir
//! `verrou.rs`). Aucun vehicule n'a de nettoyage a faire.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::watch;

use crate::journal::{self, Suivi};
use crate::reprise::{Backoff, Debordement, FileAttente};
use crate::{Activite, ApiError, Config, ConfigError, SessionCout, Supabase, Verrou, VerrouError};

/// Ce qui empeche le lecteur de demarrer, dans l'ordre ou il le rencontre.
///
/// Un vehicule sans ecran l'affiche sur la sortie d'erreur ; l'application de
/// bureau le montre dans sa fenetre. C'est pourquoi chaque cas dit deja quoi
/// faire, plutot que de laisser chaque vehicule reformuler.
#[derive(Debug, thiserror::Error)]
pub enum LecteurError {
    #[error(transparent)]
    Configuration(#[from] ConfigError),

    #[error(transparent)]
    Poste(#[from] VerrouError),

    #[error(
        "{source} Si le systeme a demande une autorisation et qu'elle a ete refusee, \
         reessaie apres l'avoir accordee, ou passe le jeton par la variable VIBEMAP_TOKEN."
    )]
    Jeton {
        #[from]
        source: crate::trousseau::TrousseauError,
    },
}

impl LecteurError {
    /// Vrai quand un autre lecteur tient deja le poste.
    ///
    /// Le cas se distingue des autres parce qu'il ne se corrige pas de la meme
    /// facon - il n'y a rien a reparer, il y a un lecteur a arreter - et parce
    /// que l'application le montre autrement qu'une panne.
    pub fn poste_tenu(&self) -> bool {
        matches!(self, LecteurError::Poste(VerrouError::DejaPris { .. }))
    }
}

/// La demande d'arret, partagee entre le vehicule et la boucle.
///
/// Un canal de veille plutot qu'un drapeau : la boucle l'attend dans son
/// `select!` au meme titre qu'une cadence, et repart aussitot qu'il bascule -
/// sans reveil periodique pour aller regarder.
#[derive(Debug, Clone)]
pub struct Arret(watch::Sender<bool>);

impl Arret {
    pub fn new() -> Self {
        Arret(watch::channel(false).0)
    }

    /// Demande l'arret. Sans effet si personne n'ecoute encore : la valeur
    /// reste posee, et la boucle qui s'y abonne ensuite la lit deja basculee.
    pub fn demander(&self) {
        self.0.send_replace(true);
    }

    /// Rend la main quand l'arret est demande, tout de suite s'il l'est deja.
    pub async fn attendre(&self) {
        let mut veille = self.0.subscribe();
        let _ = veille.wait_for(|arrete| *arrete).await;
    }
}

impl Default for Arret {
    fn default() -> Self {
        Arret::new()
    }
}

/// Le lecteur pret a tourner : sa configuration, son jeton, et le poste tenu.
///
/// La preparation est separee de la boucle parce qu'un vehicule a besoin de
/// savoir tout de suite s'il demarre ou non : l'application de bureau le dit
/// dans sa fenetre avant meme d'avoir une boucle a faire tourner.
pub struct Lecteur {
    /// Le fichier de configuration. La position de lecture des journaux vit a
    /// cote de lui.
    chemin: PathBuf,
    config: Config,
    token: String,
    /// Le poste, tenu jusqu'a la disparition de cette valeur. Jamais lu :
    /// c'est sa duree de vie qui compte.
    _verrou: Verrou,
}

/// Ecrit a la main, et jamais derive : un `Lecteur` porte le jeton de la
/// machine, qui n'a rien a faire dans une trace ni dans le message d'un test
/// qui echoue.
impl std::fmt::Debug for Lecteur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lecteur")
            .field("chemin", &self.chemin)
            .field("label", &self.config.label)
            .finish_non_exhaustive()
    }
}

impl Lecteur {
    /// Prend le poste pour ce vehicule et prepare le lecteur, ou dit ce qui
    /// l'en empeche.
    ///
    /// L'ordre compte. Le verrou se prend avant le trousseau : sa demande
    /// d'autorisation ouvre une boite de dialogue du systeme, et un second
    /// lecteur n'a pas a la faire surgir pour ensuite refuser de demarrer
    /// (FR-008, FR-054).
    pub fn preparer(
        chemin: &Path,
        chemin_du_verrou: &Path,
        vehicule: &str,
    ) -> Result<Self, LecteurError> {
        let config = Config::load(chemin)?;
        let verrou = Verrou::prendre(chemin_du_verrou, vehicule)?;

        // Dit avant d'agir : l'acces au trousseau peut ouvrir une boite de
        // dialogue du systeme, et un lecteur bloque sans avoir rien affiche est
        // indiagnosable. macOS redemande l'autorisation a chaque fois que le
        // binaire change, donc apres chaque recompilation pendant le
        // developpement.
        println!("lecture du jeton au trousseau…");

        let token = match std::env::var("VIBEMAP_TOKEN") {
            // Echappatoire pour le developpement et les machines sans trousseau
            // (conteneurs, serveurs sans session graphique).
            Ok(depuis_l_environnement) => depuis_l_environnement,
            Err(_) => crate::trousseau::lire(&config.machine_id)?,
        };

        Ok(Lecteur {
            chemin: chemin.to_path_buf(),
            config,
            token,
            _verrou: verrou,
        })
    }

    /// Le nom lisible de la machine, tel que la configuration le porte.
    pub fn label(&self) -> &str {
        &self.config.label
    }

    /// La boucle, jusqu'a l'arret demande.
    ///
    /// Elle consomme le lecteur : quand elle rend la main, le verrou du poste
    /// tombe avec lui, et un autre lecteur peut demarrer.
    pub async fn tourner(self, arret: Arret) {
        let config = &self.config;
        let client = Supabase::new(&config.supabase_url, &self.token);

        let mut horloge = tokio::time::interval(Duration::from_secs(config.interval_seconds));
        let mut arpentage = tokio::time::interval(Duration::from_secs(config.scan_seconds));
        let mut veille = tokio::time::interval(Duration::from_secs(config.journal_seconds));
        let mut chantier = tokio::time::interval(Duration::from_secs(config.worktree_seconds));
        let mut commits = tokio::time::interval(Duration::from_secs(config.commit_seconds));

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
        cartographier(&client, config, &mut carte).await;

        // La position de lecture se recharge depuis le disque : un redemarrage
        // reprend exactement ou il s'etait arrete, sans recompter la fenetre de
        // rattrapage. Elle vit a cote de la configuration.
        let chemin_offsets = self.chemin.with_file_name("offsets.json");
        let mut suivi = Suivi::charger(&chemin_offsets);
        let mut tampon = Tampon::new(config.file_plafond);

        loop {
            tokio::select! {
                _ = horloge.tick() => {
                    let instant = chrono::Utc::now();
                    match client.announce(&config.machine_id, instant).await {
                        // Un battement perdu n'arrete pas le lecteur : la machine
                        // apparaitra figee a l'ecran, ce qui est le comportement
                        // voulu.
                        Err(erreur) => eprintln!("battement perdu : {erreur}"),
                        Ok(()) => println!("{} battement", instant.format("%H:%M:%S")),
                    }
                }
                _ = arpentage.tick() => {
                    cartographier(&client, config, &mut carte).await;
                }
                _ = veille.tick() => {
                    suivre(&client, config, &carte, &mut suivi, &mut tampon).await;
                }
                _ = chantier.tick() => {
                    relever_worktrees(&client, &carte).await;
                }
                _ = commits.tick() => {
                    ingerer_commits(&client, &carte).await;
                }
                _ = arret.attendre() => {
                    return;
                }
            }
        }
    }
}

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
            Envoi::Activite {
                repo_id,
                branche,
                activites,
            } => client
                .pousser_activite(machine_id, repo_id, branche.as_deref(), activites)
                .await
                .map(|_| ()),
            Envoi::Cout {
                repo_id,
                branche,
                sessions,
            } => client
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

/// Parcourt les racines, cartographie chaque repo trouve, envoie les plans.
///
/// Un repo qui echoue n'arrete pas les autres : mieux vaut une carte partielle
/// qu'un ecran vide parce qu'un seul dossier posait probleme.
async fn cartographier(client: &Supabase, config: &Config, carte: &mut BTreeMap<PathBuf, String>) {
    let mut trouves = 0;
    let mut envoyes = 0;
    let mut blocs_prd = 0;
    let mut features_prd = 0;
    let mut features_absentes_prd = 0;

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

            let plan = match crate::scanner(&chemin) {
                Ok(plan) => plan,
                Err(erreur) => {
                    eprintln!("{erreur}");
                    continue;
                }
            };

            match client.pousser_plan(&config.machine_id, &plan).await {
                Ok(repo_id) => {
                    envoyes += 1;

                    // Meme cadence que la cartographie (300 s) : un PRD ne
                    // change pas toutes les trente secondes (issue #36).
                    let resume = crate::prd::traiter(client, &chemin, &repo_id, &plan).await;
                    blocs_prd += resume.blocs_poses;
                    features_prd += resume.features_creees;
                    features_absentes_prd += resume.features_absentes;
                    for sans_feature in &resume.sans_feature_reconnue {
                        eprintln!(
                            "PRD {} ({}) : en-tete reconnu mais aucune feature n'y est identifiee",
                            sans_feature.display(),
                            plan.name
                        );
                    }
                    // Deux documents `validé` en collision sur (date, id) :
                    // seul le premier rencontre est converti (issue #37,
                    // "on corrige le document, on ne devine pas") - le
                    // second est signale ici comme `sans_feature_reconnue`
                    // l'est deja pour FR-044, jamais absorbe en silence.
                    for cle_dupliquee in &resume.cles_dupliquees {
                        eprintln!(
                            "PRD {} ({}) : sa cle (date/id) est deja portee par un autre document lu dans ce meme passage, il n'est pas converti",
                            cle_dupliquee.display(),
                            plan.name
                        );
                    }

                    carte.insert(chemin, repo_id);
                }
                Err(erreur) => eprintln!("plan de {} non envoye : {erreur}", plan.name),
            }
        }
    }

    println!(
        "{} cartographie : {envoyes} repo(s) sur {trouves}, {blocs_prd} bloc(s) d'exploration PRD, \
         {features_prd} feature(s) PRD creee(s), {features_absentes_prd} marquee(s) absente(s)",
        chrono::Utc::now().format("%H:%M:%S")
    );
}

/// Lit ce que les agents ont fait depuis le dernier tour, et l'envoie.
///
/// Le nouveau travail est d'abord mis en file, puis la file est ecoulee dans
/// l'ordre. La position de lecture n'est persistee QUE lorsque la file est vide,
/// c'est-a-dire quand tout a bien ete accepte par Supabase. Une coupure laisse
/// donc la position en arriere : au redemarrage, le lecteur relit ce qui n'etait
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
    let horizon =
        chrono::Utc::now() - chrono::Duration::seconds(config.journal_lookback_seconds as i64);
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
        let worktrees = crate::worktrees(chemin);
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
        if let Some(tete) = crate::head(racine) {
            client.poser_dernier_commit(repo_id, &tete).await?;
        }
        return Ok(0);
    };

    let nouveaux = crate::commits_depuis(racine, &dernier);
    if nouveaux.is_empty() {
        return Ok(0);
    }

    let branche = crate::branche_courante(racine);

    for commit in &nouveaux {
        client
            .ingerer_commit(repo_id, branche.as_deref(), commit)
            .await?;
    }

    let plus_recent = &nouveaux
        .last()
        .expect("la liste vient d'etre verifiee non vide")
        .sha;
    client.poser_dernier_commit(repo_id, plus_recent).await?;

    Ok(nouveaux.len())
}
