// La fenetre de Vibe Map.
//
// Elle n'a qu'une origine : celle du service d'interface servi depuis la
// machine. Pas de barre d'adresse - une fenetre Tauri n'en a pas -, et aucune
// navigation ailleurs : `on_navigation` refuse tout ce qui n'est ni la page
// locale d'attente, ni cette origine (FR-002, FR-052).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use bureau::service::Service;
use bureau::sonde::url_de_la_fenetre;
use bureau::Echec;
use tauri::{AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};

const FENETRE: &str = "principale";

/// Ou en est l'ouverture de l'interface, telle que la page d'attente la lit.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
enum Ouverture {
    EnCours,
    Faite,
    Echouee(Echec),
}

/// Ce que l'application tient pendant qu'elle tourne : le service d'interface
/// en marche, et l'etat de son ouverture.
///
/// Le service vit ici, et non dans le fil qui l'a lance : c'est ce qui le fait
/// durer autant que l'application, et s'arreter avec elle.
struct EtatCourant {
    service: Mutex<Option<Service>>,
    ouverture: Mutex<Ouverture>,
}

impl Default for EtatCourant {
    fn default() -> Self {
        EtatCourant {
            service: Mutex::new(None),
            ouverture: Mutex::new(Ouverture::EnCours),
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(EtatCourant::default())
        .invoke_handler(tauri::generate_handler![etat_de_l_interface, reessayer])
        .setup(|app| {
            let origine = url_de_la_fenetre();
            WebviewWindowBuilder::new(app, FENETRE, WebviewUrl::App("index.html".into()))
                .title("Vibe Map")
                .inner_size(1280.0, 860.0)
                .min_inner_size(900.0, 600.0)
                .on_navigation(move |url| navigation_autorisee(url.as_str(), &origine))
                .build()?;

            // La fenetre s'ouvre tout de suite, sur sa page d'attente : le
            // service met plusieurs secondes a repondre, et attendre ici
            // laisserait l'utilisateur devant un Dock qui rebondit dans le
            // vide.
            ouvrir_en_arriere_plan(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("la fenetre de Vibe Map n'a pas pu s'ouvrir")
        .run(|app, evenement| {
            if let RunEvent::Exit = evenement {
                // Le service s'arrete avec l'application, toujours : un
                // service oublie derriere elle tiendrait le port, et le
                // lancement suivant le trouverait pris.
                app.state::<EtatCourant>()
                    .service
                    .lock()
                    .expect("service en cours")
                    .take();
            }
        });
}

/// Ce que la fenetre a le droit de charger.
///
/// Deux origines, et pas une de plus : la page locale de l'application, et le
/// service d'interface de la machine. Tout le reste - un lien vers un site,
/// une redirection - est refuse ici plutot que d'emmener la fenetre ailleurs.
fn navigation_autorisee(url: &str, origine: &str) -> bool {
    url.starts_with(origine) || url.starts_with("tauri://") || url.starts_with("http://tauri.")
}

/// Lance le service, puis charge son origine dans la fenetre.
///
/// En arriere-plan : le fil principal appartient a la fenetre, et une
/// application qui cesse de repondre pendant son demarrage se fait tuer par
/// le systeme.
fn ouvrir_en_arriere_plan(app: AppHandle) {
    std::thread::spawn(move || {
        let etat: State<EtatCourant> = app.state();
        *etat.ouverture.lock().expect("etat de l'ouverture") = Ouverture::EnCours;

        // Un essai precedent a pu laisser un service derriere lui : on
        // l'arrete avant d'en lancer un autre, sinon il tiendrait le port que
        // celui-ci reclame.
        etat.service.lock().expect("service en cours").take();

        match bureau::ouvrir_l_interface() {
            Ok(service) => {
                *etat.service.lock().expect("service en cours") = Some(service);
                *etat.ouverture.lock().expect("etat de l'ouverture") = Ouverture::Faite;
                if let (Some(fenetre), Ok(url)) =
                    (app.get_webview_window(FENETRE), url_de_la_fenetre().parse())
                {
                    let _ = fenetre.navigate(url);
                }
            }
            Err(echec) => {
                // La fenetre dit deja ce qui cloche ; cette trace le repete la
                // ou on lance l'application au terminal pour la mettre au
                // point.
                eprintln!("interface indisponible : {}", echec.detail);
                *etat.ouverture.lock().expect("etat de l'ouverture") = Ouverture::Echouee(echec);
            }
        }
    });
}

/// Ou en est l'ouverture. La page d'attente le demande, plutot que d'attendre
/// un signal : elle est ainsi juste des son affichage, meme si l'ouverture a
/// echoue avant qu'elle ne soit la.
#[tauri::command]
fn etat_de_l_interface(etat: State<EtatCourant>) -> Ouverture {
    etat.ouverture.lock().expect("etat de l'ouverture").clone()
}

/// « Reessayer » : reprend tout depuis le debut, sans quitter l'application.
#[tauri::command]
fn reessayer(app: AppHandle) {
    ouvrir_en_arriere_plan(app);
}
