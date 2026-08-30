//! Le verrou du poste : un seul lecteur a la fois sur une machine.
//!
//! Deux lecteurs qui tournent ensemble lisent les memes journaux et poussent
//! deux fois la meme activite. Le verrou appartient au lecteur lui-meme, pas a
//! l'un de ses vehicules : c'est ce module, commun, que le binaire en ligne de
//! commande et l'application de bureau appellent tous les deux, pour que la
//! regle ne soit ecrite qu'une fois (FR-008, FR-054, FR-081).
//!
//! POURQUOI un verrou du noyau (`flock`) et pas un simple fichier de PID : le
//! noyau relache le verrou quand le processus disparait, quelle qu'en soit la
//! maniere - arret propre, signal, `kill -9`, panique. Un fichier de PID, lui,
//! ne se retire qu'a l'arret ordinaire ou depuis un gestionnaire de signal :
//! `SIGKILL` ne laisse aucune chance a l'un ni a l'autre, et la marque restee
//! la condamnerait le poste. C'est la meme lecon que #57 a tiree du port de
//! l'interface, poussee jusqu'a n'avoir plus rien a nettoyer soi-meme.
//!
//! Le fichier reste sur le disque apres l'arret, et c'est voulu : ce n'est pas
//! lui qui verrouille, c'est le noyau. Le retirer a l'arret ouvrirait une
//! fenetre de course - un autre lecteur peut deja tenir ce fichier ouvert, et
//! se retrouverait a verrouiller un chemin que plus rien ne designe. Il ne
//! porte que la marque du dernier lecteur a l'avoir pris, de quoi le nommer
//! quand il refuse le poste a un second.
//!
//! Borne connue : la garantie ne vaut qu'entre lecteurs qui posent ce verrou.
//! Un lecteur d'une version anterieure ne pose aucune marque et ne prend aucun
//! verrou ; il est indetectable, et rien ici ne promet le contraire (FR-081).
//! De meme, `flock` suppose un systeme de fichiers local - le poste, jamais un
//! dossier personnel monte par le reseau.

use std::fs::OpenOptions;
use std::io::{Seek, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

/// Ce qu'un lecteur ecrit dans le fichier pour se nommer.
///
/// Sert au message de refus, jamais a decider : c'est le noyau qui dit si le
/// poste est pris.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Marque {
    /// Le processus qui tient le poste, tel que le Moniteur d'activite le
    /// montre.
    pub pid: i32,
    /// Le vehicule qui embarque le lecteur : « vibemap », l'application…
    pub vehicule: String,
    /// Depuis quand, en RFC 3339.
    pub depuis: String,
}

impl std::fmt::Display for Marque {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "« {} », processus {}, depuis {}",
            self.vehicule, self.pid, self.depuis
        )
    }
}

/// Qui tient le poste, pour le dire a celui qui arrive.
#[derive(Debug, Clone)]
pub enum Tenant {
    Nomme(Marque),
    /// Le verrou est tenu, mais la marque manque ou ne se relit pas : un
    /// lecteur tue entre la prise du verrou et son ecriture, ou d'un autre
    /// format. On refuse quand meme - le noyau fait autorite - en disant
    /// franchement qu'on ne sait pas le nommer.
    Anonyme,
}

impl std::fmt::Display for Tenant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tenant::Nomme(marque) => write!(f, "{marque}"),
            Tenant::Anonyme => {
                write!(f, "un lecteur qui n'a pas laisse de marque lisible")
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VerrouError {
    #[error(
        "un lecteur tourne deja sur cette machine : {tenant}. \
         Il n'en faut qu'un par poste, sans quoi la meme activite partirait en double : \
         arrete celui-la (ou ferme l'application de bureau) avant d'en lancer un second. \
         Le verrou est a {chemin}."
    )]
    DejaPris { chemin: PathBuf, tenant: Tenant },

    #[error(
        "le verrou du lecteur n'a pas pu etre pose a {chemin} : {source}. \
         Verifie que ce dossier existe et qu'il est accessible en ecriture."
    )]
    Inaccessible {
        chemin: PathBuf,
        source: std::io::Error,
    },
}

/// Le poste, tenu. Le verrou vit tant que cette valeur vit.
#[derive(Debug)]
pub struct Verrou {
    /// Le descripteur porte le verrou : sa fermeture le relache, que ce soit
    /// par ce destructeur ou parce que le noyau ferme tout d'un processus qui
    /// disparait. Rien d'autre n'est a faire a l'arret.
    _fichier: std::fs::File,
    chemin: PathBuf,
}

impl Verrou {
    /// Prend le poste pour ce vehicule, ou dit qui le tient deja.
    pub fn prendre(chemin: &Path, vehicule: &str) -> Result<Self, VerrouError> {
        let echec = |source| VerrouError::Inaccessible {
            chemin: chemin.to_path_buf(),
            source,
        };

        if let Some(dossier) = chemin.parent() {
            std::fs::create_dir_all(dossier).map_err(echec)?;
        }

        // Jamais `create_new` ni de troncature ici : le fichier survit a son
        // dernier tenant, et l'ouvrir ne prend rien. C'est `flock` qui tranche.
        let mut fichier = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(chemin)
            .map_err(echec)?;

        if unsafe { libc::flock(fichier.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let erreur = std::io::Error::last_os_error();
            if erreur.raw_os_error() == Some(libc::EWOULDBLOCK) {
                return Err(VerrouError::DejaPris {
                    chemin: chemin.to_path_buf(),
                    tenant: relire_la_marque(chemin),
                });
            }
            return Err(echec(erreur));
        }

        // Le poste est a nous : on se nomme, par-dessus la marque du tenant
        // precedent. Une marque qu'on ne saurait pas ecrire est un disque en
        // mauvais etat, pas un detail : on le dit et on relache (le
        // descripteur se ferme en sortant).
        let marque = Marque {
            pid: std::process::id() as i32,
            vehicule: vehicule.to_string(),
            depuis: chrono::Utc::now().to_rfc3339(),
        };
        let texte = toml::to_string(&marque).unwrap_or_default();

        fichier.set_len(0).map_err(echec)?;
        fichier.rewind().map_err(echec)?;
        fichier.write_all(texte.as_bytes()).map_err(echec)?;
        fichier.flush().map_err(echec)?;

        Ok(Self {
            _fichier: fichier,
            chemin: chemin.to_path_buf(),
        })
    }

    /// Ou vit le verrou : `~/.config/vibemap/lecteur.lock`.
    ///
    /// Un seul emplacement par poste, jamais derive du fichier de
    /// configuration donne en argument : deux configurations sur la meme
    /// machine ne doivent pas ouvrir deux postes.
    pub fn chemin_par_defaut() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
            });
        base.join("vibemap").join("lecteur.lock")
    }

    pub fn chemin(&self) -> &Path {
        &self.chemin
    }
}

/// Lit la marque du tenant. Toute lecture qui echoue rend `Anonyme` : le refus
/// ne depend pas de la qualite de ce fichier.
fn relire_la_marque(chemin: &Path) -> Tenant {
    std::fs::read_to_string(chemin)
        .ok()
        .and_then(|brut| toml::from_str::<Marque>(&brut).ok())
        .map(Tenant::Nomme)
        .unwrap_or(Tenant::Anonyme)
}
