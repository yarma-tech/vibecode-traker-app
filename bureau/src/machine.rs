//! La machine se declare elle-meme (issues #65, #66, #67 - FR-017 a FR-022,
//! FR-055 a FR-057, FR-073, FR-080).
//!
//! Le code d'appairage n'existait que pour faire se reconnaitre deux objets
//! separes. Ils n'en font plus qu'un : la fenetre et le lecteur vivent dans la
//! meme application, et une session ouverte suffit desormais a inscrire ce Mac
//! sur le compte, sans rien a recopier.
//!
//! ## Deux identites, et elles ne se melangent pas
//!
//! La SESSION de l'utilisateur vit dans les cookies de la fenetre. Le JETON de
//! la machine vit au trousseau du systeme, sous le service `fr.yarma.vibemap`
//! et sous le compte que forme l'identifiant de la machine - la meme entree,
//! exactement, que celle du binaire en ligne de commande (FR-018).
//!
//! La fenetre presente sa session a ce module ; ce module ne lui rend JAMAIS le
//! jeton de la machine. Rien dans `EtatMachine` ne peut le porter : la structure
//! n'a pas de champ ou le loger, et c'est ainsi que la regle tient - par la
//! forme, et non par la vigilance de qui ecrira la prochaine commande.
//!
//! ## L'identifiant conserve, et pourquoi il est tout
//!
//! Il n'y a pas d'empreinte materielle, pas de nom d'hote servant de cle :
//! l'application presente a chaque lancement l'identifiant que la base lui a
//! donne, conserve dans sa configuration a cote du jeton (FR-055). C'est LUI qui
//! tient la promesse « pas de doublon » (FR-019). Sans lui, une reinstallation -
//! ou un trousseau vide - fabriquerait une machine de plus a chaque fois, et la
//! liste des machines finirait par ne plus rien dire.
//!
//! ## Les deux facons de ne plus se reconnaitre, et elles ne se confondent pas
//!
//! Une machine **revoquee** n'est pas une machine **disparue** (issue #66).
//!
//! La revocation est une decision de l'utilisateur, prise depuis le web, et la
//! base la fait tenir : `machines.revoked_at` coupe les ecritures en RLS
//! (`machines_update_own`, `machine_du_jeton_valide()`) et `jeton_de_machine`
//! refuse de resigner. Redeclarer une machine revoquee la ferait revenir sous
//! une autre identite, jeton neuf compris : ce serait defaire la revocation par
//! le seul fait de rouvrir l'application. Elle s'annonce donc, et rien de plus.
//!
//! La disparition, elle, n'est la decision de personne : base remise a zero,
//! machine supprimee, compte change. L'identifiant conserve ne designe alors
//! aucune ligne, et il n'y a rien a revoquer ni a proteger. La machine se
//! redeclare, et la fenetre le dit (FR-056) - la ou se taire laisserait un poste
//! muet sans que personne ne sache pourquoi.
//!
//! La base rend elle-meme la difference (`DansLaBase`) : une machine revoquee
//! reste VISIBLE a la session de son proprietaire - la policy de lecture ne la
//! cache pas -, et c'est ce qui permet de la distinguer d'une ligne absente.
//!
//! ## Ce qui n'arrive jamais
//!
//! Un acces au trousseau refuse ne redeclare RIEN (FR-080). macOS redemande son
//! autorisation des que le programme change - et l'application EST un programme
//! different du binaire en ligne de commande : ce refus est un passage attendu,
//! pas une anomalie. Il s'annonce, avec un bouton pour reessayer. Redeclarer a
//! la place ferait apparaitre un doublon au moment precis ou l'utilisateur
//! hesite devant une boite de dialogue.

use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Value};
use vibemap::declaration::DansLaBase;
use vibemap::trousseau::TrousseauError;

