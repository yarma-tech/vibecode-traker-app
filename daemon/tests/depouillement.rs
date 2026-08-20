//! Ce que le depouillement du passe tire d'un journal, ce qu'il n'en tire pas,
//! et ce qu'il ne relit pas.
//!
//! Rien ici ne parle au reseau : le depouillement d'un journal est une fonction
//! pure sur du texte, comme sa cousine `journal::lire`, et le choix des journaux
//! a reprendre en est une autre. La propriete centrale de la tranche - « du
//! passe, mais aucune couleur » - s'eprouve de l'exterieur, contre la vraie
//! base, dans `passe_sans_couleur.rs`.
//!
//! Les tests de reprise se donnent un client injoignable et une carte des
//! depots : chaque journal lu produit alors un envoi qui echoue, et le compte
//! des defauts dit exactement quels journaux ont ete lus. Un journal qu'on
//! croyait passe et qui a ete relu se voit donc, au lieu de se deviner.

use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use vibemap::depouillement::{
    self, a_depouiller, chemin_de_la_marque, depouiller, horizon, seuil, touches_du_journal,
    Marque, FENETRE_JOURS,
};
use vibemap::journal::{Suivi, NOM_DES_OFFSETS};

const ATELIER: &str = "/Users/moi/Developer/atelier";

/// Une ligne d'assistant telle qu'elle apparait dans un vrai journal.
fn ligne(tool: &str, chemin: &str, id: &str, instant: DateTime<Utc>) -> String {
    ligne_depuis(ATELIER, tool, chemin, id, instant)
}

fn ligne_depuis(cwd: &str, tool: &str, chemin: &str, id: &str, instant: DateTime<Utc>) -> String {
    serde_json::json!({
        "type": "assistant",
        "timestamp": instant.to_rfc3339(),
        "sessionId": "9adbb340-9563-4b1d-878e-4e2d5032ac67",
        "cwd": cwd,
        "gitBranch": "main",
        "message": {
            "role": "assistant",
            "content": [{
                "type": "tool_use",
                "id": id,
                "name": tool,
                "input": { "file_path": chemin }
            }],
            "usage": { "input_tokens": 2, "output_tokens": 18 }
        }
    })
    .to_string()
}

/// La carte des depots connus : un seul, l'atelier.
fn carte() -> BTreeMap<PathBuf, String> {
    BTreeMap::from([(PathBuf::from(ATELIER), "repo-atelier".to_string())])
}

/// Les deux dates d'une zone dans un lot de touches.
fn zone<'a>(
    lots: &'a [depouillement::LotTouches],
    repo_id: &str,
    chemin: &str,
) -> Option<&'a vibemap::DerniereTouche> {
    lots.iter()
        .find(|lot| lot.repo_id == repo_id)?
        .touches
        .iter()
        .find(|touche| touche.chemin == chemin)
}

/// Un dossier de travail jetable, efface a la fin du test.
struct Bac(PathBuf);

