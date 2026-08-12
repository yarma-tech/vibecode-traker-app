//! Lire un PRD et en tirer ses blocs (issue #36 pour l'exploration d'un
//! brouillon - FR-036 a FR-038, FR-043, FR-044 ; issue #37 pour la
//! conversion d'un document `validé` en ses features - FR-039 a FR-042 ;
//! conception §6.
//!
//! Cadence de la boucle de cartographie (300 s) : un PRD ne change pas
//! toutes les trente secondes, `commits.rs` a deja sa cadence propre pour ce
//! qui va plus vite.
//!
//! **Aucun modele ici.** Le parseur est une fonction pure et deterministe du
//! texte du document : un modele reformulerait a chaque passage et
//! fabriquerait des doublons a chaque relecture (conception §6, PRD §6.2).
//! C'est aussi ce qui rend `analyser` verifiable sur de simples chaines, sans
//! reseau ni disque - `daemon/tests/prd.rs` l'eprouve sur fixtures.

use std::path::{Path, PathBuf};
use std::process::Command;

/// L'en-tete YAML d'un PRD. FR-036 n'exige que quatre champs pour qu'un
/// document soit reconnu ; `titre` est lu s'il existe mais jamais requis
/// (repli sur `id`, voir `EnTete::depuis_champs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnTete {
    pub id: String,
    pub statut: String,
    pub date: String,
    pub repo: String,
    pub titre: String,
    pub valide_le: Option<String>,
    pub maj: Option<String>,
}

/// Une section `### Fn` du corps. Volontairement pauvre : FR-043 interdit de
/// transmettre au-dela de la cle, du titre, de la priorite et du marqueur
/// « a clarifier » - ce type n'a nulle part ou loger une user story, une
/// exigence ou un critere d'acceptation. Le parseur les traverse (il doit
/// lire la section entiere pour y chercher `[À CLARIFIER]`) sans jamais les
/// recopier dans un champ : `daemon/tests/prd.rs` le prouve sur une section
/// qui contient les trois, en verifiant qu'aucun des quatre champs ci-dessous
/// n'en porte la moindre trace.
///
/// `Serialize` (issue #37) : cette meme pauvrete est ce qui part vers
/// `rpc/convertir_prd_valide` - la contrainte de typage vaut garde-fou, rien
/// de plus que ces quatre champs ne peut jamais s'y glisser.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Feature {
    pub cle: String,
    pub titre: String,
    pub priorite: Option<String>,
    pub a_clarifier: bool,
}

/// Un document lu : son en-tete, et les features qu'il porte (utilisees ici
/// uniquement pour savoir si le document en reconnait au moins une, FR-044 -
/// leur conversion en blocs est #37).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub entete: EnTete,
    pub features: Vec<Feature>,
}

/// Lit l'en-tete YAML et le corps d'un document. `None` si l'en-tete ne
/// porte pas au moins `id`, `statut`, `date` et `repo` (FR-036) : le
/// document est alors ignore avant meme d'exister pour le reste du daemon,
/// sans un mot a l'ecran - un markdown ordinaire (README, ADR, spec) en
/// porte des dizaines dans ce depot, ce n'est pas une erreur.
pub fn analyser(contenu: &str) -> Option<Document> {
    let (champs, corps) = entete_et_corps(contenu)?;
    let entete = EnTete::depuis_champs(&champs)?;
    let features = features_du_corps(corps, &entete.date, &entete.id);
    Some(Document { entete, features })
}

