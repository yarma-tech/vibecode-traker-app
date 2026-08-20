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
//!
//! Le depouillement n'est ni reserve au premier lancement, ni une relecture
//! complete a chaque fois : a chaque ouverture il reprend ou il s'etait arrete
//! (FR-049, FR-078). Un seul mecanisme sert les deux cas - la marque de
//! progression -, parce que c'est la meme question dans les deux : qu'est-ce qui
//! a ete ecrit depuis la derniere fois ? Un depouillement interrompu et une
//! application restee fermee une semaine ne se distinguent pas de ce point de
//! vue.

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
///
/// Les deux nombres de FR-050 sont `journaux_depouilles` sur `journaux_total`.
/// POURQUOI le denominateur est le total sous la racine, et non le seul reliquat
/// de ce passage : l'avancement doit se lire d'une ouverture a l'autre. Un
/// passage interrompu a 120 sur 400 doit repartir de 120 sur 400, pas de 0 sur
/// 280.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resume {
    /// Journaux trouves sous la racine.
    pub journaux_total: usize,
    /// Journaux que ce passage a retenus, la marque et la borne des trente
    /// jours ayant ecarte les autres.
    pub journaux_a_depouiller: usize,
    /// Journaux depouilles, ce passage compris : l'avancement sur le total.
    pub journaux_depouilles: usize,
    /// Zones auxquelles une date a ete proposee.
    pub zones_notees: usize,
    /// Envois qui n'ont pas abouti. Ils n'arretent pas le depouillement.
    pub en_defaut: usize,
}