impl Bac {
    fn new() -> Self {
        let chemin =
            std::env::temp_dir().join(format!("vibemap-depouillement-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&chemin).unwrap();
        Bac(chemin)
    }

    /// Le dossier des journaux, cree au besoin.
    fn journaux(&self) -> PathBuf {
        let dossier = self.0.join("projects");
        std::fs::create_dir_all(&dossier).unwrap();
        dossier
    }

    /// Ou vivrait la marque, a cote d'une configuration.
    fn marque(&self) -> PathBuf {
        chemin_de_la_marque(&self.0.join("config.toml"))
    }

    /// Ecrit un journal d'une ligne, ecrit a la date dite.
    fn journal(&self, nom: &str, ecrit_a: DateTime<Utc>) -> PathBuf {
        let chemin = self.journaux().join(nom);
        std::fs::write(
            &chemin,
            format!("{}\n", ligne("Write", "src/a.rs", "toolu_1", Utc::now())),
        )
        .unwrap();
        dater(&chemin, ecrit_a);
        chemin
    }
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Antidate un journal : sa date d'ecriture est ce sur quoi la reprise se regle.
fn dater(chemin: &Path, quand: DateTime<Utc>) {
    std::fs::File::options()
        .write(true)
        .open(chemin)
        .unwrap()
        .set_modified(SystemTime::from(quand))
        .unwrap();
}

/// Un client qui ne repondra jamais : rien ne sort de la machine.
///
/// Chaque envoi refuse compte un defaut, et c'est ce qui rend visible qu'un
/// journal a bel et bien ete lu.
fn client_injoignable() -> vibemap::Supabase {
    vibemap::Supabase::new("http://127.0.0.1:1", "jeton-de-test")
}

/// Les noms des journaux retenus, dans l'ordre ou ils seront repris.
fn noms(restants: &[(PathBuf, DateTime<Utc>)]) -> Vec<String> {
    restants
        .iter()
        .map(|(chemin, _)| chemin.file_name().unwrap().to_string_lossy().to_string())
        .collect()
}

/// Tracer bullet : un journal rend les deux dates de sa zone, chacune de son cote.
#[test]
fn un_journal_rend_les_deux_dates_de_sa_zone() {
    let maintenant = Utc::now();
    let ecrit = maintenant - Duration::days(3);
    let lu = maintenant - Duration::days(1);

    let journal = [
        ligne("Write", "src/core/auth.rs", "toolu_1", ecrit),
        ligne("Read", "src/core/auth.rs", "toolu_2", lu),
    ]
    .join("\n");

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));
    let touche = zone(&lots, "repo-atelier", "src/core").expect("la zone doit porter ses dates");

    assert_eq!(touche.derniere_ecriture, Some(ecrit));
    assert_eq!(touche.derniere_lecture, Some(lu));
}

/// Une lecture ne chasse pas la date d'ecriture, ni l'inverse (FR-039, FR-040).
#[test]
fn une_lecture_recente_ne_chasse_pas_une_ecriture_ancienne() {
    let maintenant = Utc::now();
    let vieille_ecriture = maintenant - Duration::days(20);
    let lecture_fraiche = maintenant - Duration::minutes(5);

    let journal = [
        ligne("Write", "src/a.rs", "toolu_1", vieille_ecriture),
        ligne("Read", "src/a.rs", "toolu_2", lecture_fraiche),
    ]
    .join("\n");

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));
    let touche = zone(&lots, "repo-atelier", "src").expect("la zone doit porter ses dates");

    assert_eq!(touche.derniere_ecriture, Some(vieille_ecriture));
    assert_eq!(touche.derniere_lecture, Some(lecture_fraiche));
}

/// La borne des trente jours, sur l'horodatage de la ligne (FR-051).
#[test]
fn rien_de_plus_vieux_que_trente_jours_n_est_rendu() {
    let maintenant = Utc::now();
    let journal = ligne(
        "Write",
        "src/a.rs",
        "toolu_1",
        maintenant - Duration::days(45),
    );

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));

    assert!(
        lots.is_empty(),
        "un journal de 45 jours ne doit rien poser : {lots:?}"
    );
}

/// Le meme journal peut porter les deux : seule la part recente compte.
#[test]
fn dans_un_meme_journal_seules_les_lignes_recentes_comptent() {
    let maintenant = Utc::now();
    let recent = maintenant - Duration::days(2);

    let journal = [
        ligne(
            "Write",
            "vieux/a.rs",
            "toolu_1",
            maintenant - Duration::days(FENETRE_JOURS + 1),
        ),
        ligne("Write", "recent/b.rs", "toolu_2", recent),
    ]
    .join("\n");

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));

    assert!(zone(&lots, "repo-atelier", "vieux").is_none());
    assert_eq!(
        zone(&lots, "repo-atelier", "recent")
            .expect("la zone recente doit etre la")
            .derniere_ecriture,
        Some(recent)
    );
}

/// Le pas de la borne est bien de trente jours, ni vingt-neuf ni trente et un.
#[test]
fn la_borne_tombe_a_trente_jours_pile() {
    let maintenant = Utc::now();
    assert_eq!(
        maintenant - horizon(maintenant),
        Duration::days(FENETRE_JOURS)
    );
}

/// Un depot absent du catalogue est ignore sans erreur (FR-066).
#[test]
fn un_depot_inconnu_est_ignore_sans_erreur() {
    let maintenant = Utc::now();
    let journal = [
        ligne_depuis(
            "/Users/moi/Developer/efface",
            "Write",
            "src/parti.rs",
            "toolu_1",
            maintenant - Duration::days(1),
        ),
        ligne("Write", "src/reste.rs", "toolu_2", maintenant),
    ]
    .join("\n");

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));

    assert_eq!(lots.len(), 1, "seul le depot connu compte : {lots:?}");
    assert!(zone(&lots, "repo-atelier", "src").is_some());
}