impl EnTete {
    fn depuis_champs(champs: &std::collections::HashMap<String, String>) -> Option<Self> {
        let requis = |cle: &str| champs.get(cle).filter(|v| !v.is_empty()).cloned();

        let id = requis("id")?;
        let statut = requis("statut")?;
        let date = requis("date").filter(|d| date_valide(d))?;
        let repo = requis("repo")?;

        // `titre` n'est pas dans la liste de FR-036 : sa seule consequence
        // ici est le titre du bloc d'exploration (« cadrage <titre> »), pas
        // la reconnaissance du document. Un document qui l'omet retombe sur
        // son `id`, deterministe et toujours present plutot que de deviner
        // un titre en lisant le corps.
        let titre = requis("titre").unwrap_or_else(|| id.clone());

        let valide_le = requis("valide_le").filter(|d| date_valide(d));
        let maj = requis("maj").filter(|d| date_valide(d));

        Some(Self { id, statut, date, repo, titre, valide_le, maj })
    }
}

fn date_valide(valeur: &str) -> bool {
    chrono::NaiveDate::parse_from_str(valeur, "%Y-%m-%d").is_ok()
}

/// Separe l'en-tete YAML (entre les deux `---`) du reste du document, et
/// rend les champs `cle: valeur` qu'il porte. `None` si le document ne
/// commence pas par `---` ou si le second delimiteur manque (en-tete
/// tronque) - dans les deux cas, `analyser` doit ignorer le document, pas
/// paniquer sur un fichier a moitie ecrit.
fn entete_et_corps(contenu: &str) -> Option<(std::collections::HashMap<String, String>, &str)> {
    let apres_premier = contenu.strip_prefix("---")?;
    let apres_saut = apres_premier
        .strip_prefix("\r\n")
        .or_else(|| apres_premier.strip_prefix('\n'))?;

    // Le second delimiteur doit etre seul sur sa ligne : un simple `find`
    // de "\n---" suffit, un YAML plus riche (bloc, liste) ne remet jamais
    // trois tirets en debut de ligne a l'interieur d'un en-tete de PRD.
    let fin = apres_saut.find("\n---")?;
    let bloc = &apres_saut[..fin];
    let corps = &apres_saut[fin + 4..];

    let mut champs = std::collections::HashMap::new();
    for ligne in bloc.lines() {
        let Some((cle, valeur)) = ligne.split_once(':') else { continue };
        let cle = cle.trim();
        let valeur = sans_guillemets(valeur.trim());
        if !cle.is_empty() && !valeur.is_empty() {
            champs.insert(cle.to_string(), valeur);
        }
    }

    Some((champs, corps))
}

fn sans_guillemets(valeur: &str) -> String {
    let sans = valeur
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| valeur.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(valeur);
    sans.trim().to_string()
}

/// Les sections `### Fn — titre (Priorité : Pn)` du corps. Une ligne qui
/// commence par `### F` sans chiffre a la suite n'est pas reconnue comme une
/// feature : le parseur ne devine rien, il reconnait un gabarit precis
/// (conception §6, point 4) ou passe son chemin.
fn features_du_corps(corps: &str, date: &str, id: &str) -> Vec<Feature> {
    let lignes: Vec<&str> = corps.lines().collect();
    let mut resultats = Vec::new();
    let mut i = 0;

    while i < lignes.len() {
        let Some((numero, reste)) = entete_de_feature(lignes[i]) else {
            i += 1;
            continue;
        };

        // La section court jusqu'a la prochaine ligne de titre (n'importe
        // quel niveau), ou la fin du document. C'est elle qu'on parcourt
        // pour `[À CLARIFIER]`, sans jamais en garder le texte (FR-043).
        let debut = i + 1;
        let fin = lignes[debut..]
            .iter()
            .position(|l| l.trim_start().starts_with('#'))
            .map(|d| debut + d)
            .unwrap_or(lignes.len());

        let a_clarifier = lignes[debut..fin]
            .iter()
            .any(|l| l.contains("[À CLARIFIER]"));

        let (titre, priorite) = titre_et_priorite(reste);

        resultats.push(Feature {
            cle: format!("{date}/{id}/F{numero}"),
            titre,
            priorite,
            a_clarifier,
        });

        i = fin;
    }

    resultats
}

