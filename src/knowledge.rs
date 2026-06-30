//! Connaissance globale agrégée par la base à partir des messages des robots.
//! Les robots la lisent (RwLock en lecture) pour naviguer ; seul le thread de
//! base y écrit, ce qui en fait le hub de connaissances de la simulation.

use std::collections::{HashMap, HashSet};

use crate::types::{Pos, ResourceKind};

pub struct KnownResource {
    pub kind: ResourceKind,
    pub qty: u32,
    pub claimed_by: Option<usize>,
    pub depleted: bool,
}

#[derive(Default)]
pub struct Knowledge {
    pub resources: HashMap<Pos, KnownResource>,
    pub obstacles: HashSet<Pos>,
}

impl Knowledge {
    /// Cible disponible la plus proche pour un collecteur donné :
    /// ressource non épuisée, non réclamée (ou déjà réclamée par lui).
    pub fn nearest_available(&self, from: Pos, id: usize) -> Option<Pos> {
        self.resources
            .iter()
            .filter(|(_, r)| !r.depleted && r.qty > 0)
            .filter(|(_, r)| r.claimed_by.is_none() || r.claimed_by == Some(id))
            .min_by_key(|(p, _)| from.manhattan(**p))
            .map(|(p, _)| *p)
    }
}