/// Une ligne illisible ne fait pas tomber le depouillement du reste.
#[test]
fn une_ligne_illisible_n_arrete_pas_le_journal() {
    let maintenant = Utc::now();
    let journal = format!(
        "{{ pas du JSON\n{}\n",
        ligne("Write", "src/a.rs", "toolu_1", maintenant)
    );

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));

    assert!(zone(&lots, "repo-atelier", "src").is_some());
}

/// Ce qui sort du depouillement est une liste fermee : un chemin, deux dates.
///
/// Le journal fabrique ici porte tout ce qui allume la carte - une session, des
/// identifiants d'appel d'outil, des noms de fichiers, des jetons. Rien de cela
/// n'a d'endroit ou se loger dans ce qui part.
#[test]
fn ce_qui_part_ne_porte_ni_session_ni_fichier() {
    let maintenant = Utc::now();
    let journal = [
        ligne("Write", "src/core/auth.rs", "toolu_1", maintenant),
        ligne("Read", "src/core/auth.rs", "toolu_2", maintenant),
    ]
    .join("\n");

    let lots = touches_du_journal(&journal, &carte(), horizon(maintenant));

    for lot in &lots {
        for touche in &lot.touches {
            let charge = touche.charge();
            let cles: Vec<&String> = charge
                .as_object()
                .expect("un objet JSON")
                .keys()
                .collect::<Vec<_>>();
            assert_eq!(
                cles,
                vec!["chemin", "ecrit_a", "lu_a"],
                "la charge du depouillement doit rester fermee : {charge}"
            );
            assert!(
                !charge.to_string().contains("auth.rs"),
                "aucun nom de fichier ne part : {charge}"
            );
            assert!(
                !charge.to_string().contains("toolu_"),
                "aucun identifiant d'appel d'outil ne part : {charge}"
            );
        }
    }
}

/// Les deux marques vivent dans deux fichiers, et non dans le meme (FR-084).
#[test]
fn la_marque_et_la_position_du_direct_ne_sont_pas_le_meme_fichier() {
    let config = PathBuf::from("/tmp/vibemap/config.toml");

    assert_ne!(
        chemin_de_la_marque(&config),
        config.with_file_name(NOM_DES_OFFSETS),
        "la marque du depouillement ne doit pas etre la position du direct"
    );
}

/// Poser la marque n'ecrase pas la position du direct, et reciproquement.
///
/// POURQUOI ce test : le compagnon en ligne de commande peut avoir fait avancer
/// la lecture vivante pendant que l'application etait fermee. Si les deux
/// marques partageaient un fichier, le depouillement tiendrait pour depouille ce
/// que le direct a seulement consomme.
#[test]
fn aucune_des_deux_marques_n_ecrase_l_autre() {
    let bac = Bac::new();
    let config = bac.0.join("config.toml");
    let chemin_offsets = config.with_file_name(NOM_DES_OFFSETS);
    let chemin_marque = chemin_de_la_marque(&config);

    // Le direct avance : un journal lu, une position ecrite.
    let journaux = bac.0.join("projects");
    std::fs::create_dir_all(&journaux).unwrap();
    std::fs::write(
        journaux.join("a.jsonl"),
        format!("{}\n", ligne("Write", "src/a.rs", "toolu_1", Utc::now())),
    )
    .unwrap();

    let mut suivi = Suivi::charger(&chemin_offsets);
    suivi.nouveaux(&journaux, Utc::now() - Duration::seconds(600));
    suivi.enregistrer().expect("position du direct ecrite");
    let position_avant = std::fs::read(&chemin_offsets).expect("offsets.json existe");

    // Le depouillement avance et pose sa marque.
    let mut marque = Marque::charger(&chemin_marque);
    marque.avancer(Utc::now() - Duration::hours(2));
    marque.poser(Utc::now(), 12, 40);
    marque.enregistrer().expect("marque ecrite");

    assert_eq!(
        std::fs::read(&chemin_offsets).unwrap(),
        position_avant,
        "poser la marque a touche la position du direct"
    );

    // Le direct repart : sa nouvelle position ne doit pas effacer la marque.
    let marque_avant = std::fs::read(&chemin_marque).expect("la marque existe");
    std::fs::write(
        journaux.join("b.jsonl"),
        format!("{}\n", ligne("Read", "src/b.rs", "toolu_2", Utc::now())),
    )
    .unwrap();
    suivi.nouveaux(&journaux, Utc::now() - Duration::seconds(600));
    suivi.enregistrer().expect("position du direct reecrite");

    assert_ne!(
        std::fs::read(&chemin_offsets).unwrap(),
        position_avant,
        "le direct devait bien avancer, sinon le test ne prouve rien"
    );
    assert_eq!(
        std::fs::read(&chemin_marque).unwrap(),
        marque_avant,
        "le direct a ecrase la marque du depouillement"
    );

    // Et la marque se relit telle qu'elle a ete posee.
    let relue = Marque::charger(&chemin_marque);
    assert_eq!(relue.journaux, 12);
    assert_eq!(relue.total, 40);
    assert!(relue.termine_a.is_some());
    assert!(relue.depouille_jusqu_a.is_some());
}

