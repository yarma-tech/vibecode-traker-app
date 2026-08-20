//! Le passe des journaux, depouille sans allumer une seule couleur.
//!
//! Les memes fichiers que `journal.rs`, lus dans l'autre sens et pour une autre
//! raison. Le direct consomme les journaux au fil de l'eau pour colorer la
//! carte ; le depouillement les relit sur trente jours pour n'en tirer que deux
//! dates par zone (FR-046, FR-051).
//!
//! POURQUOI un canal a part : du passe rejoue dans le flux d'activite mentirait.
//! Bleu, ambre et rouge disent ce qui se passe maintenant ; un agent qui a
//! travaille la semaine derniere ne travaille pas maintenant (FR-047). D'ou le
//! type de retour de ce module : `DerniereTouche` n'a de champ ni pour une
//! session, ni pour un identifiant d'appel d'outil, ni pour un nom de fichier.
//! Il n'y a donc rien ici a partir de quoi un evenement d'activite pourrait etre
//! forme, meme par inadvertance.
//!
//! La monotonie est tenue cote base (`noter_dernieres_touches`) : une date ne
//! recule jamais, et ce module peut donc parcourir le passe dans le desordre
//! sans faire reculer une date fraiche (FR-088).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::journal::{self, Evenement, Nature};
use crate::{DerniereTouche, Supabase};

/// Trente jours, et pas un de plus (FR-051).
pub const FENETRE_JOURS: i64 = 30;

/// Le nom du fichier qui porte la marque de progression du depouillement.
///
/// Distinct de `journal::NOM_DES_OFFSETS`, et jamais le meme fichier : voir
/// `Marque`.
pub const NOM_DE_LA_MARQUE: &str = "depouillement.json";

/// Le point au-dela duquel on ne remonte pas.
pub fn horizon(maintenant: DateTime<Utc>) -> DateTime<Utc> {
    maintenant - Duration::days(FENETRE_JOURS)
}

/// Ou vit la marque : a cote de la configuration, comme la position du direct.
pub fn chemin_de_la_marque(config: &Path) -> PathBuf {
    config.with_file_name(NOM_DE_LA_MARQUE)
}

/// Ce qu'un depot recolte d'un depouillement : ses zones et leurs deux dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotTouches {
    pub repo_id: String,
    pub touches: Vec<DerniereTouche>,
}

/// Ce qu'un depouillement a fait, tel que les reglages le liront.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resume {
    /// Journaux trouves sous la racine.
    pub journaux_total: usize,
    /// Journaux traites, y compris ceux que la borne des trente jours ecarte.
    pub journaux_depouilles: usize,
    /// Zones auxquelles une date a ete proposee.
    pub zones_notees: usize,
    /// Envois qui n'ont pas abouti. Ils n'arretent pas le depouillement.
    pub en_defaut: usize,
}

/// Ramene des appels d'outils du passe a deux dates par zone, depot par depot.
///
/// La borne des trente jours se tient sur l'horodatage de chaque ligne, pas sur
/// celui du fichier : un journal ecrit hier peut porter des lignes d'il y a deux
/// mois.
///
/// Un evenement dont le dossier courant ne descend d'aucune racine connue est
/// ignore sans erreur (FR-066) : le depot a pu etre efface du disque, ou n'avoir
/// jamais ete cartographie. C'est aussi la seule barriere entre le disque entier
/// et ce qui part vers Supabase, exactement comme dans `journal::rattacher`.
pub fn touches(
    evenements: &[Evenement],
    repos: &BTreeMap<PathBuf, String>,
    horizon: DateTime<Utc>,
) -> Vec<LotTouches> {
    let mut par_depot: BTreeMap<String, BTreeMap<String, DerniereTouche>> = BTreeMap::new();

    for evenement in evenements {
        if evenement.instant < horizon {
            continue;
        }

        let Some((racine, repo_id)) = journal::repo_de(&evenement.cwd, repos) else {
            continue;
        };
        let Some((module_path, _)) =
            journal::localiser(&evenement.chemin, evenement.dossier, racine)
        else {
            continue;
        };

        let zone = par_depot
            .entry(repo_id.clone())
            .or_default()
            .entry(module_path.clone())
            .or_insert_with(|| DerniereTouche {
                chemin: module_path,
                derniere_ecriture: None,
                derniere_lecture: None,
            });

        // Une ecriture ne renseigne que la date d'ecriture, une lecture que
        // celle de lecture : l'une ne chasse jamais l'autre (FR-039, FR-040).
        let date = match evenement.nature {
            Nature::Ecriture => &mut zone.derniere_ecriture,
            Nature::Lecture => &mut zone.derniere_lecture,
        };
        *date = Some(match *date {
            Some(connue) => connue.max(evenement.instant),
            None => evenement.instant,
        });
    }

    par_depot
        .into_iter()
        .map(|(repo_id, zones)| LotTouches {
            repo_id,
            touches: zones.into_values().collect(),
        })
        .collect()
}

/// Les touches que porte un journal entier, et rien d'autre.
///
/// Fonction pure sur du texte, sans reseau : c'est ce qui la rend verifiable,
/// comme `journal::lire` dont elle est la cousine.
pub fn touches_du_journal(
    contenu: &str,
    repos: &BTreeMap<PathBuf, String>,
    horizon: DateTime<Utc>,
) -> Vec<LotTouches> {
    touches(&journal::lire(contenu), repos, horizon)
}

