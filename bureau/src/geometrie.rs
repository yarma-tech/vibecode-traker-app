//! Ou la fenetre etait, la derniere fois qu'on l'a fermee.
//!
//! Cette geometrie ne se range pas dans la configuration du lecteur
//! (`~/.config/vibemap/config.toml`) : ce fichier-la dit ou joindre la base et
//! quels depots suivre, il se recopie d'un poste a l'autre et se relit a la
//! main. La taille d'une fenetre n'a de sens que sur l'ecran ou on l'a
//! deplacee ; elle appartient a l'etat de l'application sur ce poste, et a
//! rien d'autre (FR-005).
//!
//! Rien ici ne connait Tauri : la lecture et l'ecriture sont deux fonctions
//! sur un chemin, ce qui les rend verifiables sans ouvrir de fenetre.

use std::path::{Path, PathBuf};

/// La fenetre du premier lancement. Assez large pour que la carte se lise, et
/// assez petite pour tenir sur un portable.
pub const LARGEUR_PAR_DEFAUT: f64 = 1280.0;
pub const HAUTEUR_PAR_DEFAUT: f64 = 860.0;

/// En-dessous, la carte n'est plus lisible : c'est le plancher que la fenetre
/// impose a l'utilisateur, et donc aussi celui qu'une geometrie relue doit
/// respecter pour etre credible.
pub const LARGEUR_MINIMALE: f64 = 900.0;
pub const HAUTEUR_MINIMALE: f64 = 600.0;

/// Au-dela, une valeur ne decrit plus un ecran. Elle vient d'un fichier
/// abime, et la croire placerait la fenetre hors de vue.
const ETENDUE_MAXIMALE: f64 = 32_000.0;

/// Le coin haut-gauche de la fenetre, en coordonnees logiques.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Taille et position de la fenetre, en points logiques.
///
/// Logiques, et non physiques : le meme etat relu sur un ecran de densite
/// differente rend une fenetre de la meme taille apparente, au lieu d'une
/// fenetre deux fois plus grande ou deux fois plus petite.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Geometrie {
    pub largeur: f64,
    pub hauteur: f64,
    /// Absente tant que la fenetre n'a jamais ete deplacee : le systeme la
    /// place alors lui-meme, ce qu'il fait mieux que nous - il sait ou est
    /// l'ecran actif et ou sont les barres.
    #[serde(default)]
    pub position: Option<Position>,
}

impl Default for Geometrie {
    fn default() -> Self {
        Geometrie {
            largeur: LARGEUR_PAR_DEFAUT,
            hauteur: HAUTEUR_PAR_DEFAUT,
            position: None,
        }
    }
}

impl Geometrie {
    /// La meme geometrie, debarrassee de ce qui ne decrit pas une fenetre.
    ///
    /// Un fichier peut etre syntaxiquement juste et pourtant absurde : une
    /// hauteur negative, une largeur infinie, un coin a cent mille points de
    /// l'origine. Une taille invraisemblable renvoie au defaut entier ; une
    /// position invraisemblable est seulement oubliee, car la taille relue,
    /// elle, reste bonne a prendre.
    fn assainie(self) -> Self {
        if !plausible(self.largeur, LARGEUR_MINIMALE) || !plausible(self.hauteur, HAUTEUR_MINIMALE)
        {
            return Geometrie::default();
        }

        let position = self.position.filter(|coin| {
            coin.x.is_finite()
                && coin.y.is_finite()
                && coin.x.abs() <= ETENDUE_MAXIMALE
                && coin.y.abs() <= ETENDUE_MAXIMALE
        });

        Geometrie { position, ..self }
    }
}

fn plausible(mesure: f64, minimum: f64) -> bool {
    mesure.is_finite() && mesure >= minimum && mesure <= ETENDUE_MAXIMALE
}

/// Le fichier qui porte la geometrie, dans le dossier d'etat de l'application.
pub fn chemin(dossier_de_l_etat: &Path) -> PathBuf {
    dossier_de_l_etat.join("fenetre.json")
}

/// La geometrie ecrite au dernier passage, ou celle par defaut.
///
/// Un fichier absent, illisible, tronque ou absurde ne fait pas paniquer et
/// n'empeche jamais l'application de s'ouvrir : la fenetre reprend alors sa
/// taille de premier lancement. Une application qui refuse de demarrer parce
/// qu'elle ne sait plus ou etait sa fenetre serait pire que la fenetre mal
/// placee qu'elle evite.
pub fn lire(chemin: &Path) -> Geometrie {
    std::fs::read(chemin)
        .ok()
        .and_then(|octets| serde_json::from_slice::<Geometrie>(&octets).ok())
        .map(Geometrie::assainie)
        .unwrap_or_default()
}

/// Ecrit la geometrie, sans jamais laisser un fichier a moitie ecrit.
///
/// Meme precaution que le journal du daemon : on ecrit a cote, puis on
/// renomme, et le `rename` d'un systeme POSIX est atomique. Une application
/// tuee en pleine ecriture laisse donc l'ancienne geometrie intacte plutot
/// qu'un fichier tronque.
pub fn ecrire(chemin: &Path, geometrie: &Geometrie) -> std::io::Result<()> {
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }

    let contenu = serde_json::to_vec_pretty(geometrie).map_err(std::io::Error::other)?;

    let temporaire = chemin.with_extension("json.tmp");
    std::fs::write(&temporaire, &contenu)?;
    std::fs::rename(&temporaire, chemin)
}