/// Une marque absente ne fait pas paniquer : elle est simplement vierge.
#[test]
fn une_marque_absente_rend_une_marque_vierge() {
    let bac = Bac::new();
    let marque = Marque::charger(&bac.0.join("jamais-ecrite.json"));

    assert_eq!(marque.termine_a, None);
    assert_eq!(marque.depouille_jusqu_a, None);
    assert_eq!(marque.journaux, 0);
}

/// Une marque corrompue non plus : un demarrage refuse serait pire.
///
/// Elle repart alors d'une marque absente : trente jours de journaux a relire,
/// ce qui coute un passage de plus et rien d'autre.
#[test]
fn une_marque_corrompue_ne_fait_pas_paniquer() {
    let bac = Bac::new();
    let chemin = bac.0.join("depouillement.json");
    std::fs::write(&chemin, "{ a moitie ecr").unwrap();

    let marque = Marque::charger(&chemin);

    assert_eq!(marque.termine_a, None);
    assert_eq!(marque.depouille_jusqu_a, None);
}

/// Le reste du lecteur continue de tourner pendant un depouillement (FR-048).
///
/// POURQUOI ce test tient sans reseau et sans mesure de temps : il tourne sur
/// l'ordonnanceur a un seul fil de `#[tokio::test]`. Une tache voisine n'y
/// avance que lorsque la tache principale rend la main. Depouiller trois cents
/// journaux d'affilee sans jamais rendre la main - ce que ferait une lecture
/// posee directement dans la boucle - laisserait donc le compteur a zero, et le
/// battement du lecteur figé tout autant.
///
/// Aucun depot n'est connu ici : rien ne part sur le reseau, et les seuls
/// points ou la main se rend sont ceux de la lecture des fichiers.
#[tokio::test]
async fn le_depouillement_rend_la_main_entre_deux_journaux() {
    let bac = Bac::new();
    let journaux = bac.0.join("projects");
    std::fs::create_dir_all(&journaux).unwrap();

    for i in 0..300 {
        std::fs::write(
            journaux.join(format!("session-{i}.jsonl")),
            format!("{}\n", ligne("Write", "src/a.rs", "toolu_1", Utc::now())),
        )
        .unwrap();
    }

    // Un voisin qui ne fait qu'avancer, comme le battement du lecteur.
    let tours = Arc::new(AtomicUsize::new(0));
    let compteur = tours.clone();
    let voisin = tokio::spawn(async move {
        loop {
            compteur.fetch_add(1, Ordering::Relaxed);
            tokio::task::yield_now().await;
        }
    });

    let client = client_injoignable();
    let resume = depouiller(
        &client,
        &journaux,
        &BTreeMap::new(),
        horizon(Utc::now()),
        &mut Marque::default(),
    )
    .await;
    voisin.abort();

    assert_eq!(resume.journaux_depouilles, 300);
    assert!(
        tours.load(Ordering::Relaxed) >= 100,
        "le voisin n'a tourne que {} fois : le depouillement a garde la main",
        tours.load(Ordering::Relaxed)
    );
}

// ----------------------------------------------------------------------------
// Reprendre le depouillement la ou il s'est arrete (FR-049, FR-050, FR-078).
// ----------------------------------------------------------------------------

