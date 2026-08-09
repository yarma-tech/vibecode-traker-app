//! Tenir le coup quand le reseau tombe.
//!
//! Deux petites machines, sans dependance au reseau ni au temps reel :
//!
//!   `FileAttente`  une file bornee : ce qui n'a pas pu partir attend ici, et si
//!                  elle deborde, c'est le plus ancien qui tombe, jamais le plus
//!                  recent. Le journal sur disque reste la memoire durable : un
//!                  element jete y demeure, et un redemarrage le relira.
//!
//!   `Backoff`      la temporisation de renvoi : elle double a chaque echec et se
//!                  plafonne, pour ne jamais marteler Supabase. Un succes la remet
//!                  a son pas de depart.

use std::collections::VecDeque;
use std::time::Duration;

/// Ce qu'un `pousser` a du sacrifier pour tenir dans le plafond.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Debordement<T> {
    /// Il restait de la place : rien n'a ete jete.
    Aucun,
    /// La file etait pleine : cet element, le plus ancien, est tombe.
    Jete(T),
}

/// Une file d'attente bornee, qui jette le plus ancien quand elle deborde.
///
/// Le plafond est un choix assume : sans lui, une coupure longue ferait enfler
/// la memoire sans fin. Jeter le plus ancien plutot que le plus recent garde a
/// l'ecran l'etat le plus proche du present.
#[derive(Debug)]
pub struct FileAttente<T> {
    elements: VecDeque<T>,
    plafond: usize,
}

impl<T> FileAttente<T> {
    /// Une file d'au plus `plafond` elements. Un plafond de zero est ramene a un :
    /// une file qui ne peut rien tenir ne rendrait aucun service.
    pub fn new(plafond: usize) -> Self {
        Self {
            elements: VecDeque::new(),
            plafond: plafond.max(1),
        }
    }

    /// Ajoute un element. Si la file etait pleine, rend le plus ancien, jete.
    pub fn pousser(&mut self, element: T) -> Debordement<T> {
        let debordement = if self.elements.len() >= self.plafond {
            self.elements
                .pop_front()
                .map(Debordement::Jete)
                .unwrap_or(Debordement::Aucun)
        } else {
            Debordement::Aucun
        };
        self.elements.push_back(element);
        debordement
    }

    /// Retire et rend le plus ancien, ou `None` si la file est vide.
    pub fn tirer(&mut self) -> Option<T> {
        self.elements.pop_front()
    }

    /// Regarde le plus ancien sans le retirer : de quoi tenter un envoi avant de
    /// decider de le sortir de la file.
    pub fn premier(&self) -> Option<&T> {
        self.elements.front()
    }

    // La file expose `est_vide` (son nom francais) au lieu de `is_empty` : le
    // reste du code parle francais, on ne fait pas d'exception pour un lint.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Le nom francais de `is_empty`.
    pub fn est_vide(&self) -> bool {
        self.elements.is_empty()
    }
}

/// La temporisation de renvoi : exponentielle, plafonnee.
///
/// A chaque echec, l'attente double, jusqu'a un plafond qu'elle ne franchit
/// jamais. Un succes la remet a son pas de depart. Elle ne dort pas elle-meme :
/// elle rend une duree, et c'est l'appelant qui la respecte.
#[derive(Debug)]
pub struct Backoff {
    depart: Duration,
    plafond: Duration,
    prochain: Duration,
}

impl Backoff {
    pub fn new(depart: Duration, plafond: Duration) -> Self {
        Self {
            depart,
            plafond,
            prochain: depart,
        }
    }

    /// Rend l'attente a respecter, puis prepare la suivante (le double, plafonne).
    pub fn prochain(&mut self) -> Duration {
        let actuel = self.prochain.min(self.plafond);
        self.prochain = (actuel * 2).min(self.plafond);
        actuel
    }

    /// Un envoi a reussi : on repart au pas de depart.
    pub fn reset(&mut self) {
        self.prochain = self.depart;
    }
}