/// `### F12 — ...` -> `Some(("12", " — ..."))`. `### Fabuleux` -> `None`,
/// faute de chiffre apres le `F`.
fn entete_de_feature(ligne: &str) -> Option<(&str, &str)> {
    let reste = ligne.strip_prefix("### F")?;
    let fin_numero = reste.find(|c: char| !c.is_ascii_digit()).unwrap_or(reste.len());
    if fin_numero == 0 {
        return None;
    }
    Some((&reste[..fin_numero], &reste[fin_numero..]))
}

/// Separe le titre de la priorite dans le reste d'une ligne `### Fn ...`.
/// Le separateur attendu est le tiret cadratin qu'emploient les PRD de ce
/// depot (`—`, U+2014) ; un simple tiret est accepte en repli, au cas ou un
/// document l'ecrit autrement - dans les deux cas c'est une donnee qu'on
/// LIT, pas une regle qu'on impose a l'auteur du PRD.
fn titre_et_priorite(reste: &str) -> (String, Option<String>) {
    let reste = reste.trim_start();
    let sans_tiret = reste
        .strip_prefix('\u{2014}')
        .or_else(|| reste.strip_prefix('-'))
        .unwrap_or(reste)
        .trim_start();

    match sans_tiret.rfind("(Priorité") {
        Some(position) => {
            let titre = sans_tiret[..position].trim().to_string();
            let priorite = sans_tiret[position..]
                .split_once(':')
                .map(|(_, apres)| apres.trim().trim_end_matches(')').trim().to_string())
                .filter(|p| !p.is_empty());
            (titre, priorite)
        }
        None => (sans_tiret.trim().to_string(), None),
    }
}

/// Le nom logique d'un depot, tel qu'un en-tete de PRD le designe
/// (`repo: vibecode-traker-app`) : le dernier segment de l'identite distante
/// quand elle existe - elle survit a un renommage de dossier, contrairement
/// au nom du dossier lui-meme (conception, section 3) -, le nom du dossier
/// sinon, seule identite qu'un depot sans distant possede.
pub fn nom_logique(plan: &crate::Plan) -> String {
    if plan.identity.starts_with("local:") {
        plan.name.clone()
    } else {
        plan.identity.rsplit('/').next().unwrap_or(&plan.name).to_string()
    }
}

/// Les fichiers `*.md` du depot, comme `git` les voit - suivis ou nouveaux
/// mais pas ignores, meme convention que `plan::fichiers_du_depot` : un
/// `.gitignore` qui ecarte un brouillon de PRD vaut aussi pour ce lecteur.
/// Aucune restriction de dossier : FR-036 reconnait un document a son
/// en-tete, pas a son emplacement.
fn markdowns_du_depot(racine: &Path) -> Vec<PathBuf> {
    let sortie = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "*.md"])
        .current_dir(racine)
        .output();

    let Ok(sortie) = sortie else { return Vec::new() };
    if !sortie.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&sortie.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Les champs qu'un bloc d'exploration porte, groupes pour l'appel a
/// `Supabase::creer_bloc_exploration_prd` - voir son commentaire pour le
/// pourquoi du regroupement (clippy, `too_many_arguments`).
pub struct ExplorationPrd<'a> {
    pub titre: &'a str,
    pub chemin: &'a str,
    pub prd_cle: &'a str,
    pub prd_statut: &'a str,
    pub prd_maj: Option<&'a str>,
    pub prd_valide_le: Option<&'a str>,
}

/// Les champs qu'une conversion `validé` porte, groupes pour l'appel a
/// `Supabase::convertir_prd_valide` - meme raison que `ExplorationPrd`
/// (clippy, `too_many_arguments`).
pub struct ConversionPrd<'a> {
    /// La cle du DOCUMENT (`<date>/<id>`), celle de son eventuelle
    /// exploration - jamais celle d'une feature.
    pub doc_prd_cle: &'a str,
    pub prd_statut: &'a str,
    pub prd_maj: Option<&'a str>,
    pub prd_valide_le: Option<&'a str>,
    pub features: &'a [Feature],
}