/// Sans marque, la reprise ne remonte pas au-dela des trente jours.
#[test]
fn sans_marque_seuls_les_trente_derniers_jours_restent_a_depouiller() {
    let maintenant = Utc::now();
    let journaux = vec![
        (
            PathBuf::from("dormant.jsonl"),
            maintenant - Duration::days(45),
        ),
        (
            PathBuf::from("dans-la-fenetre.jsonl"),
            maintenant - Duration::days(3),
        ),
        (
            PathBuf::from("frais.jsonl"),
            maintenant - Duration::hours(1),
        ),
    ];

    let restants = a_depouiller(&journaux, None, horizon(maintenant));

    assert_eq!(noms(&restants), ["dans-la-fenetre.jsonl", "frais.jsonl"]);
    assert_eq!(seuil(None, horizon(maintenant)), horizon(maintenant));
}

/// Un journal ecrit avant la marque a deja ete depouille : il ne revient pas.
#[test]
fn ce_qui_precede_la_marque_ne_revient_pas() {
    let maintenant = Utc::now();
    let marque = maintenant - Duration::hours(2);
    let journaux = vec![
        (
            PathBuf::from("deja-lu.jsonl"),
            maintenant - Duration::days(3),
        ),
        (
            PathBuf::from("depuis.jsonl"),
            maintenant - Duration::minutes(30),
        ),
    ];

    let restants = a_depouiller(&journaux, Some(marque), horizon(maintenant));

    assert_eq!(noms(&restants), ["depuis.jsonl"]);
}

/// Un journal ecrit depuis la marque revient, meme s'il avait deja ete lu.
///
/// C'est le rattrapage de ce qu'un agent lance au terminal a ajoute pendant que
/// l'application etait fermee (FR-078).
#[test]
fn un_journal_ecrit_depuis_la_marque_revient() {
    let maintenant = Utc::now();
    let marque = maintenant - Duration::hours(6);

    // Le meme journal, avant et apres que l'agent y ait ajoute des lignes.
    let avant = vec![(
        PathBuf::from("session.jsonl"),
        maintenant - Duration::days(2),
    )];
    let apres = vec![(
        PathBuf::from("session.jsonl"),
        maintenant - Duration::minutes(5),
    )];

    assert!(a_depouiller(&avant, Some(marque), horizon(maintenant)).is_empty());
    assert_eq!(
        noms(&a_depouiller(&apres, Some(marque), horizon(maintenant))),
        ["session.jsonl"]
    );
}

/// Un passage mene a son terme ne laisse rien a relire au suivant.
#[test]
fn deux_passages_de_suite_ne_rendent_pas_deux_fois_le_meme_journal() {
    let maintenant = Utc::now();
    let journaux = vec![
        (PathBuf::from("a.jsonl"), maintenant - Duration::days(4)),
        (PathBuf::from("b.jsonl"), maintenant - Duration::days(2)),
        (PathBuf::from("c.jsonl"), maintenant - Duration::hours(3)),
    ];

    let premier = a_depouiller(&journaux, None, horizon(maintenant));
    assert_eq!(premier.len(), 3);

    // La marque posee au terme du premier passage : l'instant ou il a commence.
    let second = a_depouiller(&journaux, Some(maintenant), horizon(maintenant));
    assert!(
        second.is_empty(),
        "des journaux reviennent deux fois : {second:?}"
    );
}

/// Une marque plus vieille que la fenetre ne fait pas remonter plus loin.
///
/// L'application est restee fermee six semaines : il n'y a rien a tirer d'un
/// journal dont aucune ligne ne tient dans les trente jours.
#[test]
fn une_marque_plus_vieille_que_la_fenetre_ne_fait_pas_remonter_plus_loin() {
    let maintenant = Utc::now();
    let marque = maintenant - Duration::days(60);
    let journaux = vec![
        (
            PathBuf::from("tres-vieux.jsonl"),
            marque + Duration::days(1),
        ),
        (
            PathBuf::from("recent.jsonl"),
            maintenant - Duration::days(2),
        ),
    ];

    let restants = a_depouiller(&journaux, Some(marque), horizon(maintenant));

    assert_eq!(noms(&restants), ["recent.jsonl"]);
    assert_eq!(
        seuil(Some(marque), horizon(maintenant)),
        horizon(maintenant)
    );
}

