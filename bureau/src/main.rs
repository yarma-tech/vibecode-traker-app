// La fenetre de Vibe Map.
//
// Elle n'a qu'une origine : celle du service d'interface servi depuis la
// machine. Pas de barre d'adresse - une fenetre Tauri n'en a pas -, et aucune
// navigation ailleurs : `on_navigation` refuse tout ce qui n'est ni la page
// locale d'attente, ni cette origine (FR-002, FR-052).
//
// Elle se rouvre la ou on l'a laissee (FR-005) : sa geometrie se note a chaque
// deplacement et s'ecrit en quittant.
//
// Le pont des commandes locales lui est ouvert sur ses DEUX pages : la page
// d'attente que l'application sert elle-meme, et l'interface servie sur
// `127.0.0.1:51789` - voir `capabilities/default.json`. Ce n'est pas une
// ouverture au dehors : cette origine est celle de l'application, qui sert sa
// propre interface depuis la machine, et c'est pour cela que cette architecture
// a ete choisie plutot qu'une page distante. Sans cette ouverture, l'interface
// ne pourrait rien dire des faits du poste - dossiers surveilles, lecteur
// arrete -, que la base ne porte pas.
//
// L'autorisation GitHub, elle, sort de la fenetre et y revient (FR-071,
// FR-072) : GitHub refuse les vues embarquees, et `on_navigation` refuserait
// de toute facon d'emmener la fenetre ailleurs. C'est donc l'application qui
// ouvre la page d'autorisation dans le navigateur du systeme, et qui reprend
// le premier plan quand la session s'ouvre - la fenetre ne se deplace jamais.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use bureau::autorisation;
use bureau::dossiers::{Ajout, Reautorisation, Retrait, Surveillance};
use bureau::geometrie::{self, Geometrie, Position, HAUTEUR_MINIMALE, LARGEUR_MINIMALE};
use bureau::lecteur::{CasEchec, EchecLecteur, EtatLecteur, LecteurEmbarque};
use bureau::machine::{self, EtatMachine};
use bureau::service::Service;
use bureau::sonde::{url_de_la_fenetre, PORT_INTERFACE};
use bureau::Echec;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, RunEvent, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_dialog::DialogExt;

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
    /// Ce que le dernier demarrage a donne. Il ne fait pas foi tant qu'il y a
    /// un lecteur a regarder : celui-la dit lui-meme s'il tourne encore. Il ne
    /// sert que quand l'application n'en tient aucun - pendant un demarrage, ou
    /// apres un demarrage refuse.
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
            etat_lecteur: Mutex::new(EtatLecteur::EnDemarrage),
            geometrie: Mutex::new(None),
            etat_de_la_fenetre: Mutex::new(None),
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(EtatCourant::default())
        // Le selecteur de fichiers du systeme (FR-031). Il n'est ouvert que
        // depuis le Rust, par la commande `ajouter_un_dossier` : la fenetre n'a
        // aucune permission de l'ouvrir elle-meme, et ne recoit donc jamais de
        // chemin a nous rendre.
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            etat_de_l_interface,
            etat_du_lecteur,
            relancer_le_lecteur,
            arreter_le_lecteur,
            dossiers_surveilles,
            contexte_du_poste,
            ajouter_un_dossier,
            retirer_un_dossier,
            redemander_l_autorisation,
            ouvrir_l_autorisation,
            declarer_la_machine,
            revenir_au_premier_plan,
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
            relancer_le_lecteur_en_arriere_plan(app.handle().clone());

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

/// Demarre le lecteur du poste, ou le remet en marche, et retient ce qu'il en
/// est.
///
/// En arriere-plan, pour la meme raison que le service : la lecture du jeton au
/// trousseau peut ouvrir une boite de dialogue du systeme, et une application
/// qui cesse de repondre pendant son demarrage se fait tuer.
fn relancer_le_lecteur_en_arriere_plan(app: AppHandle) {
    remettre_le_lecteur_en_marche(app, bureau::lecteur::relancer_le_lecteur);
}

