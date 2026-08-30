//! Ouvrir l'autorisation GitHub dans le navigateur du systeme (FR-071).
//!
//! GitHub refuse les vues web embarquees : la page d'autorisation ne peut pas
//! s'afficher dans la fenetre, et la fenetre refuse de toute facon toute
//! origine qui n'est pas la sienne. C'est donc l'application qui l'ouvre
//! dehors, dans le navigateur habituel de l'utilisateur.
//!
//! ## Une des deux commandes du pont qui recoivent quelque chose
//!
//! Les autres sont des gestes nommes, sans argument. Deux font exception, et
//! pour la meme raison : la fenetre detient quelque chose que l'application ne
//! peut pas obtenir autrement. Ici c'est l'adresse d'autorisation ; dans
//! `machine.rs`, c'est la session de l'utilisateur, qui vit dans les cookies de
//! la fenetre. Celle-ci recoit une adresse, parce que la fenetre est seule a
//! savoir la demander a Supabase -
//! elle porte le defi PKCE que la fenetre vient de tirer, et l'application n'a
//! aucun moyen de le fabriquer a sa place.
//!
//! Recevoir une adresse a ouvrir dehors est precisement ce qui pourrait faire
//! de l'application une porte : « ouvre ceci pour moi ». D'ou la borne posee
//! ici, et non chez l'appelant : une adresse est acceptee quand elle vise
//! l'endroit d'autorisation d'un service GoTrue - le chemin
//! `/auth/v1/authorize`, exactement -, en clair seulement sur la boucle locale.
//! Tout le reste est refuse, et le refus est dit a la fenetre plutot qu'avale.

use std::process::Command;

/// Le seul chemin qu'une adresse d'autorisation peut viser. C'est l'endroit
/// d'autorisation de GoTrue, et il ne varie pas d'un projet Supabase a l'autre.
pub const CHEMIN_D_AUTORISATION: &str = "/auth/v1/authorize";

/// Pourquoi une adresse n'est pas ouverte.
///
/// Les cas sont separes parce qu'ils ne disent pas la meme chose : l'un
/// signale une pile locale mal configuree, les autres une adresse qui n'avait
/// rien a faire la.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "cas", rename_all = "snake_case")]
pub enum Refus {
    /// Ni `https`, ni `http` : rien de tout cela ne s'ouvre dans un navigateur.
    ProtocoleRefuse,
    /// Du `http` vers le dehors. L'autorisation porte un secret de session :
    /// elle ne voyage pas en clair hors de la machine.
    EnClairHorsDeLaMachine,
    /// Une adresse qu'on ne sait pas lire, ou qui cache sa vraie destination.
    AdresseSuspecte,
    /// Une adresse bien formee, mais qui ne vise pas une autorisation.
    PasUneAutorisation,
}

impl std::fmt::Display for Refus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refus::ProtocoleRefuse => write!(
                f,
                "cette adresse d'autorisation n'est pas une adresse web : Vibe Map ne l'ouvre pas."
            ),
            Refus::EnClairHorsDeLaMachine => write!(
                f,
                "cette adresse d'autorisation sort de la machine en clair : Vibe Map ne l'ouvre \
                 pas. Verifie l'adresse de ta pile Supabase."
            ),
            Refus::AdresseSuspecte => write!(
                f,
                "cette adresse d'autorisation ne se lit pas comme telle : Vibe Map ne l'ouvre pas."
            ),
            Refus::PasUneAutorisation => write!(
                f,
                "cette adresse ne mene pas a une autorisation ({CHEMIN_D_AUTORISATION}) : \
                 Vibe Map ne l'ouvre pas."
            ),
        }
    }
}

/// L'adresse est-elle une adresse d'autorisation, et rien d'autre ?
///
/// Quatre regles, et elles tiennent ensemble :
///
/// 1. `https`, ou `http` sur la boucle locale seulement - la pile Supabase de
///    developpement sert en clair sur `127.0.0.1` ;
/// 2. aucune identite dans l'autorite (`https://vrai.example.com@piege.test/`),
///    qui donne a lire un hote et en vise un autre ;
/// 3. le chemin vaut EXACTEMENT `/auth/v1/authorize` - un chemin qui se
///    contente de se terminer ainsi laisserait passer `/piege/auth/v1/authorize` ;
/// 4. ni espace ni caractere de controle, qu'une adresse legitime n'a pas.
///
/// L'hote n'est pas verifie : l'application ne connait pas le projet Supabase
/// de l'utilisateur, et le lui faire connaitre ne ferait que deplacer la
/// question. Ce qui est borne, c'est la NATURE de la destination.
pub fn verifier_l_adresse(url: &str) -> Result<(), Refus> {
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Refus::AdresseSuspecte);
    }

    let (en_clair, reste) = match url.strip_prefix("https://") {
        Some(reste) => (false, reste),
        None => match url.strip_prefix("http://") {
            Some(reste) => (true, reste),
            None => return Err(Refus::ProtocoleRefuse),
        },
    };

    let fin_autorite = reste.find(['/', '?', '#']).unwrap_or(reste.len());
    let autorite = &reste[..fin_autorite];
    if autorite.is_empty() || autorite.contains('@') {
        return Err(Refus::AdresseSuspecte);
    }
    if en_clair && !boucle_locale(autorite) {
        return Err(Refus::EnClairHorsDeLaMachine);
    }

    let chemin = reste[fin_autorite..]
        .split(['?', '#'])
        .next()
        .unwrap_or_default();
    if chemin != CHEMIN_D_AUTORISATION {
        return Err(Refus::PasUneAutorisation);
    }

    Ok(())
}

/// L'autorite designe-t-elle cette machine ?
fn boucle_locale(autorite: &str) -> bool {
    let hote = match autorite.strip_prefix('[') {
        // Une adresse IPv6 litterale : `[::1]:54321`.
        Some(reste) => match reste.split_once(']') {
            Some((hote, _)) => hote,
            None => return false,
        },
        None => autorite.split(':').next().unwrap_or_default(),
    };
    matches!(hote, "127.0.0.1" | "localhost" | "::1")
}

/// Ouvre l'adresse dans le navigateur du systeme.
///
/// Par un programme du systeme, avec l'adresse en argument : jamais par un
/// interpreteur de commandes, ou le contenu de l'adresse serait relu comme du
/// code.
///
/// L'adresse doit avoir passe `verifier_l_adresse` avant d'arriver ici.
pub fn ouvrir_dans_le_navigateur(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let programme = "/usr/bin/open";
    #[cfg(not(target_os = "macos"))]
    let programme = "xdg-open";

    let statut = Command::new(programme).arg(url).status()?;
    if statut.success() {
        return Ok(());
    }
    Err(std::io::Error::other(format!(
        "{programme} a rendu {statut}"
    )))
}