/// Les journaux reviennent du plus ancien ecrit au plus recent.
///
/// POURQUOI cet ordre compte : la marque avance au fil du passage. Reprendre
/// dans le desordre la ferait passer devant des journaux non lus, qui seraient
/// alors tenus pour depouilles.
///
/// Les noms vont ici a rebours des dates : l'ordre eprouve est bien celui des
/// ecritures, et non celui, gratuit, que le systeme de fichiers rend.
#[test]
fn les_journaux_reviennent_du_plus_ancien_au_plus_recent() {
    let maintenant = Utc::now();
    let journaux = vec![
        (PathBuf::from("a-tout-frais.jsonl"), maintenant),
        (
            PathBuf::from("c-le-plus-vieux.jsonl"),
            maintenant - Duration::days(9),
        ),
        (
            PathBuf::from("b-au-milieu.jsonl"),
            maintenant - Duration::days(4),
        ),
    ];

    let restants = a_depouiller(&journaux, None, horizon(maintenant));

    assert_eq!(
        noms(&restants),
        [
            "c-le-plus-vieux.jsonl",
            "b-au-milieu.jsonl",
            "a-tout-frais.jsonl"
        ]
    );
}

/// Une seconde ouverture ne relit pas ce qui a deja ete depouille (FR-078).
#[tokio::test]
async fn un_second_depouillement_ne_relit_pas_ce_qui_l_a_deja_ete() {
    let bac = Bac::new();
    let hier = Utc::now() - Duration::days(1);
    for i in 0..4 {
        bac.journal(&format!("session-{i}.jsonl"), hier + Duration::minutes(i));
    }

    let client = client_injoignable();
    let mut marque = Marque::charger(&bac.marque());
    let premier = depouiller(
        &client,
        &bac.journaux(),
        &carte(),
        horizon(Utc::now()),
        &mut marque,
    )
    .await;

    assert_eq!(premier.journaux_total, 4);
    assert_eq!(premier.journaux_a_depouiller, 4);
    assert_eq!(premier.depart(), 0, "le premier passage part de zero");
    assert_eq!(premier.journaux_depouilles, 4);
    assert_eq!(
        premier.en_defaut, 4,
        "les quatre journaux devaient etre lus, et leurs dates refusees par le client injoignable"
    );

    // Deuxieme ouverture : la marque se relit depuis le disque.
    let mut marque = Marque::charger(&bac.marque());
    let second = depouiller(
        &client,
        &bac.journaux(),
        &carte(),
        horizon(Utc::now()),
        &mut marque,
    )
    .await;

    assert_eq!(
        second.journaux_a_depouiller, 0,
        "des journaux ont ete relus"
    );
    assert_eq!(
        second.en_defaut, 0,
        "un journal deja depouille a ete rouvert"
    );
    assert_eq!(second.journaux_total, 4);
    assert_eq!(
        second.journaux_depouilles, 4,
        "l'avancement doit rester a quatre sur quatre, pas retomber a zero"
    );
}

/// Un journal auquel un agent a ajoute des lignes pendant la fermeture est relu.
#[tokio::test]
async fn un_journal_modifie_depuis_le_dernier_passage_est_relu() {
    let bac = Bac::new();
    let hier = Utc::now() - Duration::days(1);
    for i in 0..4 {
        bac.journal(&format!("session-{i}.jsonl"), hier + Duration::minutes(i));
    }

    let client = client_injoignable();
    let mut marque = Marque::charger(&bac.marque());
    depouiller(
        &client,
        &bac.journaux(),
        &carte(),
        horizon(Utc::now()),
        &mut marque,
    )
    .await;

    // Application fermee, un agent au terminal ecrit dans une zone.
    bac.journal("session-2.jsonl", Utc::now());

    let mut marque = Marque::charger(&bac.marque());
    let second = depouiller(
        &client,
        &bac.journaux(),
        &carte(),
        horizon(Utc::now()),
        &mut marque,
    )
    .await;

    assert_eq!(
        second.journaux_a_depouiller, 1,
        "seul le journal grossi devait revenir"
    );
    assert_eq!(second.en_defaut, 1, "le journal grossi n'a pas ete relu");
    assert_eq!(second.depart(), 3);
    assert_eq!(second.journaux_depouilles, 4);
    assert_eq!(second.journaux_total, 4);
}