/// Fait repartir le lecteur pour qu'il relise sa configuration, meme s'il
/// tourne (FR-033, FR-034).
///
/// Appele apres un ajout de dossier, et seulement la : c'est le depart du
/// lecteur qui relit la liste des dossiers surveilles, et sa premiere
/// cartographie qui parcourt le dossier ajoute sans attendre l'intervalle.
fn reprendre_le_lecteur_en_arriere_plan(app: AppHandle) {
    remettre_le_lecteur_en_marche(app, bureau::lecteur::reprendre_la_configuration);
}

/// Le fond commun des deux : le demarrage se fait en arriere-plan et l'etat
/// suit, quelle que soit la porte par laquelle on remet le lecteur en marche.
fn remettre_le_lecteur_en_marche(
    app: AppHandle,
    demarrage: fn(&mut Option<LecteurEmbarque>) -> EtatLecteur,
) {
    // Note avant de partir, et non depuis le fil : la fenetre doit voir le
    // demarrage des le clic qui l'a demande, et non au passage d'apres.
    *app.state::<EtatCourant>()
        .etat_lecteur
        .lock()
        .expect("etat du lecteur") = EtatLecteur::EnDemarrage;

    std::thread::spawn(move || {
        let etat: State<EtatCourant> = app.state();
        let mut lecteur = etat.lecteur.lock().expect("lecteur en cours");

        let resultat = demarrage(&mut lecteur);
        if let EtatLecteur::EnEchec(echec) = &resultat {
            // La fenetre dit deja ce qui cloche ; cette trace le repete la ou
            // on lance l'application au terminal pour la mettre au point.
            eprintln!("lecteur non demarre : {}", echec.raison);
        }

        // Tant que le lecteur est retenu : sans cela, la fenetre lirait un
        // demarrage encore en cours alors qu'il vient d'aboutir.
        *etat.etat_lecteur.lock().expect("etat du lecteur") = resultat;
    });
}

/// Ou en est l'ouverture. La page d'attente le demande, plutot que d'attendre
/// un signal : elle est ainsi juste des son affichage, meme si l'ouverture a
/// echoue avant qu'elle ne soit la.
#[tauri::command]
fn etat_de_l_interface(etat: State<EtatCourant>) -> Ouverture {
    etat.ouverture.lock().expect("etat de l'ouverture").clone()
}

/// Ou en est le lecteur, a l'instant ou on le demande. Un fait du poste, que la
/// base ne porte pas et ne portera pas : elle ne sait rien d'un verrou pris sur
/// cette machine.
///
/// Cette commande ne recoit rien et ne lit aucun fichier : elle rend un fait
/// que l'application tient deja, ce qui est la seule nature de commande que le
/// pont accepte.
///
/// La fenetre la redemande sans cesse ; c'est ainsi qu'un lecteur qui cesse de
/// tourner se voit en une fraction de seconde, la ou FR-010 accorde dix
/// secondes.
#[tauri::command]
fn etat_du_lecteur(etat: State<EtatCourant>) -> EtatLecteur {
    let dernier = etat.etat_lecteur.lock().expect("etat du lecteur").clone();

    // Le lecteur n'est retenu que le temps d'un demarrage, qui peut attendre
    // une autorisation du trousseau. On rend alors le dernier etat connu - « en
    // demarrage » - plutot que d'attendre : la fenetre ne doit pas se figer
    // derriere une boite de dialogue du systeme.
    let Ok(lecteur) = etat.lecteur.try_lock() else {
        return dernier;
    };

    bureau::lecteur::etat_a_montrer(
        lecteur.as_ref().map(LecteurEmbarque::tourne_encore),
        &dernier,
    )
}

