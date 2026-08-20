//! Le service d'interface : le meme `web/` que le site, servi depuis la machine.
//!
//! L'interface n'est jamais recopiee ici. L'application la sert depuis la
//! boucle locale, parce que ses ecrans lisent la session dans les cookies et
//! appellent la base au moment de la requete : ce sont des pages, pas des
//! fichiers a poser dans un dossier.
//!
//! POURQUOI le `npm` du systeme : cette tranche (#57) construit la fenetre et
//! la sonde, pas le paquet `.app` - c'est l'objet de #86. Le vehicule qui
//! demarre le service tient donc en une seule fonction, `lancer`, et rien
//! d'autre dans le programme ne sait comment le service est mis en route.
//! #86 n'a qu'a remplacer le corps de cette fonction par l'executable
//! compagnon embarque dans le paquet ; la sonde, la fenetre et la page
//! d'indisponibilite n'en sauront rien.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};

use std::os::unix::process::CommandExt;

/// Ce qui empeche de meme lancer le service.
#[derive(Debug)]
pub enum ErreurService {
    /// Le dossier `web/` n'est pas la ou il devrait etre.
    InterfaceIntrouvable(PathBuf),
    /// Les dependances de l'interface n'ont jamais ete installees.
    DependancesAbsentes(PathBuf),
    /// L'interface n'a jamais ete construite : il n'y a rien a servir.
    InterfaceNonConstruite(PathBuf),
    /// `npm` n'a pas pu etre execute.
    VehiculeIndisponible(std::io::Error),
}

impl std::fmt::Display for ErreurService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
    let racine = racine_de_l_interface();
    if !racine.join("package.json").is_file() {
        return Err(ErreurService::InterfaceIntrouvable(racine));
    }
    if !racine.join("node_modules").is_dir() {
        return Err(ErreurService::DependancesAbsentes(racine));
    }
    // `BUILD_ID` n'existe qu'apres une construction : c'est ce qui distingue
    // une interface prete a servir d'un dossier de travail.
    if !racine.join(".next/BUILD_ID").is_file() {
        return Err(ErreurService::InterfaceNonConstruite(racine));
    }

    let enfant = Command::new("npm")
        // L'interface construite, pas le serveur de developpement : c'est
        // celle-la que le paquet servira (#86), et un serveur de
        // developpement refuse de tourner deux fois sur le meme dossier -
        // un `npm run dev` ouvert a cote empecherait l'application de
        // s'ouvrir.
        .args([
            "run",
            "start",
            "--",
            "-H",
            "127.0.0.1",
            "-p",
            &port.to_string(),
        ])
        .current_dir(&racine)
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

/// Ou vit l'interface.
///
/// En developpement, c'est le `web/` du depot, voisin de `bureau/`. Cette
/// resolution appartient au vehicule : elle disparait avec lui quand #86
/// embarquera le service dans le paquet.
fn racine_de_l_interface() -> PathBuf {
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
