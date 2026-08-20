//! Ce que le poste sait de lui-meme : sa version, son nom, ou en est son
//! depouillement (issues #81, #85 - FR-050, FR-065, FR-068).
//!
//! Des faits du POSTE, au meme titre que les dossiers surveilles, et ils
//! empruntent donc la meme porte : le pont des commandes locales, jamais la base
//! (FR-059). La base ne porte ni le numero de version de l'application qui
//! tourne ici, ni l'avancement d'un depouillement qui n'a lieu que sur cette
//! machine ; elle ne le portera pas davantage demain.
//!
//! POURQUOI ces faits se relevent a chaque demande, plutot que de se noter au
//! demarrage : l'avancement du depouillement BOUGE pendant qu'on le regarde.
//! L'ecran redemande, et chaque reponse est celle de l'instant - c'est la meme
//! discipline que `dossiers.rs`, ou la question posee est « qu'en est-il
//! maintenant ».
//!
//! Le depouillement, lui, ne se laisse pas interroger directement : il tourne
//! dans une tache a part du lecteur, et rien ne relie cette tache a la fenetre.
//! Ce qu'on lit ici, c'est sa marque de progression sur le disque
//! (`depouillement.json`), que le lecteur avance journal par journal. Elle
//! survit aux fermetures de l'application, et c'est precisement ce qui rend
//! visible qu'un depouillement REPREND au lieu de recommencer : au premier
//! regard d'une nouvelle ouverture, l'avancement est deja celui d'avant, jamais
//! zero (FR-049, FR-050, FR-078).

use std::path::Path;

use vibemap::depouillement::{chemin_de_la_marque, Marque};
use vibemap::Config;

use crate::version::VERSION;

/// Ou en est le depouillement des trente derniers jours de journaux (FR-050).
///
/// Quatre etats, et aucun ne se confond avec un autre - c'est tout l'objet de ce
/// type. « Rien a depouiller » n'est pas « zero sur zero » (FR-065), et « jamais
/// commence » n'est pas « termine » : les quatre s'ecrivent autrement a l'ecran
/// parce qu'ils ne disent pas la meme chose de ce qui va se passer ensuite.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum Depouillement {
    /// Aucun passage n'a laisse de trace : le lecteur n'a pas encore ouvert son
    /// premier journal sur ce poste.
    Jamais,
    /// Un passage a abouti sans trouver un seul journal a depouiller (FR-065).
    ///
    /// Le cas du Mac ou Claude Code n'a jamais tourne. Il se dit tout de suite,
    /// et il se dit AUTREMENT qu'un avancement : « 0 sur 0 » se lirait comme un
    /// depouillement qui n'avance pas.
    RienADepouiller,
    /// Les deux nombres de FR-050 : les journaux depouilles sur le total sous la
    /// racine.
    EnCours { journaux: usize, total: usize },
    /// Le dernier passage est alle a son terme, a cette heure-la.
    Termine {
        /// En RFC 3339 : c'est la fenetre qui l'ecrit dans le fuseau du lecteur.
        quand: String,
        journaux: usize,
    },
}

/// Ce que la commande de pont « lire le contexte » rend.
///
/// Une seule reponse pour les trois faits, et non trois commandes : ils
/// s'affichent ensemble, au meme endroit de l'ecran, et trois appels separes
/// pourraient tomber de part et d'autre d'un depouillement qui s'acheve et se
/// contredire.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Contexte {
    /// Le numero que l'application porte, fixe a la compilation (FR-068).
    pub version: &'static str,
    /// Le nom de cette machine, tel que la liste des machines le montre.
    ///
    /// `None` quand la configuration du lecteur ne se lit pas : l'ecran se tait
    /// alors sur le nom plutot que d'en inventer un. Un « machine sans nom »
    /// affiche a cote de la version se lirait comme le nom du poste.
    pub machine: Option<String>,
    pub depouillement: Depouillement,
}

/// Le contexte du poste, aux emplacements habituels du lecteur.
pub fn contexte_du_poste() -> Contexte {
    contexte(&Config::chemin_par_defaut())
}

/// Le meme contexte, autour d'une configuration donnee.
///
/// Le chemin est passe explicitement pour que ces faits s'eprouvent sans rien
/// devoir a ce qui vit sur la machine.
pub fn contexte(chemin_config: &Path) -> Contexte {
    Contexte {
        version: VERSION,
        machine: nom_de_la_machine(chemin_config),
        depouillement: depouillement(chemin_config),
    }
}

/// Ou en est le depouillement, tel que sa marque le dit.
pub fn depouillement(chemin_config: &Path) -> Depouillement {
    avancement(&Marque::charger(&chemin_de_la_marque(chemin_config)))
}

/// La lecture de la marque, separee de sa relecture sur disque : c'est ici que
/// vit la decision, et elle s'eprouve sans fichier.
///
/// L'ordre compte.
///
/// 1. Sans denominateur, il n'y a pas d'avancement a montrer. Un passage l'a-t-il
///    conclu ? Alors il n'y a rien a depouiller ici (FR-065). Sinon, personne
///    n'a encore rien regarde.
/// 2. Un passage mene a son terme dont l'avancement a rejoint le total : il est
///    termine, et l'heure est celle de ce terme-la.
/// 3. Tout le reste avance : un passage interrompu, un passage en cours, ou un
///    nouveau passage ouvert par des journaux ecrits depuis la derniere fois -
///    l'heure de fin qu'ils portent encore est celle d'un passage precedent, et
///    l'annoncer comme un terme serait dire fini de ce qui recommence.
pub fn avancement(marque: &Marque) -> Depouillement {
    if marque.total == 0 {
        return match marque.termine_a {
            Some(_) => Depouillement::RienADepouiller,
            None => Depouillement::Jamais,
        };
    }

    match marque.termine_a {
        Some(quand) if marque.journaux >= marque.total => Depouillement::Termine {
            quand: quand.to_rfc3339(),
            journaux: marque.journaux,
        },
        _ => Depouillement::EnCours {
            journaux: marque.journaux,
            total: marque.total,
        },
    }
}

/// Le nom de cette machine, tel que la configuration du lecteur le porte
/// (FR-020, issue #65).
///
/// POURQUOI la configuration plutot que le systeme d'exploitation, alors que le
/// nom vient de lui : c'est CE nom-la qui est parti avec la declaration, et donc
/// celui que la liste des machines affiche. Redemander a `scutil` rendrait un
/// autre nom des que l'utilisateur renomme son Mac, et l'ecran Reglages
/// annoncerait un nom que la liste des machines ne porte pas - alors que le
/// critere demande justement qu'ils soient identiques.
///
/// C'est aussi ce qui evite de lancer un programme du systeme a chaque fois que
/// l'ecran redemande le contexte, c'est-a-dire toutes les deux secondes.
fn nom_de_la_machine(chemin_config: &Path) -> Option<String> {
    Config::load(chemin_config)
        .ok()
        .map(|config| config.label.trim().to_string())
        .filter(|label| !label.is_empty())
}