/// « Lister les dossiers surveilles » : pour chacun, son chemin, son compte de
/// depots, sa lisibilite et son etat d'autorisation (FR-027 a FR-029).
///
/// Des faits du poste, que la base ne porte pas et ne portera pas (FR-059) : le
/// catalogue sait quels depots ont ete cartographies, jamais depuis quels
/// dossiers ni ce que ces dossiers sont devenus depuis. Ils ne sortent pas de
/// la machine : cette commande repond dans la fenetre et nulle part ailleurs.
///
/// Elle ne recoit rien - pas de chemin, pas de filtre. C'est la borne du pont :
/// un geste nomme, jamais un acces au disque (PRD-002, decisions
/// d'implementation).
#[tauri::command]
fn dossiers_surveilles() -> Surveillance {
    bureau::dossiers::dossiers_du_poste()
}

/// « Lire le contexte » : ce que le poste sait de lui-meme - la version qu'il
/// execute, le nom de cette machine, et ou en est le depouillement du passe
/// (FR-050, FR-065, FR-068).
///
/// Des faits du poste, comme les dossiers surveilles : la base ne porte ni le
/// numero de version de l'application qui tourne ici, ni l'avancement d'un
/// depouillement qui n'a lieu que sur cette machine (FR-059).
///
/// Elle ne recoit rien et ne rend qu'un constat. L'ecran la redemande sans
/// cesse ; c'est ainsi que l'avancement du depouillement se voit bouger sans
/// recharger la page.
#[tauri::command]
fn contexte_du_poste() -> bureau::contexte::Contexte {
    bureau::contexte::contexte_du_poste()
}

/// « Ajouter un dossier » : ouvre le selecteur du systeme, inscrit le dossier
/// choisi dans la liste des dossiers surveilles, et fait repartir le lecteur
/// pour qu'il le prenne en compte tout de suite (FR-031, FR-033, FR-034).
///
/// La borne du pont tient : la fenetre ne fournit AUCUN chemin. Elle demande un
/// geste, et c'est l'application qui va demander a l'utilisateur, dans le
/// selecteur du systeme, quel dossier il designe. Une page qui appellerait
/// cette commande n'obtiendrait donc pas d'acces au disque : elle ferait
/// apparaitre une fenetre de choix devant l'utilisateur, et rien d'autre.
///
/// Elle rend la liste entiere plutot que le seul dossier ajoute, pour que
/// l'ecran se remette a jour d'un seul tenant - et le fichier de configuration
/// n'a jamais eu a etre ouvert (FR-036).
#[tauri::command]
async fn ajouter_un_dossier(app: AppHandle) -> Ajout {
    let Some(choisi) = choisir_un_dossier(app.clone()).await else {
        // Selecteur ferme sans choix : rien n'est ecrit, et il n'y a rien a
        // annoncer. Un geste repris n'est pas un echec.
        return Ajout::Annule;
    };

    let ajout = bureau::dossiers::ajouter_au_poste(&choisi);

    // Le lecteur repart seulement si quelque chose a ete ecrit : couper le
    // battement de la machine apres un ajout qui n'a pas eu lieu le couperait
    // pour rien.
    if let Ajout::Ajoute { .. } = ajout {
        reprendre_le_lecteur_en_arriere_plan(app);
    }

    ajout
}

/// Cesse de surveiller un dossier, sans effacer ce qu'on y a deja observe.
///
/// La fenetre fournit ici un chemin, contrairement a l'ajout - mais un chemin
/// qu'elle a recu de l'application elle-meme, et qui ne sert qu'a designer une
/// ligne de la liste. `retirer_du_poste` ne retire que ce qui y figure deja :
/// un chemin inconnu ne retire rien et le dit (FR-035).
#[tauri::command]
fn retirer_un_dossier(app: AppHandle, chemin: String) -> Retrait {
    let retrait = bureau::dossiers::retirer_du_poste(&chemin);

    // Meme regle que pour l'ajout : le lecteur ne repart que si la liste a
    // vraiment change.
    if let Retrait::Retire { .. } = retrait {
        reprendre_le_lecteur_en_arriere_plan(app);
    }

    retrait
}