impl Resume {
    /// L'avancement d'ou ce passage est parti : ce qu'il n'a pas eu a relire.
    ///
    /// C'est le nombre que les reglages affichent tant que le passage n'a pas
    /// avance d'un journal, et donc ce qui rend visible qu'il reprend au lieu de
    /// recommencer (FR-049, FR-050).
    pub fn depart(&self) -> usize {
        self.journaux_total
            .saturating_sub(self.journaux_a_depouiller)
    }
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

/// A partir d'ou reprendre : la marque, jamais plus loin que l'horizon.
///
/// Une marque absente rend les trente derniers jours, et rien de plus. Une
/// marque plus vieille que l'horizon - l'application est restee fermee six
/// semaines - ne fait pas remonter au-dela : il n'y a rien a tirer d'un journal
/// dont aucune ligne ne tient dans la fenetre.
pub fn seuil(marque: Option<DateTime<Utc>>, horizon: DateTime<Utc>) -> DateTime<Utc> {
    marque.map_or(horizon, |marque| marque.max(horizon))
}

/// Les journaux qui restent a depouiller, du plus ancien ecrit au plus recent.
///
/// Chaque journal est donne avec la date de sa derniere ecriture. Un journal
/// ecrit avant le seuil a deja ete depouille par un passage precedent : il ne
/// revient pas. Un journal ecrit depuis revient, meme s'il a deja ete lu -
/// c'est ce qui rattrape ce qu'un agent au terminal y a ajoute pendant que
/// l'application etait fermee (FR-078).
///
/// POURQUOI cet ordre : la marque avance au fil du passage, et elle ne peut le
/// faire que si ce qui reste est toujours plus recent que ce qui est fait.
/// Trier par date d'ecriture est ce qui rend la reprise exacte.
pub fn a_depouiller(
    journaux: &[(PathBuf, DateTime<Utc>)],
    marque: Option<DateTime<Utc>>,
    horizon: DateTime<Utc>,
) -> Vec<(PathBuf, DateTime<Utc>)> {
    let seuil = seuil(marque, horizon);

    let mut restants: Vec<(PathBuf, DateTime<Utc>)> = journaux
        .iter()
        .filter(|(_, ecrit_a)| *ecrit_a >= seuil)
        .cloned()
        .collect();

    // Le chemin departage deux journaux ecrits au meme instant : sans lui,
    // l'ordre dependrait du systeme de fichiers.
    restants.sort_by(|(chemin_a, a), (chemin_b, b)| a.cmp(b).then_with(|| chemin_a.cmp(chemin_b)));
    restants
}

/// Tous les journaux sous la racine, avec la date de leur derniere ecriture.
///
/// Un journal dont on ne sait rien passe pour ecrit a l'instant : dans le doute
/// on le lit, rater un journal serait pire que le lire pour rien.
fn journaux_dates(racine: &Path, dans_le_doute: DateTime<Utc>) -> Vec<(PathBuf, DateTime<Utc>)> {
    journal::journaux(racine)
        .into_iter()
        .map(|chemin| {
            let ecrit_a = std::fs::metadata(&chemin)
                .and_then(|infos| infos.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or(dans_le_doute);
            (chemin, ecrit_a)
        })
        .collect()
}

/// Depouille les journaux ecrits depuis la marque et pose leurs dates.
///
/// Rien d'autre n'est ecrit : ni evenement d'activite, ni session, ni jeton.
/// Un journal illisible, un depot inconnu, un envoi refuse - rien de tout cela
/// n'arrete le passage ; le resume dit ce qui a manque et le depouillement va a
/// son terme (FR-066).
///
/// Sans journal sous la racine, le resume est vide et la fonction rend la main
/// aussitot (FR-065).
///
/// La marque avance journal par journal, et non a la fin : s'arreter en cours de
/// route ne coute alors que le journal en cours, jamais le passage entier
/// (FR-049). Une marque sans chemin ne garde rien entre deux vies, ce qui donne
/// a chaque appel un depouillement complet - c'est ce dont les tests purs ont
/// besoin.
pub async fn depouiller(
    client: &Supabase,
    racine_journaux: &Path,
    repos: &BTreeMap<PathBuf, String>,
    horizon: DateTime<Utc>,
    marque: &mut Marque,
) -> Resume {
    // Releve avant de lire quoi que ce soit : c'est jusqu'ici, et pas jusqu'a la
    // fin du passage, que le depouillement pourra se dire complet. Un journal
    // grossi pendant qu'on lisait ses voisins doit revenir au passage suivant.
    let debut = Utc::now();
    let journaux = journaux_dates(racine_journaux, debut);
    let restants = a_depouiller(&journaux, marque.depouille_jusqu_a, horizon);

    let mut resume = Resume {
        journaux_total: journaux.len(),
        journaux_a_depouiller: restants.len(),
        // Ce que ce passage ne relit pas est deja depouille : un passage
        // precedent l'a lu, ou il dort au-dela des trente jours et ne peut rien
        // porter dans la fenetre. Sur un poste dont les journaux pesent un
        // gigaoctet, c'est ce qui evite de tout relire a chaque ouverture.
        journaux_depouilles: journaux.len() - restants.len(),
        ..Resume::default()
    };

    for (rang, (chemin, _)) in restants.iter().enumerate() {
        // Lire depuis la boucle d'evenements la figerait : le battement et la
        // lecture vivante s'arreteraient le temps du depouillement (FR-048).
        // Chaque fichier se lit donc sur un fil a part, et l'attente rend la
        // main a la boucle entre deux journaux.
        let a_lire = chemin.clone();
        let lu = tokio::task::spawn_blocking(move || std::fs::read_to_string(&a_lire)).await;

        match lu {
            Ok(Ok(contenu)) => {
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
            // Un journal illisible - efface entre le parcours et la lecture, ou
            // pas en UTF-8 - n'est pas une panne du depouillement. Il est tenu
            // pour depouille : le relire a chaque ouverture n'en tirerait pas
            // davantage.
            _ => resume.en_defaut += 1,
        }

        resume.journaux_depouilles += 1;

        // La marque avance jusqu'a la date du prochain journal a lire : tout ce
        // qui a ete ecrit avant lui est desormais depouille. Apres le dernier,
        // c'est le debut du passage qui fait foi.
        let atteint = restants
            .get(rang + 1)
            .map_or(debut, |(_, ecrit_a)| *ecrit_a);
        marque.progresser(atteint, resume.journaux_depouilles, resume.journaux_total);
    }

    resume
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
/// Elle porte deux choses de nature differente. `depouille_jusqu_a` est ce qui
/// commande la reprise : tout journal ecrit avant cette date a ete depouille, et
/// ne sera pas relu. Les deux autres champs ne servent qu'a dire aux reglages ou
/// en est le depouillement et quand il s'est termine (FR-050).
///
/// POURQUOI l'avancement s'ecrit ici, et pas ailleurs : c'est le seul endroit
/// que les reglages peuvent lire. Le depouillement tourne dans une tache a part,
/// et l'ecran ne lui parle pas ; il lit ce fichier a chaque fois qu'il pose la
/// question, comme la liste des dossiers se releve a chaque demande. Ce qui y
/// est ecrit survit d'une ouverture a l'autre, ce qui est exactement ce que
/// FR-050 demande de montrer - une reprise qui repart de son avancement, jamais
/// de zero.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Marque {
    /// Fin du dernier depouillement mene a son terme.
    pub termine_a: Option<DateTime<Utc>>,
    /// Jusqu'ou les journaux ont ete depouilles, par leur date d'ecriture.
    ///
    /// Absente, le depouillement rend les trente derniers jours et rien de plus.
    #[serde(default)]
    pub depouille_jusqu_a: Option<DateTime<Utc>>,
    /// Nombre de journaux depouilles, tel que le passage en cours l'a laisse.
    ///
    /// Il ne retombe pas a zero d'un passage a l'autre : un passage qui reprend
    /// part de ce que les precedents ont deja lu (`Resume::depart`).
    #[serde(default)]
    pub journaux: usize,
    /// Nombre de journaux sous la racine a ce moment-la : le denominateur de
    /// l'avancement. Zero, une fois un passage termine, dit qu'il n'y a rien a
    /// depouiller sur ce poste (FR-065).
    #[serde(default)]
    pub total: usize,
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

    /// Note qu'un depouillement vient d'aboutir, et avec quel avancement.
    pub fn poser(&mut self, quand: DateTime<Utc>, journaux: usize, total: usize) {
        self.termine_a = Some(quand);
        self.journaux = journaux;
        self.total = total;
    }

    /// Fait avancer la marque jusqu'a cette date, et l'ecrit aussitot.
    ///
    /// POURQUOI ecrire a chaque journal plutot qu'a la fin du passage : c'est la
    /// seule chose qui rende un depouillement interrompu reprenable la ou il en
    /// etait (FR-049). Une ecriture qui echoue ne dit rien et n'arrete rien -
    /// elle coute au pire de relire ces journaux a la prochaine ouverture, et
    /// l'ecriture de fin de passage, elle, signale le defaut.
    ///
    /// Une marque ne recule jamais : rien de ce qui a ete depouille ne redevient
    /// a depouiller.
    pub fn avancer(&mut self, jusqu_a: DateTime<Utc>) {
        if self.depouille_jusqu_a.is_some_and(|deja| deja >= jusqu_a) {
            return;
        }

        self.depouille_jusqu_a = Some(jusqu_a);
        let _ = self.enregistrer();
    }

    /// La meme avancee, avec l'avancement chiffre que les reglages liront
    /// (FR-050).
    ///
    /// POURQUOI les deux nombres s'ecrivent a chaque journal, et pas seulement
    /// au terme du passage : c'est ce qui rend l'avancement OBSERVABLE pendant
    /// qu'il avance. Un compte pose une seule fois, a la fin, laisserait l'ecran
    /// annoncer l'avancement du passage precedent tout le temps que dure
    /// celui-ci - et sur un poste dont les journaux pesent un gigaoctet, cela se
    /// compte en minutes.
    ///
    /// Ils s'ecrivent meme quand la date ne bouge pas : deux journaux ecrits
    /// dans la meme seconde ne feraient sinon avancer aucun compteur, et l'ecran
    /// se figerait sur un depouillement qui, lui, progresse.
    pub fn progresser(&mut self, jusqu_a: DateTime<Utc>, depouilles: usize, total: usize) {
        self.journaux = depouilles;
        self.total = total;

        // Une marque ne recule jamais : rien de ce qui a ete depouille ne
        // redevient a depouiller.
        if !self.depouille_jusqu_a.is_some_and(|deja| deja >= jusqu_a) {
            self.depouille_jusqu_a = Some(jusqu_a);
        }

        let _ = self.enregistrer();
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