/// L'adresse de la base, telle que CETTE application la connait (FR-073).
///
/// Fixee a la compilation du paquet, la meme pour tous les postes. Elle ne vient
/// ni de la fenetre - qui pourrait alors faire parler l'application a n'importe
/// quelle base -, ni d'une configuration de ligne de commande heritee, qui
/// pointe aujourd'hui sur la pile locale de developpement et n'a rien a
/// detourner.
///
/// La valeur de repli est cette meme pile locale : c'est la seule base qui
/// existe tant que l'hebergement n'est pas tranche (PRD-002, points ouverts).
pub fn adresse_de_la_base() -> &'static str {
    option_env!("VIBEMAP_SUPABASE_URL").unwrap_or("http://127.0.0.1:54321")
}

/// La plateforme, telle qu'elle part avec la declaration. Rien de plus precis :
/// « macos » suffit a la liste des machines, et une version de systeme serait
/// une ligne de plus dans ce qui sort du poste (FR-022).
pub fn plateforme() -> &'static str {
    std::env::consts::OS
}

/// Le nom que le systeme d'exploitation donne a cette machine (FR-020).
///
/// `scutil --get ComputerName` rend le nom que l'utilisateur a choisi dans les
/// Reglages du Mac - « MacBook de Yarma » -, et non le nom d'hote reseau, qui en
/// est une deformation sans espaces. C'est ce nom-la que l'utilisateur
/// reconnait dans la liste des machines.
///
/// Le programme est appele avec ses arguments, jamais par un interpreteur de
/// commandes : rien de ce qu'il rend n'est relu comme du code.
pub fn nom_de_la_machine() -> String {
    std::process::Command::new("/usr/sbin/scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .filter(|sortie| sortie.status.success())
        .map(|sortie| String::from_utf8_lossy(&sortie.stdout).trim().to_string())
        .filter(|nom| !nom.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .filter(|nom| !nom.trim().is_empty())
        .unwrap_or_else(|| "machine sans nom".to_string())
}

/* ---------- ce que le poste sait de lui-meme ---------- */

/// Ce que le trousseau rend quand on lui demande le jeton d'une machine.
///
/// Les trois cas ne se corrigent pas de la meme facon, et les confondre serait
/// exactement la faute que FR-080 interdit : un acces REFUSE se reessaie apres
/// avoir accorde l'autorisation, un jeton ABSENT se redemande a la base pour la
/// meme machine. Traiter le premier comme le second redeclarerait la machine
/// d'un utilisateur qui a simplement clique « Refuser ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuTrousseau {
    /// Le jeton est la.
    Present,
    /// Aucune entree pour cette machine : trousseau vide, poste reinstalle.
    Absent,
    /// Le systeme refuse l'acces, et dit pourquoi.
    Refuse(String),
}

/// Va voir au trousseau, sous l'entree exacte du binaire en ligne de commande
/// (FR-018, FR-080).
///
/// Sur macOS, cet appel peut faire apparaitre une boite de dialogue du systeme
/// et durer aussi longtemps qu'il faut a l'utilisateur pour y repondre : il ne
/// s'appelle donc jamais depuis le fil de la fenetre.
pub fn au_trousseau(machine_id: &str) -> AuTrousseau {
    // La meme echappatoire que le lecteur (`Lecteur::preparer`) : sans elle,
    // l'application et le binaire ne liraient pas le meme jeton sur un poste de
    // developpement ou dans un conteneur sans session graphique.
    if std::env::var("VIBEMAP_TOKEN").is_ok() {
        return AuTrousseau::Present;
    }

    match vibemap::trousseau::lire(machine_id) {
        Ok(_) => AuTrousseau::Present,
        Err(TrousseauError::Absent(_)) => AuTrousseau::Absent,
        Err(refus) => AuTrousseau::Refuse(refus.to_string()),
    }
}

/* ---------- ce que la fenetre lit ---------- */

/// Ou en est la machine, tel que la fenetre le montre.
///
/// Aucun de ces cas ne porte de jeton, et c'est la garantie centrale de cette
/// tranche : le jeton de la machine ne transite jamais par la fenetre. La
/// structure n'a pas de champ ou le loger.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatMachine {
    /// L'identifiant conserve designe bien une machine du compte, et son jeton
    /// est au trousseau. Rien n'a ete envoye, rien n'a ete cree (FR-019).
    Reprise { machine_id: String, label: String },
    /// Elle vient d'etre inscrite sur le compte (FR-017).
    Declaree { machine_id: String, label: String },
    /// Elle a ete revoquee depuis le web (FR-021) : elle n'emet plus, et elle
    /// ne se redeclare pas sous une autre identite.
    Revoquee { machine_id: String, label: String },
    /// L'identifiant conserve ne designait plus rien sur ce compte - base
    /// remise a zero, machine supprimee, compte change -, et la machine vient
    /// d'etre inscrite a nouveau (FR-056). Le `machine_id` est le NOUVEAU :
    /// l'ancien ne designait rien, et il n'y a rien a en faire.
    ///
    /// Ce cas s'annonce, contrairement a `Declaree` : l'utilisateur voit sa
    /// carte se vider puis se repeupler, et il a droit a la raison.
    Redeclaree { machine_id: String, label: String },
    /// Le systeme a refuse l'acces au jeton (FR-080). On le dit, on propose de
    /// reessayer, et on ne declare rien.
    TrousseauRefuse { machine_id: String, raison: String },
    /// Tout le reste : base injoignable, configuration qui ne s'ecrit pas.
    Echec { raison: String },
}

