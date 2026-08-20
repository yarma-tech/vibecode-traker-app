//! Le service d'interface : le meme `web/` que le site, servi depuis la machine.
//!
//! L'interface n'est jamais recopiee ici. L'application la sert depuis la
//! boucle locale, parce que ses ecrans lisent la session dans les cookies et
//! appellent la base au moment de la requete : ce sont des pages, pas des
//! fichiers a poser dans un dossier.
//!
//! Deux voies la servent, et une seule question les distingue : l'executable
//! est-il dans un paquet ?
//!
//! - **Le paquet** (FR-069) porte dans ses ressources la sortie autonome de
//!   l'interface et l'executable Node qui la fait tourner ;
//!   `bureau/embarquer-le-service.sh` les y depose avant l'empaquetage. Cette
//!   voie ne demande rien au poste : ni depot, ni Node, ni dependances
//!   installees. C'est la seule qui vaille sur un poste qui a telecharge
//!   l'application.
//! - **Le depot** sert `web/` par le `npm` du systeme, exactement comme le
//!   site. Cette voie n'existe que sur un poste de developpement, et elle
//!   reste : `cargo run` depuis le depot doit continuer d'ouvrir la carte sans
//!   avoir a empaqueter quoi que ce soit.
//!
//! Le vehicule tient dans `lancer`, et rien d'autre dans le programme ne sait
//! comment le service est mis en route : la sonde, la fenetre et la page
//! d'indisponibilite n'en savent rien.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};

use std::os::unix::process::CommandExt;

use crate::sonde::HOTE_INTERFACE;

/// Le nom du dossier que le paquet porte dans ses ressources.
///
/// Il est ecrit ici et dans `tauri.conf.json`, qui l'y depose ; les deux se
/// repondent, et `bureau/tests/service.rs` verifie qu'ils ne divergent pas.
const DOSSIER_EMBARQUE: &str = "service";

/// Ce qui empeche de meme lancer le service.
#[derive(Debug)]
pub enum ErreurService {
    /// L'application n'a pas su dire ou elle se trouve, donc d'ou servir
    /// l'interface.
    ExecutableIntrouvable(std::io::Error),
    /// Le paquet ne porte pas le service qu'il devrait porter.
    ServiceEmbarqueIncomplet(PathBuf),
    /// Le dossier `web/` n'est pas la ou il devrait etre.
    InterfaceIntrouvable(PathBuf),
    /// Les dependances de l'interface n'ont jamais ete installees.
    DependancesAbsentes(PathBuf),
    /// L'interface n'a jamais ete construite : il n'y a rien a servir.
    InterfaceNonConstruite(PathBuf),
    /// Le service n'a pas pu etre execute.
    VehiculeIndisponible(std::io::Error),
}

impl std::fmt::Display for ErreurService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurService::ExecutableIntrouvable(source) => write!(
                f,
                "l'application n'a pas su lire son propre emplacement : {source}. \
                 Relance-la depuis le dossier Applications."
            ),
            ErreurService::ServiceEmbarqueIncomplet(chemin) => write!(
                f,
                "l'interface embarquee est incomplete dans {}. Ce paquet est abime : \
                 telecharge l'application a nouveau.",
                chemin.display()
            ),
            ErreurService::InterfaceIntrouvable(chemin) => write!(
                f,
                "l'interface est introuvable a {}. Lance l'application depuis une copie \
                 complete du depot.",
                chemin.display()
            ),
            ErreurService::DependancesAbsentes(chemin) => write!(
                f,
                "les dependances de l'interface manquent a {}. Lance `npm install` dans \
                 `web/`, puis reessaie.",
                chemin.display()
            ),
            ErreurService::InterfaceNonConstruite(chemin) => write!(
                f,
                "l'interface n'a pas encore ete construite dans {}. Lance `npm run build` dans \
                 `web/`, puis reessaie.",
                chemin.display()
            ),
            ErreurService::VehiculeIndisponible(source) => write!(
                f,
                "le service d'interface n'a pas pu etre lance : {source}. Verifie que \
                 Node et npm sont installes, puis reessaie."
            ),
        }
    }
}

