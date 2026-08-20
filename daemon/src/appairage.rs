//! Ce qu'une machine sait d'elle-meme une fois reliee.
//!
//! Ce module portait l'echange d'un code d'appairage contre un jeton. Ce chemin
//! est ferme (FR-082) : plus rien ne l'emprunte, ni l'ecran ni le binaire, et
//! l'echange a disparu d'ici. La table des codes et la fonction `appairer_machine`
//! restent en base - leur retrait est un nettoyage de schema, hors scope - mais
//! aucun appelant ne les atteint plus depuis le poste.
//!
//! Ce qui survit, c'est la forme : une machine reliee connait son identifiant,
//! son nom et son jeton, quelle que soit la porte par laquelle elle est entree.
//! `declaration.rs`, qui est la porte d'aujourd'hui (FR-017), rend cette meme
//! structure.

use serde::Deserialize;

/// Ce qu'une machine sait d'elle-meme une fois reliee au compte.
#[derive(Debug, Deserialize)]
pub struct Identite {
    pub machine_id: String,
    pub label: String,
    /// Ne finit jamais dans un fichier : il va au trousseau du systeme.
    pub token: String,
}