/// Ce qu'une lecture de PRD a produit sur un depot - a l'appelant de
/// l'afficher (comme `cartographier` et `ingerer_commits` le font deja pour
/// leur propre resume dans `main.rs`), jamais a `traiter` de decider tout
/// seul ce qui merite l'ecran : c'est ce qui rend FR-044 verifiable par un
/// test plutot que seulement par un commentaire.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Resume {
    /// Blocs d'exploration poses ou retrouves (l'appel est idempotent : un
    /// document deja lu compte ici sans rien creer de neuf).
    pub blocs_poses: usize,
    /// FR-039/FR-040 (#37) : features NEUVES posees a cette lecture - une
    /// feature deja connue dont seul le titre a change n'y entre pas, elle
    /// est mise a jour en place, jamais recomptee comme creation.
    pub features_creees: usize,
    /// FR-041 (#37) : features que CE passage vient de marquer `prd_absent`
    /// (pas le total cumule en base, seulement ce qui vient de disparaitre).
    pub features_absentes: usize,
    /// FR-044 : chemins des documents dont l'en-tete est reconnu, mais dont
    /// aucune feature ne l'est.
    pub sans_feature_reconnue: Vec<PathBuf>,
    /// Decision de #37 sur les `id` de PRD dupliques : deux documents
    /// `validé` qui partagent la meme cle de document (date, id)
    /// produiraient des features sous les MEMES `prd_cle` - le second
    /// ecraserait ou serait absorbe en silence par le premier. Plutot que de
    /// deviner lequel des deux fait foi (PRD, hypotheses : "on corrige le
    /// document, on ne devine pas"), seul le premier document rencontre dans
    /// CE passage (ordre alphabetique stable de `markdowns_du_depot`) est
    /// converti ; les suivants atterrissent ici, signales a l'ecran comme
    /// `sans_feature_reconnue` l'est deja pour FR-044.
    pub cles_dupliquees: Vec<PathBuf>,
}