/// Sans marque, un journal endormi depuis plus de trente jours n'est pas ouvert.
#[tokio::test]
async fn sans_marque_un_journal_endormi_n_est_pas_ouvert() {
    let bac = Bac::new();
    bac.journal("dormant.jsonl", Utc::now() - Duration::days(45));
    bac.journal("frais.jsonl", Utc::now() - Duration::hours(2));

    let resume = depouiller(
        &client_injoignable(),
        &bac.journaux(),
        &carte(),
        horizon(Utc::now()),
        &mut Marque::charger(&bac.marque()),
    )
    .await;

    assert_eq!(resume.journaux_total, 2);
    assert_eq!(resume.journaux_a_depouiller, 1);
    assert_eq!(resume.en_defaut, 1, "le journal dormant a ete ouvert");
    assert_eq!(resume.journaux_depouilles, 2);
}

/// Un depouillement interrompu repart de son avancement, jamais de zero.
///
/// Le scenario du critere d'acceptation, a l'echelle : quatre cents journaux,
/// l'application fermee au cent-vingtieme. Chaque journal est ecrit une minute
/// apres le precedent, ce qui donne a l'avancement un repere exact - la marque
/// est une date, et cette date designe un rang.
///
/// L'interruption ne se joue pas sur un delai, qui rendrait le test capricieux :
/// une tache voisine lit la marque sur le disque et coupe des qu'elle a passe le
/// cent-vingt-et-unieme journal.
///
/// Les noms vont a rebours des dates d'ecriture : une reprise qui suivrait
/// l'ordre du systeme de fichiers ferait sauter la marque par-dessus des
/// journaux jamais lus, et c'est ce que la borne haute de l'avancement dit.
#[tokio::test]
async fn un_depouillement_interrompu_repart_de_son_avancement() {
    let bac = Bac::new();
    let premiere_ecriture = Utc::now() - Duration::days(2);
    for i in 0..400 {
        bac.journal(
            &format!("session-{:03}.jsonl", 399 - i),
            premiere_ecriture + Duration::minutes(i),
        );
    }

    let chemin_marque = bac.marque();
    let dossier = bac.journaux();
    let sans_depot = BTreeMap::new();
    let client = client_injoignable();
    let mut marque = Marque::charger(&chemin_marque);

    let cent_vingt_et_unieme = premiere_ecriture + Duration::minutes(120);
    let fermeture = async {
        loop {
            let vue = Marque::charger(&chemin_marque);
            if vue
                .depouille_jusqu_a
                .is_some_and(|jusqu_a| jusqu_a >= cent_vingt_et_unieme)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    };

    tokio::select! {
        _ = depouiller(&client, &dossier, &sans_depot, horizon(Utc::now()), &mut marque) =>
            panic!("le depouillement devait etre interrompu avant son terme"),
        _ = fermeture => {}
    }

    let interrompue = Marque::charger(&chemin_marque);
    assert!(
        interrompue.depouille_jusqu_a.is_some(),
        "un depouillement interrompu a tout perdu"
    );
    assert_eq!(
        interrompue.termine_a, None,
        "un passage interrompu ne doit pas se dire termine"
    );

    // Reouverture.
    let mut marque = Marque::charger(&chemin_marque);
    let reprise = depouiller(
        &client,
        &dossier,
        &sans_depot,
        horizon(Utc::now()),
        &mut marque,
    )
    .await;

    assert_eq!(reprise.journaux_total, 400);
    assert!(
        reprise.depart() >= 120,
        "l'avancement repart de {} sur 400, au lieu de 120 au moins",
        reprise.depart()
    );
    assert!(
        reprise.journaux_a_depouiller <= 280,
        "{} journaux a reprendre : le depouillement recommence au lieu de reprendre",
        reprise.journaux_a_depouiller
    );
    // Et pas davantage : une marque qui aurait saute par-dessus des journaux non
    // lus les tiendrait pour depouilles, et personne ne les relirait jamais.
    assert!(
        reprise.depart() < 200,
        "l'avancement repart de {} sur 400 alors que l'interruption etait au 120e : \
         la marque a devance ce qui avait ete lu",
        reprise.depart()
    );
    assert_eq!(
        reprise.journaux_depouilles, 400,
        "le passage de reprise doit mener l'avancement au total"
    );
}
