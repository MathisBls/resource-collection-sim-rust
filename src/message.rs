//! Messages envoyés par les robots vers le thread de base (communication
//! asynchrone via un canal mpsc).

use crate::types::{Pos, ResourceKind};

pub enum Message {
    /// Un éclaireur (ou collecteur) a repéré une ressource.
    FoundResource { pos: Pos, kind: ResourceKind, qty: u32 },
    /// Découverte d'un obstacle jusque-là inconnu.
    FoundObstacle { pos: Pos },
    /// Un collecteur réserve une ressource comme cible.
    Claim { id: usize, pos: Pos },
    /// La ressource est épuisée, on la retire de la connaissance.
    Deplete { pos: Pos },
    /// Un collecteur a déchargé une unité à la base.
    Collected { kind: ResourceKind, amount: u32 },
}
