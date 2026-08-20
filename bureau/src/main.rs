// La fenetre de Vibe Map.
//
// Elle n'a qu'une origine : celle du service d'interface servi depuis la
// machine. Pas de barre d'adresse - une fenetre Tauri n'en a pas -, et aucune
// navigation ailleurs : `on_navigation` refuse tout ce qui n'est ni la page
// locale d'attente, ni cette origine (FR-002, FR-052).
//
// Elle se rouvre la ou on l'a laissee (FR-005) : sa geometrie se note a chaque
// deplacement et s'ecrit en quittant.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use bureau::geometrie::{self, Geometrie, Position, HAUTEUR_MINIMALE, LARGEUR_MINIMALE};
use bureau::lecteur::{EtatLecteur, LecteurEmbarque};
use bureau::service::Service;
use bureau::sonde::{url_de_la_fenetre, PORT_INTERFACE};
use bureau::Echec;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, RunEvent, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

const FENETRE: &str = "principale";

/// Ou en est l'ouverture de l'interface, telle que la page d'attente la lit.
///
/// `Faite` porte le port : c'est la page qui va charger la carte, une fois
/// qu'elle a aussi vu ou en est le lecteur, et elle a besoin de l'adresse pour
/// cela.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
enum Ouverture {
    EnCours,
    Faite { port: u16 },
    Echouee(Echec),
}

/// Ce que l'application tient pendant qu'elle tourne : le service d'interface
/// en marche, le lecteur du poste, l'etat de l'un et de l'autre, et la
/// derniere geometrie vue.
///
/// Le service et le lecteur vivent ici, et non dans le fil qui les a lances :
/// c'est ce qui les fait durer autant que l'application, et s'arreter avec elle.
struct EtatCourant {
    service: Mutex<Option<Service>>,
    ouverture: Mutex<Ouverture>,
    lecteur: Mutex<Option<LecteurEmbarque>>,
    etat_lecteur: Mutex<EtatLecteur>,
    /// La geometrie est tenue en memoire au fil des deplacements, et non lue
    /// au moment de quitter : quand l'application s'arrete, la fenetre est
    /// deja detruite et n'a plus de taille a donner.
    geometrie: Mutex<Option<Geometrie>>,
    /// Ou l'ecrire. `None` quand le systeme ne dit pas ou est le dossier
    /// d'etat de l'application : on se passe alors de memoire plutot que
    /// d'ecrire au hasard.
    etat_de_la_fenetre: Mutex<Option<PathBuf>>,
}