/* ---------- la decision ---------- */

/// Ce qu'il y a a faire d'un identifiant conserve.
///
/// Ne porte aucun jeton, a dessein : ce type circule entre la decision et son
/// execution, et un jeton qui s'y logerait finirait par ressortir quelque part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Geste {
    /// Rien a envoyer, rien a ecrire : la machine est la et son jeton aussi.
    Reprendre { label: String },
    /// Un jeton neuf pour LA MEME machine : le trousseau ne l'avait plus.
    RedemanderLeJeton { label: String },
    /// Inscrire cette machine a nouveau, et le dire (FR-056).
    ///
    /// Le SEUL geste de reprise qui cree une ligne, et il ne s'atteint que
    /// depuis `DansLaBase::Inconnue` - jamais depuis une revocation, jamais
    /// depuis un refus du trousseau. Il ne porte pas de label : l'ancien ne vaut
    /// plus rien, et le nom part du systeme d'exploitation comme pour une
    /// premiere declaration (FR-020).
    Redeclarer,
    /// Ne rien envoyer, et le dire.
    Annoncer(EtatMachine),
}

/// La regle, une fois la base interrogee sur l'identifiant conserve.
///
/// POURQUOI le trousseau arrive en fermeture et non en valeur : sur macOS,
/// l'ouvrir fait apparaitre une boite de dialogue du systeme. Il n'y a aucune
/// raison de la montrer pour une machine que le compte ne connait plus, ni pour
/// une machine revoquee dont on vient de couper les ecritures. La fermeture rend
/// aussi cette retenue eprouvable : un test qui passe une fermeture qui panique
/// prouve qu'elle n'a pas ete appelee.
///
/// Et la regle qui compte : une machine que la base RECONNAIT ne se redeclare
/// jamais, quoi que dise le trousseau (FR-019, FR-080) - qu'elle emette encore
/// ou qu'elle ait ete revoquee. La redeclaration ne s'atteint que par un
/// identifiant qui ne designe plus aucune ligne (FR-056).
pub fn geste_de_reprise(
    machine_id: &str,
    dans_la_base: DansLaBase,
    trousseau: impl FnOnce() -> AuTrousseau,
) -> Geste {
    let label = match dans_la_base {
        // Rien a proteger : cet identifiant ne designe aucune ligne, donc
        // aucune revocation ne peut etre contournee ici.
        DansLaBase::Inconnue => return Geste::Redeclarer,
        DansLaBase::Revoquee { label } => {
            return Geste::Annoncer(EtatMachine::Revoquee {
                machine_id: machine_id.to_string(),
                label,
            })
        }
        DansLaBase::Presente { label } => label,
    };

    match trousseau() {
        AuTrousseau::Present => Geste::Reprendre { label },
        AuTrousseau::Absent => Geste::RedemanderLeJeton { label },
        AuTrousseau::Refuse(raison) => Geste::Annoncer(EtatMachine::TrousseauRefuse {
            machine_id: machine_id.to_string(),
            raison,
        }),
    }
}

