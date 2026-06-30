//! Thread de la base : unique écrivain de la connaissance globale.
//! Il agrège en continu les découvertes des robots reçues par le canal mpsc
//! et met à jour les compteurs de ressources collectées.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use crate::knowledge::{Knowledge, KnownResource};
use crate::map::World;
use crate::message::Message;
use crate::types::ResourceKind;

pub fn run_base(
    rx: Receiver<Message>,
    world: Arc<Mutex<World>>,
    knowledge: Arc<RwLock<Knowledge>>,
    running: Arc<AtomicBool>,
) {
    while running.load(Ordering::Relaxed) {
        // recv_timeout pour rester réactif à l'arrêt sans bloquer indéfiniment.
        let msg = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(m) => m,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        match msg {
            Message::FoundResource { pos, kind, qty } => {
                let mut k = knowledge.write().unwrap_or_else(|e| e.into_inner());
                k.obstacles.remove(&pos);
                let entry = k.resources.entry(pos).or_insert(KnownResource {
                    kind,
                    qty,
                    claimed_by: None,
                    depleted: false,
                });
                entry.kind = kind;
                entry.qty = qty;
                if qty == 0 {
                    entry.depleted = true;
                }
            }
            Message::FoundObstacle { pos } => {
                let mut k = knowledge.write().unwrap_or_else(|e| e.into_inner());
                k.obstacles.insert(pos);
            }
            Message::Claim { id, pos } => {
                let mut k = knowledge.write().unwrap_or_else(|e| e.into_inner());
                if let Some(r) = k.resources.get_mut(&pos) {
                    if r.claimed_by.is_none() {
                        r.claimed_by = Some(id);
                    }
                }
            }
            Message::Deplete { pos } => {
                let mut k = knowledge.write().unwrap_or_else(|e| e.into_inner());
                if let Some(r) = k.resources.get_mut(&pos) {
                    r.depleted = true;
                    r.qty = 0;
                    r.claimed_by = None;
                }
            }
            Message::Collected { kind, amount } => {
                let mut w = world.lock().unwrap_or_else(|e| e.into_inner());
                match kind {
                    ResourceKind::Energy => w.collected_energy += amount,
                    ResourceKind::Crystal => w.collected_crystals += amount,
                }
            }
        }
    }
}