/// Lit les PRD d'un depot et publie ce que chaque document decrit : le bloc
/// d'exploration d'un brouillon (#36, FR-038), ou la conversion en features
/// d'un document `validé` (#37, FR-039 a FR-042), ou le retrait silencieux
/// (`prd_absent`) des blocs d'un document `abandonné`. Un document qui
/// echoue a s'ecrire n'arrete pas les autres, comme le reste de la
/// cartographie.
pub async fn traiter(
    client: &crate::Supabase,
    racine: &Path,
    repo_id: &str,
    plan: &crate::Plan,
) -> Resume {
    let nom_du_depot = nom_logique(plan);
    let mut resume = Resume::default();

    // Cles de DOCUMENT (`<date>/<id>`) deja converties pendant CE passage -
    // voir le commentaire de `Resume::cles_dupliquees` pour le raisonnement
    // complet sur les id dupliques.
    let mut cles_validees_ce_passage: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for chemin_relatif in markdowns_du_depot(racine) {
        let Ok(contenu) = std::fs::read_to_string(racine.join(&chemin_relatif)) else {
            continue;
        };

        let Some(document) = analyser(&contenu) else {
            continue; // FR-036 : en-tete non conforme, ignore sans bruit.
        };

        if document.entete.repo != nom_du_depot {
            continue; // FR-037 : ce document est ecrit pour un autre depot.
        }

        if document.features.is_empty() {
            resume.sans_feature_reconnue.push(chemin_relatif.clone());
        }

        // Cle du DOCUMENT, jamais celle d'une feature (`<date>/<id>/Fn`,
        // FR-040) : sans le `/Fn`, elle ne collisionne avec aucune d'elles,
        // et la meme paire (date, id) identifie deja le document.
        let prd_cle_document = format!("{}/{}", document.entete.date, document.entete.id);

        match document.entete.statut.as_str() {
            // FR-038 : un brouillon laisse un bloc, et un seul.
            "draft" => {
                let chemin_texte = chemin_relatif.to_string_lossy().replace('\\', "/");
                let titre = format!("cadrage {}", document.entete.titre);

                let doc = ExplorationPrd {
                    titre: &titre,
                    chemin: &chemin_texte,
                    prd_cle: &prd_cle_document,
                    prd_statut: &document.entete.statut,
                    prd_maj: document.entete.maj.as_deref(),
                    prd_valide_le: document.entete.valide_le.as_deref(),
                };

                match client.creer_bloc_exploration_prd(repo_id, &doc).await {
                    // Une ligne SQL NULL de type composite (`return null;`
                    // cote fonction) n'arrive PAS ici comme un scalaire JSON
                    // `null` : PostgREST la rend comme un OBJET dont chaque
                    // colonne vaut `null` (constate a l'usage - `to_json`
                    // d'un composite NULL suit la meme regle que ROW(NULL,
                    // NULL, ...)). `id` est la seule colonne NOT NULL sans
                    // defaut absent : une vraie ligne l'a toujours, une
                    // ligne NULL ne l'a jamais. C'est ce garde-fou (#37,
                    // regression `validé` -> `draft`) qui produit ce cas :
                    // une conversion a deja produit des features pour cette
                    // cle de document, rien de neuf n'a ete pose.
                    Ok(valeur) if !valeur["id"].is_null() => resume.blocs_poses += 1,
                    Ok(_) => {}
                    Err(erreur) => {
                        eprintln!("PRD {} non publie : {erreur}", chemin_relatif.display());
                    }
                }
            }

            // FR-039 a FR-042 (#37) : l'exploration cede la place aux
            // features, dans la meme transaction cote base.
            "validé" => {
                if document.features.is_empty() {
                    // Deja signale ci-dessus (FR-044) : un document sans
                    // feature reconnue ne convertit rien - ni l'exploration
                    // (elle pourrait porter un travail humain), ni quoi que
                    // ce soit d'autre. On ne devine jamais a partir d'un
                    // document illisible.
                    continue;
                }

                if !cles_validees_ce_passage.insert(prd_cle_document.clone()) {
                    resume.cles_dupliquees.push(chemin_relatif.clone());
                    continue;
                }

                let doc = ConversionPrd {
                    doc_prd_cle: &prd_cle_document,
                    prd_statut: &document.entete.statut,
                    prd_maj: document.entete.maj.as_deref(),
                    prd_valide_le: document.entete.valide_le.as_deref(),
                    features: &document.features,
                };

                match client.convertir_prd_valide(repo_id, &doc).await {
                    Ok(resultat) => {
                        resume.features_creees += resultat.features_creees;
                        resume.features_absentes += resultat.features_absentes;
                    }
                    Err(erreur) => {
                        eprintln!("PRD {} non converti : {erreur}", chemin_relatif.display());
                    }
                }
            }

            // `statut: abandonné` : plus de creation, les blocs deja issus
            // de ce document passent `prd_absent`. Idempotent par
            // construction (`marquer_prd_absent` ne touche que ce qui a
            // change), une collision d'id ici est donc sans consequence -
            // contrairement a `validé`, aucun garde-fou n'est necessaire.
            "abandonné" => {
                if let Err(erreur) = client
                    .marquer_prd_absent(
                        repo_id,
                        &prd_cle_document,
                        &document.entete.statut,
                        document.entete.maj.as_deref(),
                    )
                    .await
                {
                    eprintln!("PRD {} non marque absent : {erreur}", chemin_relatif.display());
                }
            }

            // Un statut hors du vocabulaire du produit (ni draft, ni
            // validé, ni abandonné) : ni exploration, ni conversion, ni
            // retrait. On ne devine jamais ce qu'un statut inconnu voudrait
            // dire (meme principe que #36 pour un en-tete non conforme).
            _ => {}
        }
    }

    resume
}