/// Depouille tous les journaux sous la racine et pose leurs dates.
///
/// Rien d'autre n'est ecrit : ni evenement d'activite, ni session, ni jeton.
/// Un journal illisible, un depot inconnu, un envoi refuse - rien de tout cela
/// n'arrete le passage ; le resume dit ce qui a manque et le depouillement va a
/// son terme (FR-066).
///
/// Sans journal sous la racine, le resume est vide et la fonction rend la main
/// aussitot (FR-065).
pub async fn depouiller(
    client: &Supabase,
    racine_journaux: &Path,
    repos: &BTreeMap<PathBuf, String>,
    horizon: DateTime<Utc>,
) -> Resume {
    let journaux = journal::journaux(racine_journaux);
    let mut resume = Resume {
        journaux_total: journaux.len(),
        ..Resume::default()
    };

    for chemin in journaux {
        resume.journaux_depouilles += 1;

        // La date de modification borne le travail avant meme d'ouvrir le
        // fichier : un journal auquel personne n'a touche depuis plus de trente
        // jours ne peut porter aucune ligne dans la fenetre. Sur un poste dont
        // les journaux pesent un gigaoctet, c'est ce qui evite de tout lire.
        if !touche_depuis(&chemin, horizon) {
            continue;
        }

        // Lire depuis la boucle d'evenements la figerait : le battement et la
        // lecture vivante s'arreteraient le temps du depouillement (FR-048).
        // Chaque fichier se lit donc sur un fil a part, et l'attente rend la
        // main a la boucle entre deux journaux.
        let a_lire = chemin.clone();
        let lu = tokio::task::spawn_blocking(move || std::fs::read_to_string(&a_lire)).await;

        let Ok(Ok(contenu)) = lu else {
            // Un journal illisible - efface entre le parcours et la lecture,
            // ou pas en UTF-8 - n'est pas une panne du depouillement.
            resume.en_defaut += 1;
            continue;
        };

        for lot in touches_du_journal(&contenu, repos, horizon) {
            match client
                .pousser_dernieres_touches(&lot.repo_id, &lot.touches)
                .await
            {
                Ok(_) => resume.zones_notees += lot.touches.len(),
                Err(erreur) => {
                    resume.en_defaut += 1;
                    eprintln!("dates du passe non posees : {erreur}");
                }
            }
        }
    }

    resume
}

/// Vrai quand le fichier a ete touche depuis l'horizon, ou qu'on ne sait pas.
///
/// Dans le doute on lit : rater un journal serait pire que le lire pour rien.
fn touche_depuis(chemin: &Path, horizon: DateTime<Utc>) -> bool {
    std::fs::metadata(chemin)
        .and_then(|infos| infos.modified())
        .map(|instant| DateTime::<Utc>::from(instant) >= horizon)
        .unwrap_or(true)
}

/// La marque de progression du depouillement en arriere.
///
/// POURQUOI son propre fichier, et jamais `offsets.json` (FR-084) : la position
/// de lecture du direct avance quand le compagnon en ligne de commande tourne,
/// y compris pendant que l'application est fermee. Les confondre reviendrait a
/// tenir pour depouille ce que le direct a seulement consomme, et les dates du
/// passe ne seraient jamais rattrapees. Les deux marques avancent a leur rythme
/// et aucune n'ecrit dans l'autre.
///
/// Elle ne se pose qu'une fois le passage mene a son terme : un depouillement
/// interrompu ne laisse rien, plutot qu'une marque qui mentirait sur ce qui a
/// deja ete lu.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Marque {
    /// Fin du dernier depouillement mene a son terme.
    pub termine_a: Option<DateTime<Utc>>,
    /// Nombre de journaux depouilles lors de ce passage.
    #[serde(default)]
    pub journaux: usize,
    /// Ou ecrire. `None` : une marque ephemere, sans memoire entre deux vies.
    #[serde(skip)]
    chemin: Option<PathBuf>,
}

impl Marque {
    /// Relit la marque ecrite sur disque, ou en rend une vierge.
    ///
    /// Un fichier absent, illisible ou a moitie ecrit ne fait pas paniquer :
    /// une marque perdue coute un depouillement de plus, jamais un demarrage
    /// refuse.
    pub fn charger(chemin: &Path) -> Self {
        let mut marque = std::fs::read(chemin)
            .ok()
            .and_then(|octets| serde_json::from_slice::<Marque>(&octets).ok())
            .unwrap_or_default();
        marque.chemin = Some(chemin.to_path_buf());
        marque
    }

    /// Note qu'un depouillement vient d'aboutir.
    pub fn poser(&mut self, quand: DateTime<Utc>, journaux: usize) {
        self.termine_a = Some(quand);
        self.journaux = journaux;
    }

    /// Ecrit la marque, sans jamais laisser un fichier a moitie ecrit.
    ///
    /// Meme precaution que `journal::Suivi::enregistrer` : on ecrit a cote puis
    /// on renomme, le `rename` etant atomique sur un systeme POSIX. Une marque
    /// sans chemin ne fait rien.
    pub fn enregistrer(&self) -> std::io::Result<()> {
        let Some(chemin) = &self.chemin else {
            return Ok(());
        };

        if let Some(dossier) = chemin.parent() {
            std::fs::create_dir_all(dossier)?;
        }

        let contenu = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let temporaire = chemin.with_extension("json.tmp");
        std::fs::write(&temporaire, &contenu)?;
        std::fs::rename(&temporaire, chemin)
    }
}