/* ---------- l'identite conservee sur le poste (FR-055, FR-057, FR-073) ------ */

/// Ce qu'une configuration deja presente sur le poste rend a l'application.
///
/// C'est le chemin de migration, et il n'en existe pas d'autre : le binaire en
/// ligne de commande ecrit `~/.config/vibemap/config.toml`, l'application lit ce
/// meme fichier, et un poste deja appaire se retrouve donc chez lui sans rien
/// avoir a refaire (FR-057).
///
/// ## Ce qui n'est PAS repris, et pourquoi la structure le garantit
///
/// L'adresse de la base (FR-073). Ce champ est obligatoire dans la
/// configuration du binaire, et il pointe aujourd'hui sur la pile locale de
/// developpement : une reprise naive ferait parler l'application publiee a une
/// base de mise au point. Elle tient son adresse de sa compilation
/// (`adresse_de_la_base`), la meme pour tous les postes.
///
/// `Reprise` n'a aucun champ ou loger cette adresse - meme forme que pour le
/// jeton dans `EtatMachine` : la regle tient par la structure, et non par la
/// vigilance de qui ecrira la prochaine ligne. L'ajouter ici ferait tomber la
/// compilation des tests qui construisent ce type.
///
/// Le JETON n'y est pas non plus, pour une raison differente : il n'a jamais ete
/// dans ce fichier. Il vit au trousseau, sous la meme entree que celle du
/// binaire, et c'est `au_trousseau` qui va l'y chercher (FR-018, FR-080).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reprise {
    /// L'identifiant de machine du binaire en ligne de commande. C'est lui qui
    /// evite le doublon : sans lui, l'application declarerait une seconde
    /// machine a cote de celle qui bat deja (FR-019).
    pub machine_id: String,
    /// Les dossiers surveilles, tels qu'ils sont ecrits. Ils ne sont ni
    /// dupliques ni effaces : l'application ne les recopie nulle part, elle
    /// laisse le fichier les porter et se contente de les relire.
    pub dossiers: Vec<String>,
}

