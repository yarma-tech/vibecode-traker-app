//! Ce que le depouillement du passe tire d'un journal, et ce qu'il n'en tire pas.
//!
//! Rien ici ne parle au reseau : le depouillement d'un journal est une fonction
//! pure sur du texte, comme sa cousine `journal::lire`. La propriete centrale de
//! la tranche - « du passe, mais aucune couleur » - s'eprouve de l'exterieur,
//! contre la vraie base, dans `passe_sans_couleur.rs`.

use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use vibemap::depouillement::{
    self, chemin_de_la_marque, depouiller, horizon, touches_du_journal, Marque, FENETRE_JOURS,
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
}

impl Drop for Bac {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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

    // Le depouillement pose sa marque.
    let mut marque = Marque::charger(&chemin_marque);
    marque.poser(Utc::now(), 12);
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
    assert!(relue.termine_a.is_some());
}

/// Une marque absente ne fait pas paniquer : elle est simplement vierge.
#[test]
fn une_marque_absente_rend_une_marque_vierge() {
    let bac = Bac::new();
    let marque = Marque::charger(&bac.0.join("jamais-ecrite.json"));

    assert_eq!(marque.termine_a, None);
    assert_eq!(marque.journaux, 0);
}

/// Une marque corrompue non plus : un demarrage refuse serait pire.
#[test]
fn une_marque_corrompue_ne_fait_pas_paniquer() {
    let bac = Bac::new();
    let chemin = bac.0.join("depouillement.json");
    std::fs::write(&chemin, "{ a moitie ecr").unwrap();

    let marque = Marque::charger(&chemin);

    assert_eq!(marque.termine_a, None);
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

    let client = vibemap::Supabase::new("http://127.0.0.1:1", "jeton-de-test");
    let resume = depouiller(&client, &journaux, &BTreeMap::new(), horizon(Utc::now())).await;
    voisin.abort();

    assert_eq!(resume.journaux_depouilles, 300);
    assert!(
        tours.load(Ordering::Relaxed) >= 100,
        "le voisin n'a tourne que {} fois : le depouillement a garde la main",
        tours.load(Ordering::Relaxed)
    );
}