/// D'ou l'interface est servie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vehicule {
    /// Ce que le paquet embarque : le serveur autonome et son propre Node.
    Embarque { racine: PathBuf, node: PathBuf },
    /// Le `web/` du depot, servi par le `npm` du systeme.
    Depot { racine: PathBuf },
}

/// Le dossier des ressources du paquet, si l'executable en vient.
///
/// macOS range l'executable d'une application dans `Contents/MacOS`, et ce
/// qu'elle embarque dans `Contents/Resources`. Cette forme est le seul indice
/// utilise pour choisir la voie.
///
/// POURQUOI pas la presence des fichiers : un paquet dont les ressources
/// seraient abimees passerait alors pour un poste de developpement, et
/// l'application renverrait l'utilisateur vers un depot qu'il n'a jamais eu.
/// POURQUOI pas une variable d'environnement : elle se pose du mauvais cote
/// sans que rien ne le signale.
pub fn ressources_du_paquet(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    if macos.file_name()? != "MacOS" {
        return None;
    }
    let contenu = macos.parent()?;
    if contenu.file_name()? != "Contents" {
        return None;
    }
    Some(contenu.join("Resources"))
}

/// Choisit la voie a partir du seul chemin de l'executable.
///
/// Rien n'est lu sur le disque ici : le choix se decide, et ce qui manque se
/// dit ensuite, dans les mots de la voie choisie.
pub fn vehicule(executable: &Path, web_du_depot: &Path) -> Vehicule {
    match ressources_du_paquet(executable) {
        Some(ressources) => {
            let racine = ressources.join(DOSSIER_EMBARQUE);
            Vehicule::Embarque {
                node: racine.join("node"),
                racine,
            }
        }
        None => Vehicule::Depot {
            racine: web_du_depot.to_path_buf(),
        },
    }
}

/// Le service d'interface en marche. Il s'arrete avec cette valeur.
pub struct Service {
    enfant: Child,
}

/// Lance le service d'interface sur la boucle locale, au port donne.
///
/// Le port est passe explicitement, et jamais lu ailleurs : le service ecoute
/// la ou la fenetre ira chercher, sans intermediaire ou une valeur pourrait
/// diverger.
pub fn lancer(port: u16) -> Result<Service, ErreurService> {
    let executable = std::env::current_exe().map_err(ErreurService::ExecutableIntrouvable)?;
    let mut commande = preparer(&vehicule(&executable, &web_du_depot()), port)?;

    let enfant = commande
        // Le service devient le chef de son propre groupe de processus : il
        // en lance lui-meme d'autres, et c'est le groupe entier qu'il faudra
        // arreter. Un survivant garderait le port, et le lancement suivant le
        // trouverait pris.
        .process_group(0)
        .spawn()
        .map_err(ErreurService::VehiculeIndisponible)?;

    GROUPE_DU_SERVICE.store(enfant.id() as i32, Ordering::SeqCst);
    armer_l_arret_au_signal();

    Ok(Service { enfant })
}