/// Lit la configuration deja presente sur le poste, s'il y en a une.
///
/// Lecture tolerante, et volontairement plus large que `Config::load` : un poste
/// neuf n'a pas de configuration du tout, et une configuration a laquelle il
/// manque un champ obligatoire ne doit pas empecher de retrouver l'identifiant
/// qui, lui, est la. `Config::load` refuserait le fichier entier, et
/// l'application declarerait une machine de plus a cote de celle qui existe.
///
/// Sans identifiant de machine, il n'y a rien a reprendre : ce poste ne sait pas
/// a quelle machine il appartient, et le seul chemin honnete est la declaration.
/// Ses dossiers, eux, restent la ou ils sont - `poser_l_identite` ne les touche
/// pas.
pub fn reprise_de_la_configuration(chemin_config: &Path) -> Option<Reprise> {
    let brut = std::fs::read_to_string(chemin_config).ok()?;
    let document: DocumentMut = brut.parse().ok()?;

    let machine_id = document
        .get("machine_id")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|machine_id| !machine_id.is_empty())?
        .to_string();

    let dossiers = document
        .get("roots")
        .and_then(Item::as_array)
        .map(|racines| {
            racines
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|dossier| !dossier.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Some(Reprise {
        machine_id,
        dossiers,
    })
}

/// L'identifiant de machine que ce poste conserve, s'il en a un (FR-055).
///
/// La meme lecture que `reprise_de_la_configuration`, et volontairement la
/// meme : deux lecteurs du meme fichier finiraient par ne plus dire la meme
/// chose, et le jour ou ils divergeraient, l'un d'eux redeclarerait une machine
/// que l'autre reconnaissait.
pub fn identite_conservee(chemin_config: &Path) -> Option<String> {
    reprise_de_la_configuration(chemin_config).map(|reprise| reprise.machine_id)
}

/// Ecrit l'identite de la machine dans la configuration du poste (FR-055).
///
/// Le fichier reste le support de stockage, et l'edition preserve tout le
/// reste : les dossiers surveilles, les cadences, et les commentaires que
/// l'utilisateur y a mis (FR-036, FR-057). Aucun secret n'y entre - le jeton est
/// au trousseau, et il n'y a pas de champ `token` ici.
///
/// L'adresse de la base est ecrite avec, et c'est voulu : c'est celle de la
/// compilation de l'application, la meme pour tous les postes, et c'est elle que
/// le lecteur doit lire ensuite (FR-073). Une configuration heritee qui en
/// designait une autre ne detourne donc pas l'application.
///
/// POURQUOI un fichier voisin puis un renommage : une ecriture interrompue au
/// milieu laisserait une configuration tronquee, et le poste ne saurait plus a
/// quelle machine il appartient. Le renommage, lui, est atomique. Meme
/// precaution que pour l'ajout d'un dossier surveille, et pour la meme raison.
pub fn poser_l_identite(
    chemin_config: &Path,
    base: &str,
    machine_id: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(dossier) = chemin_config.parent() {
        std::fs::create_dir_all(dossier)
            .map_err(|erreur| format!("{} n'a pas pu etre cree : {erreur}", dossier.display()))?;
    }

    let brut = std::fs::read_to_string(chemin_config).unwrap_or_default();
    let mut document: DocumentMut = brut
        .parse()
        .map_err(|erreur| format!("{} est illisible : {erreur}", chemin_config.display()))?;

    document["supabase_url"] = Item::Value(Value::from(base));
    document["machine_id"] = Item::Value(Value::from(machine_id));
    document["label"] = Item::Value(Value::from(label));

    // Un poste neuf n'a jamais eu de configuration : sans cette ligne, le
    // lecteur ne surveillerait rien du tout et la carte resterait vide sans
    // qu'on sache pourquoi. Une configuration existante garde la sienne.
    if document.get("roots").is_none() {
        let mut racines = toml_edit::Array::new();
        racines.push("~/Developer");
        document["roots"] = Item::Value(Value::Array(racines));
    }

    ecrire_sans_perdre(chemin_config, &document.to_string())
}

/// Aligne l'adresse de la base de la configuration sur celle de cette
/// application (FR-073). Rend `true` si le fichier a ete touche.
///
/// POURQUOI cela ne concerne pas que la fenetre : l'application et le lecteur
/// qu'elle embarque partagent ce fichier. La fenetre, elle, tient son adresse de
/// sa compilation et ne lit jamais `supabase_url` - mais le lecteur, lui, ne
/// connait que ce champ. Une configuration heritee du binaire en ligne de
/// commande y designe la pile locale de developpement : sans cet alignement, le
/// poste ecrirait ses cartes dans une base pendant que la fenetre en lirait une
/// autre, et l'utilisateur verrait sa machine apparaitre sans jamais voir sa
/// carte.
///
/// N'ecrit QUE si l'adresse differe. Un lancement ordinaire - et il y en aura
/// des milliers pour une seule migration - ne touche alors pas au disque, et les
/// commentaires comme la mise en page du fichier restent ou ils sont.
///
/// Le reste de la configuration n'est pas relu ni valide : un champ qui manque
/// ou une cadence farfelue ne doivent pas empecher d'ecrire cette ligne-la.
pub fn aligner_l_adresse_de_la_base(chemin_config: &Path, base: &str) -> Result<bool, String> {
    let brut = std::fs::read_to_string(chemin_config).map_err(|erreur| {
        format!(
            "{} n'a pas pu etre relu : {erreur}",
            chemin_config.display()
        )
    })?;
    let mut document: DocumentMut = brut
        .parse()
        .map_err(|erreur| format!("{} est illisible : {erreur}", chemin_config.display()))?;

    if document.get("supabase_url").and_then(Item::as_str) == Some(base) {
        return Ok(false);
    }

    document["supabase_url"] = Item::Value(Value::from(base));
    ecrire_sans_perdre(chemin_config, &document.to_string())?;
    Ok(true)
}

/// L'ecriture en deux temps : un voisin, puis un renommage atomique.
///
/// Le meme geste que `dossiers::ajouter`, ecrit ici plutot que partage : les
/// deux chantiers touchent ce fichier pour des raisons differentes, et une
/// fonction commune les ferait se marcher dessus pour economiser dix lignes.
fn ecrire_sans_perdre(chemin: &Path, contenu: &str) -> Result<(), String> {
    let voisin = voisin_temporaire(chemin);

    std::fs::write(&voisin, contenu)
        .map_err(|erreur| format!("{} n'a pas pu etre ecrit : {erreur}", voisin.display()))?;

    std::fs::rename(&voisin, chemin).map_err(|erreur| {
        let _ = std::fs::remove_file(&voisin);
        format!("{} n'a pas pu etre remplace : {erreur}", chemin.display())
    })
}

/// Le fichier voisin, dans le meme dossier : un renommage n'est atomique qu'au
/// sein d'un meme systeme de fichiers.
fn voisin_temporaire(chemin: &Path) -> PathBuf {
    let nom = chemin
        .file_name()
        .map(|nom| nom.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.toml".to_string());

    chemin.with_file_name(format!(".{nom}.{}.vibemap", std::process::id()))
}

/* ---------- le parcours complet ---------- */

/// Presente l'identite du poste, et declare la machine s'il n'en a pas.
///
/// L'ordre n'est pas negociable :
///
/// 1. **l'identifiant conserve d'abord**. S'il existe, il n'y a rien a
///    declarer, et c'est tout l'objet de FR-019 ;
/// 2. **la base ensuite**, qui dit si cet identifiant designe encore quelque
///    chose. Une lecture, jamais une ecriture ;
/// 3. **le trousseau en dernier**, et seulement si la base a confirme : c'est
///    lui qui peut ouvrir une boite de dialogue devant l'utilisateur.
///
/// Entre les deux derniers s'intercale l'alignement de l'adresse de la base
/// (FR-073), pour la meme raison qu'une declaration ecrit la configuration avant
/// d'ouvrir le trousseau : le disque d'abord, la boite de dialogue ensuite.
///
/// La declaration, elle, ecrit la configuration AVANT de ranger le jeton. Si le
/// trousseau echoue, le poste garde son identifiant et le lancement suivant
/// redemande un jeton pour la meme machine - la ou l'ordre inverse aurait laisse
/// un poste sans identite devant une machine deja creee, donc un doublon au
/// lancement suivant.
///
/// Et une base injoignable n'est pas une identite perdue : elle s'arrete a
/// l'etape 2, sur un `Echec`, et ne redeclare rien. Confondre les deux
/// fabriquerait une machine de plus a chaque coupure de reseau.
pub async fn assurer(
    chemin_config: &Path,
    base: &str,
    jeton_de_session: &str,
    nom: &str,
    plateforme: &str,
) -> EtatMachine {
    // Seul l'identifiant conduit la suite : les dossiers surveilles ne sont ni
    // repris ni recopies, ils restent dans le fichier qui les porte (FR-057).
    let Some(Reprise { machine_id, .. }) = reprise_de_la_configuration(chemin_config) else {
        // Aucune identite conservee : le seul cas de premiere declaration
        // (FR-017). Une configuration deja presente sur le poste - dossiers
        // surveilles, cadences, commentaires - traverse l'ecriture intacte.
        return match inscrire(chemin_config, base, jeton_de_session, nom, plateforme).await {
            Ok((machine_id, label)) => EtatMachine::Declaree { machine_id, label },
            Err(echec) => echec,
        };
    };

    let dans_la_base =
        match vibemap::declaration::representer(base, jeton_de_session, &machine_id).await {
            Ok(vue) => vue,
            Err(erreur) => {
                return EtatMachine::Echec {
                    raison: erreur.to_string(),
                }
            }
        };

    // La machine est reconnue : elle va reprendre son battement, et le lecteur
    // qui l'emet doit ecrire dans la base que la fenetre lit (FR-073). Une
    // configuration heritee du binaire en ligne de commande designe la pile
    // locale de developpement, et c'est ici que cet heritage s'arrete.
    //
    // Avant le trousseau, comme pour une declaration : le disque d'abord, la
    // boite de dialogue du systeme ensuite.
    if matches!(dans_la_base, DansLaBase::Presente { .. }) {
        if let Err(raison) = aligner_l_adresse_de_la_base(chemin_config, base) {
            return EtatMachine::Echec { raison };
        }
    }

    match geste_de_reprise(&machine_id, dans_la_base, || au_trousseau(&machine_id)) {
        Geste::Annoncer(etat) => etat,
        Geste::Reprendre { label } => EtatMachine::Reprise { machine_id, label },
        Geste::RedemanderLeJeton { label } => {
            match vibemap::declaration::jeton_de_machine(base, jeton_de_session, &machine_id).await
            {
                Ok(jeton) => match vibemap::trousseau::ranger(&machine_id, &jeton) {
                    Ok(()) => EtatMachine::Reprise { machine_id, label },
                    Err(refus) => EtatMachine::TrousseauRefuse {
                        machine_id,
                        raison: refus.to_string(),
                    },
                },
                Err(erreur) => EtatMachine::Echec {
                    raison: erreur.to_string(),
                },
            }
        }
        // L'identifiant conserve est remplace par le neuf au passage : c'est
        // `poser_l_identite` qui l'ecrit, et le lancement suivant se reconnaitra
        // sous cette nouvelle identite plutot que de redeclarer encore.
        Geste::Redeclarer => {
            match inscrire(chemin_config, base, jeton_de_session, nom, plateforme).await {
                Ok((machine_id, label)) => EtatMachine::Redeclaree { machine_id, label },
                Err(echec) => echec,
            }
        }
    }
}

/// Le seul chemin qui cree une machine (FR-017, FR-056).
///
/// Deux portes y menent, et deux seulement : un poste sans identite conservee,
/// et un identifiant que la base ne reconnait plus. Une machine revoquee n'en
/// est pas une - `geste_de_reprise` la renvoie sur une annonce, et c'est la que
/// la revocation tient.
///
/// Rend l'identite inscrite, ou l'etat a annoncer si quelque chose a cede en
/// route. C'est l'appelant qui nomme ce qui vient d'avoir lieu - une premiere
/// declaration ou une redeclaration -, parce que lui seul sait d'ou il vient.
async fn inscrire(
    chemin_config: &Path,
    base: &str,
    jeton_de_session: &str,
    nom: &str,
    plateforme: &str,
) -> Result<(String, String), EtatMachine> {
    let identite =
        match vibemap::declaration::declarer(base, jeton_de_session, nom, Some(plateforme)).await {
            Ok(identite) => identite,
            Err(erreur) => {
                return Err(EtatMachine::Echec {
                    raison: erreur.to_string(),
                })
            }
        };

    if let Err(raison) =
        poser_l_identite(chemin_config, base, &identite.machine_id, &identite.label)
    {
        return Err(EtatMachine::Echec { raison });
    }

    if let Err(refus) = vibemap::trousseau::ranger(&identite.machine_id, &identite.token) {
        return Err(EtatMachine::TrousseauRefuse {
            machine_id: identite.machine_id,
            raison: refus.to_string(),
        });
    }

    Ok((identite.machine_id, identite.label))
}