/// Redemande au systeme l'autorisation de lire un dossier deja surveille.
///
/// macOS ne repose pas une question a laquelle on a repondu « non ». Le seul
/// geste qui rouvre la porte est que l'utilisateur designe lui-meme le dossier
/// dans le selecteur du systeme : c'est donc l'application qui l'ouvre, sur le
/// dossier concerne, et la lisibilite est relevee APRES le choix (FR-061).
#[tauri::command]
async fn redemander_l_autorisation(app: AppHandle, chemin: String) -> Reautorisation {
    let choisi = choisir_un_dossier(app.clone()).await;
    let rendu = bureau::dossiers::redemander_au_poste(&chemin, choisi.as_deref());

    if let Reautorisation::Accordee { .. } = rendu {
        reprendre_le_lecteur_en_arriere_plan(app);
    }

    rendu
}

/// Le dossier que l'utilisateur designe dans le selecteur du systeme, ou rien
/// s'il le referme.
///
/// Sur un fil dedie : le selecteur est modal et retient le fil qui l'ouvre
/// aussi longtemps que l'utilisateur cherche son dossier. Sur celui de la
/// fenetre, l'application cesserait de repondre et le systeme la tuerait.
async fn choisir_un_dossier(app: AppHandle) -> Option<PathBuf> {
    tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
        .await
        .ok()
        .flatten()
        .and_then(|choix| choix.into_path().ok())
}

/// Ce que « ouvrir l'autorisation » a donne.
///
/// Un refus est rendu a la fenetre plutot qu'avale : sans cela, l'ecran de
/// connexion attendrait un retour qui ne viendra jamais, devant un bouton
/// eteint et sans un mot (FR-016).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
enum OuvertureAutorisation {
    Ouverte,
    Refusee { raison: String },
}

/// « Ouvrir l'autorisation » : porte l'adresse d'autorisation dans le
/// navigateur du systeme, jamais dans la fenetre (FR-071).
///
/// La SEULE commande du pont qui recoit quelque chose, et elle ne recoit pas
/// n'importe quoi : `bureau::autorisation` dit a quoi ressemble une adresse
/// d'autorisation, et refuse tout le reste. Sans cette borne, la fenetre
/// disposerait d'un « ouvre ceci pour moi » sans limite.
///
/// Sur un fil dedie : l'ouverture passe par un programme du systeme, et
/// attendre sa reponse sur le fil de la fenetre ferait cesser l'application de
/// repondre.
#[tauri::command]
async fn ouvrir_l_autorisation(url: String) -> OuvertureAutorisation {
    if let Err(refus) = bureau::autorisation::verifier_l_adresse(&url) {
        // La fenetre dit deja ce qui cloche ; cette trace le repete la ou on
        // lance l'application au terminal pour la mettre au point.
        eprintln!("adresse d'autorisation refusee : {refus}");
        return OuvertureAutorisation::Refusee {
            raison: refus.to_string(),
        };
    }

    let ouverture =
        tauri::async_runtime::spawn_blocking(move || autorisation::ouvrir_dans_le_navigateur(&url))
            .await;

    match ouverture {
        Ok(Ok(())) => OuvertureAutorisation::Ouverte,
        Ok(Err(erreur)) => OuvertureAutorisation::Refusee {
            raison: format!("le navigateur du systeme n'a pas pu etre ouvert : {erreur}"),
        },
        Err(erreur) => OuvertureAutorisation::Refusee {
            raison: format!("l'ouverture du navigateur n'a pas abouti : {erreur}"),
        },
    }
}