/// La commande qui met le service en route, ou ce qui manque pour cela.
fn preparer(vehicule: &Vehicule, port: u16) -> Result<Command, ErreurService> {
    match vehicule {
        Vehicule::Embarque { racine, node } => {
            let serveur = racine.join("server.js");
            if !node.is_file() || !serveur.is_file() {
                return Err(ErreurService::ServiceEmbarqueIncomplet(racine.clone()));
            }

            let mut commande = Command::new(node);
            // Le serveur autonome ne prend pas d'arguments : son hote et son
            // port se lisent dans l'environnement, et nulle part ailleurs.
            commande
                .arg(&serveur)
                .current_dir(racine)
                .env("HOSTNAME", HOTE_INTERFACE.to_string())
                .env("PORT", port.to_string());
            Ok(commande)
        }
        Vehicule::Depot { racine } => {
            if !racine.join("package.json").is_file() {
                return Err(ErreurService::InterfaceIntrouvable(racine.clone()));
            }
            if !racine.join("node_modules").is_dir() {
                return Err(ErreurService::DependancesAbsentes(racine.clone()));
            }
            // `BUILD_ID` n'existe qu'apres une construction : c'est ce qui
            // distingue une interface prete a servir d'un dossier de travail.
            if !racine.join(".next/BUILD_ID").is_file() {
                return Err(ErreurService::InterfaceNonConstruite(racine.clone()));
            }

            let mut commande = Command::new("npm");
            // L'interface construite, pas le serveur de developpement : c'est
            // celle-la que le paquet sert, et un serveur de developpement
            // refuse de tourner deux fois sur le meme dossier - un
            // `npm run dev` ouvert a cote empecherait l'application de
            // s'ouvrir.
            commande
                .args([
                    "run",
                    "start",
                    "--",
                    "-H",
                    &HOTE_INTERFACE.to_string(),
                    "-p",
                    &port.to_string(),
                ])
                .current_dir(racine);
            Ok(commande)
        }
    }
}

/// Le groupe du service en cours, lisible depuis un gestionnaire de signal.
static GROUPE_DU_SERVICE: AtomicI32 = AtomicI32::new(0);

/// Arrete le service quand l'application est tuee par un signal.
///
/// Constate a l'usage : un `Drop` ne suffit pas. Une application tuee par un
/// signal ne deroule aucun destructeur, et le service lui survivait en tenant
/// le port - le lancement suivant le trouvait pris. Ce chemin-la est le seul
/// que la fenetre ne couvre pas d'elle-meme.
fn armer_l_arret_au_signal() {
    static UNE_FOIS: Once = Once::new();
    UNE_FOIS.call_once(|| unsafe {
        libc::signal(
            libc::SIGTERM,
            arreter_au_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            arreter_au_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGHUP,
            arreter_au_signal as *const () as libc::sighandler_t,
        );
    });
}

extern "C" fn arreter_au_signal(signal: libc::c_int) {
    let groupe = GROUPE_DU_SERVICE.load(Ordering::SeqCst);
    if groupe != 0 {
        unsafe {
            libc::kill(-groupe, libc::SIGTERM);
        }
    }
    // `_exit` et non `exit` : rien d'autre ne peut se faire sans risque
    // depuis un gestionnaire de signal.
    unsafe { libc::_exit(128 + signal) }
}

/// Ou vit l'interface dans le depot, voisine de `bureau/`.
///
/// Ce chemin est fige a la compilation : il ne designe quelque chose que sur
/// la machine qui a compile l'application. C'est exactement pourquoi il ne
/// sert que la voie de developpement.
fn web_du_depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."))
        .join("web")
}

impl Drop for Service {
    fn drop(&mut self) {
        let groupe = self.enfant.id() as i32;
        let _ = GROUPE_DU_SERVICE.compare_exchange(groupe, 0, Ordering::SeqCst, Ordering::SeqCst);
        // Le negatif vise le groupe entier, pas le seul processus lance :
        // voir `process_group` ci-dessus.
        unsafe {
            libc::kill(-groupe, libc::SIGTERM);
        }

        // On laisse le service se fermer proprement, puis on insiste : un
        // service qui traine tient le port, et le port est le seul que
        // l'application accepte.
        let echeance = Instant::now() + Duration::from_secs(3);
        loop {
            match self.enfant.try_wait() {
                Ok(Some(_)) | Err(_) => return,
                Ok(None) if Instant::now() >= echeance => break,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            }
        }

        unsafe {
            libc::kill(-groupe, libc::SIGKILL);
        }
        let _ = self.enfant.wait();
    }
}