impl Default for EtatCourant {
    fn default() -> Self {
        EtatCourant {
            service: Mutex::new(None),
            ouverture: Mutex::new(Ouverture::EnCours),
            lecteur: Mutex::new(None),
            etat_lecteur: Mutex::new(EtatLecteur::Arrete),
            geometrie: Mutex::new(None),
            etat_de_la_fenetre: Mutex::new(None),
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(EtatCourant::default())
        .invoke_handler(tauri::generate_handler![
            etat_de_l_interface,
            etat_du_lecteur,
            reessayer
        ])
        .setup(|app| {
            let etat_de_la_fenetre = etat_de_la_fenetre(app.handle());
            let geometrie = etat_de_la_fenetre
                .as_deref()
                .map(geometrie::lire)
                .unwrap_or_default();
            *app.state::<EtatCourant>()
                .etat_de_la_fenetre
                .lock()
                .expect("emplacement de l'etat de la fenetre") = etat_de_la_fenetre;

            let origine = url_de_la_fenetre();
            let mut constructeur =
                WebviewWindowBuilder::new(app, FENETRE, WebviewUrl::App("index.html".into()))
                    .title("Vibe Map")
                    .inner_size(geometrie.largeur, geometrie.hauteur)
                    .min_inner_size(LARGEUR_MINIMALE, HAUTEUR_MINIMALE)
                    .on_navigation(move |url| navigation_autorisee(url.as_str(), &origine));

            // Une position absente, c'est un poste ou l'application n'a jamais
            // tourne : le systeme place alors la fenetre lui-meme.
            if let Some(coin) = geometrie.position {
                constructeur = constructeur.position(coin.x, coin.y);
            }
            let fenetre = constructeur.build()?;

            // La geometrie d'ouverture est notee tout de suite : une fenetre a
            // laquelle personne ne touche n'emet aucun evenement, et sans cela
            // le poste ne retiendrait rien de cette session - pas meme la
            // position que le systeme vient de choisir.
            noter_la_geometrie(app.handle());

            // Ensuite, chaque deplacement et chaque redimensionnement la met a
            // jour. C'est tant que la fenetre existe qu'on peut la lire : au
            // moment de quitter, elle est deja detruite.
            let suivi = app.handle().clone();
            fenetre.on_window_event(move |evenement| {
                if matches!(evenement, WindowEvent::Resized(_) | WindowEvent::Moved(_)) {
                    noter_la_geometrie(&suivi);
                }
            });

            // Le lecteur part avant le service : sa reponse tient en une
            // lecture de fichier et une prise de verrou, la ou le service met
            // plusieurs secondes a repondre. La page d'attente sait donc ou en
            // est le lecteur bien avant d'avoir une carte a afficher.
            demarrer_le_lecteur_en_arriere_plan(app.handle().clone());

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
                enregistrer_la_geometrie(app);

                // Le service s'arrete avec l'application, toujours : un
                // service oublie derriere elle tiendrait le port, et le
                // lancement suivant le trouverait pris.
                app.state::<EtatCourant>()
                    .service
                    .lock()
                    .expect("service en cours")
                    .take();

                // Le lecteur aussi (FR-007). Il tourne dans ce processus : le
                // laisser tomber ici l'arrete proprement, et une application
                // tuee par un signal l'emmene de toute facon avec elle - le
                // noyau relache alors le verrou du poste.
                let etat = app.state::<EtatCourant>();
                etat.lecteur.lock().expect("lecteur en cours").take();
                *etat.etat_lecteur.lock().expect("etat du lecteur") = EtatLecteur::Arrete;
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

/// Le fichier qui garde la geometrie, dans le dossier d'etat de l'application.
///
/// Le dossier d'etat local du poste, jamais la configuration du lecteur : un
/// `config.toml` se recopie d'une machine a l'autre et se relit a la main,
/// alors qu'une position de fenetre ne vaut que sur l'ecran ou elle a ete
/// choisie. `None` quand le systeme ne dit pas ou est ce dossier : sans
/// emplacement sur, on renonce a la memoire plutot que d'ecrire ailleurs.
fn etat_de_la_fenetre(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|dossier| geometrie::chemin(&dossier))
}

/// Retient ou en est la fenetre, sans rien ecrire sur le disque.
///
/// Rien n'est ecrit ici : un deplacement produit une rafale d'evenements, et
/// ecrire a chacun mettrait le disque a contribution pour rien.
fn noter_la_geometrie(app: &AppHandle) {
    let Some(fenetre) = app.get_webview_window(FENETRE) else {
        return;
    };
    let Some(vue) = geometrie_vue(&fenetre) else {
        return;
    };

    *app.state::<EtatCourant>()
        .geometrie
        .lock()
        .expect("geometrie de la fenetre") = Some(vue);
}

/// La geometrie qu'affiche la fenetre en ce moment, si elle en montre une.
///
/// Une fenetre reduite ne dit rien d'utile - le systeme lui prete alors une
/// taille et un coin qui n'ont pas de rapport avec ceux que l'utilisateur a
/// choisis -, et une mesure qu'on ne sait pas prendre vaut mieux oubliee que
/// devinee.
fn geometrie_vue(fenetre: &WebviewWindow) -> Option<Geometrie> {
    if fenetre.is_minimized().unwrap_or(false) {
        return None;
    }

    // En points logiques : le meme etat relu sur un ecran de densite
    // differente rend une fenetre de la meme taille apparente.
    let echelle = fenetre.scale_factor().ok()?;
    let taille: LogicalSize<f64> = fenetre.inner_size().ok()?.to_logical(echelle);
    let coin: LogicalPosition<f64> = fenetre.outer_position().ok()?.to_logical(echelle);

    Some(Geometrie {
        largeur: taille.width,
        hauteur: taille.height,
        position: Some(Position {
            x: coin.x,
            y: coin.y,
        }),
    })
}

/// Ecrit la derniere geometrie vue, au moment de quitter.
///
/// Une ecriture qui echoue ne retient pas l'application : elle est en train de
/// se fermer, et une fenetre qui rouvre a sa taille par defaut vaut mieux
/// qu'une application qui refuse de quitter.
fn enregistrer_la_geometrie(app: &AppHandle) {
    let etat = app.state::<EtatCourant>();

    let chemin = etat
        .etat_de_la_fenetre
        .lock()
        .expect("emplacement de l'etat de la fenetre")
        .clone();
    let geometrie = *etat.geometrie.lock().expect("geometrie de la fenetre");

    let (Some(chemin), Some(geometrie)) = (chemin, geometrie) else {
        return;
    };

    if let Err(erreur) = geometrie::ecrire(&chemin, &geometrie) {
        eprintln!(
            "la geometrie de la fenetre n'a pas pu etre enregistree dans {} : {erreur}",
            chemin.display()
        );
    }
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
                *etat.ouverture.lock().expect("etat de l'ouverture") = Ouverture::Faite {
                    port: PORT_INTERFACE,
                };
                // C'est la page d'attente qui charge la carte, et non
                // l'application : elle seule sait ce qu'elle a encore a dire.
                // Un lecteur qui n'a pas demarre se signale la, sur cette page,
                // avant qu'elle ne cede la place - la carte, elle, vient de
                // `web/` et ne connait pas les faits du poste.
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

/// Demarre le lecteur du poste, et retient ce qu'il en est.
///
/// En arriere-plan, pour la meme raison que le service : la lecture du jeton au
/// trousseau peut ouvrir une boite de dialogue du systeme, et une application
/// qui cesse de repondre pendant son demarrage se fait tuer.
fn demarrer_le_lecteur_en_arriere_plan(app: AppHandle) {
    std::thread::spawn(move || {
        let etat: State<EtatCourant> = app.state();

        // Un lecteur deja en marche est arrete d'abord : c'est lui qui tient le
        // verrou du poste, et le suivant se le refuserait a lui-meme.
        etat.lecteur.lock().expect("lecteur en cours").take();

        match bureau::lecteur::demarrer_le_lecteur() {
            Ok(lecteur) => {
                *etat.lecteur.lock().expect("lecteur en cours") = Some(lecteur);
                *etat.etat_lecteur.lock().expect("etat du lecteur") = EtatLecteur::EnMarche;
            }
            Err(echec) => {
                // La fenetre dit deja ce qui cloche ; cette trace le repete la
                // ou on lance l'application au terminal pour la mettre au point.
                eprintln!("lecteur non demarre : {}", echec.raison);
                *etat.etat_lecteur.lock().expect("etat du lecteur") = EtatLecteur::EnEchec(echec);
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

/// Ou en est le lecteur. Un fait du poste, que la base ne porte pas et ne
/// portera pas : elle ne sait rien d'un verrou pris sur cette machine.
///
/// Cette commande ne recoit rien et ne lit aucun fichier : elle rend un fait
/// que l'application tient deja, ce qui est la seule nature de commande que le
/// pont accepte.
#[tauri::command]
fn etat_du_lecteur(etat: State<EtatCourant>) -> EtatLecteur {
    etat.etat_lecteur.lock().expect("etat du lecteur").clone()
}

/// « Reessayer » : reprend tout depuis le debut, sans quitter l'application.
#[tauri::command]
fn reessayer(app: AppHandle) {
    ouvrir_en_arriere_plan(app);
}