/// « Declarer la machine » : ce Mac s'inscrit sur le compte de la session
/// ouverte, sans qu'aucun code n'ait ete demande (FR-017 a FR-022, FR-055).
///
/// La SECONDE commande du pont qui recoit quelque chose, et il faut dire
/// pourquoi. La fenetre est seule a detenir la session de l'utilisateur : elle
/// vit dans ses cookies, et l'application n'a aucun moyen de la lire. Elle la
/// presente donc ici, le temps d'un appel.
///
/// Ce qui NE remonte jamais en sens inverse, c'est le jeton de la machine :
/// `EtatMachine` n'a pas de champ ou le loger. Il va du reseau au trousseau du
/// systeme sans passer par la fenetre, et c'est ce qui fait que les deux
/// identites ne se melangent pas.
///
/// Elle est appelee a chaque affichage de l'accueil, et pas seulement apres une
/// connexion : c'est ainsi que l'identifiant conserve se represente a chaque
/// lancement (FR-055). Elle ne cree une machine que lorsque le poste n'en
/// conserve aucune.
///
/// Sur un fil dedie : elle parle a la base, et elle ouvre le trousseau - ce qui
/// peut faire apparaitre une boite de dialogue du systeme et durer aussi
/// longtemps qu'il faut a l'utilisateur pour y repondre.
#[tauri::command]
async fn declarer_la_machine(jeton: String) -> EtatMachine {
    let chemin = vibemap::Config::chemin_par_defaut();
    let nom = tauri::async_runtime::spawn_blocking(machine::nom_de_la_machine)
        .await
        .unwrap_or_else(|_| "machine sans nom".to_string());

    machine::assurer(
        &chemin,
        machine::adresse_de_la_base(),
        &jeton,
        &nom,
        machine::plateforme(),
    )
    .await
}

/// « Revenir au premier plan » : l'application reprend la main dans sa propre
/// fenetre (FR-072).
///
/// Appelee quand la session vient de s'ouvrir. A cet instant, c'est le
/// navigateur du systeme qui est devant : sans ce geste, la carte s'afficherait
/// derriere lui, et l'utilisateur devrait aller rechercher Vibe Map lui-meme -
/// exactement ce que FR-072 refuse.
#[tauri::command]
fn revenir_au_premier_plan(app: AppHandle) {
    let Some(fenetre) = app.get_webview_window(FENETRE) else {
        return;
    };
    // Reduite, cachee ou simplement derriere : les trois se defont, et aucune
    // ne doit empecher les autres si elle echoue.
    let _ = fenetre.unminimize();
    let _ = fenetre.show();
    let _ = fenetre.set_focus();
}

/// « Relancer le lecteur » : le remet en marche sans quitter l'application
/// (FR-010). Sans effet s'il tourne deja.
#[tauri::command]
fn relancer_le_lecteur(app: AppHandle) {
    relancer_le_lecteur_en_arriere_plan(app);
}

/// « Arreter le lecteur » : la deconnexion coupe le battement de cette machine
/// (FR-015).
///
/// POURQUOI la fenetre doit l'attendre : elle ferme la session juste apres, et
/// c'est cet appel qui rend le verrou du poste. Rendre l'etat plutot que rien
/// est ce qui lui permet de le montrer sans redemander.
///
/// Sur un fil dedie : laisser tomber le lecteur attend qu'il finisse son envoi
/// en cours, et le fil de la fenetre ne doit pas s'y suspendre.
#[tauri::command]
async fn arreter_le_lecteur(app: AppHandle) -> EtatLecteur {
    let arret = tauri::async_runtime::spawn_blocking(move || {
        let etat = app.state::<EtatCourant>();
        let resultat =
            bureau::lecteur::arreter(&mut etat.lecteur.lock().expect("lecteur en cours"));
        *etat.etat_lecteur.lock().expect("etat du lecteur") = resultat.clone();
        resultat
    })
    .await;

    // Un fil qui n'aboutit pas laisse un lecteur dont on ne sait plus rien :
    // l'annoncer vaut mieux que rendre « arrete » sans l'avoir constate.
    arret.unwrap_or_else(|erreur| {
        EtatLecteur::EnEchec(EchecLecteur {
            cas: CasEchec::Panne,
            raison: format!("l'arret du lecteur n'a pas abouti : {erreur}."),
            tenant: None,
        })
    })
}

/// « Reessayer » : reprend tout depuis le debut, sans quitter l'application.
#[tauri::command]
fn reessayer(app: AppHandle) {
    ouvrir_en_arriere_plan(app);
}
