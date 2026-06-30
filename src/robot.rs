//! Comportements des robots. Chaque robot tourne dans son propre thread,
//! lit la connaissance partagée (RwLock) pour naviguer, agit brièvement sur le
//! monde (Mutex) et communique ses découvertes à la base via un canal mpsc.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use rand::seq::SliceRandom;
use rand::Rng;

use crate::knowledge::Knowledge;
use crate::map::World;
use crate::message::Message;
use crate::path;
use crate::types::{Pos, ResourceKind, RobotKind};

struct State {
    id: usize,
    pos: Pos,
    carrying: Option<ResourceKind>,
    target: Option<Pos>,
    /// Direction courante de l'éclaireur (dx, dy), pour une exploration
    /// "avec élan" qui couvre plus de terrain qu'une marche aléatoire pure.
    dir: (i32, i32),
}

pub fn run_robot(
    id: usize,
    kind: RobotKind,
    start: Pos,
    world: Arc<Mutex<World>>,
    knowledge: Arc<RwLock<Knowledge>>,
    tx: Sender<Message>,
    running: Arc<AtomicBool>,
) {
    let mut rng = rand::thread_rng();
    let tick = Duration::from_millis(rng.gen_range(90..140));
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let mut st = State {
        id,
        pos: start,
        carrying: None,
        target: None,
        dir: dirs[rng.gen_range(0..4)],
    };

    while running.load(Ordering::Relaxed) {
        match kind {
            RobotKind::Scout => scout_tick(&mut st, &world, &knowledge, &tx, &mut rng),
            RobotKind::Collector => collector_tick(&mut st, &world, &knowledge, &tx, &mut rng),
        }
        thread::sleep(tick);
    }
}

/// Éclaireur : explore au hasard en évitant les obstacles connus, repère et
/// diffuse ressources et obstacles. Ne collecte jamais.
fn scout_tick(
    st: &mut State,
    world: &Arc<Mutex<World>>,
    knowledge: &Arc<RwLock<Knowledge>>,
    tx: &Sender<Message>,
    rng: &mut impl Rng,
) {
    let known_obstacles = knowledge.read().unwrap_or_else(|e| e.into_inner()).obstacles.clone();

    let mut msgs = Vec::new();
    {
        let mut w = world.lock().unwrap_or_else(|e| e.into_inner());
        sense(&w, st.pos, &mut msgs);

        // Exploration avec élan : continue dans la direction courante tant
        // qu'elle est franchissable ; sinon, choisit une nouvelle direction
        // libre au hasard. Couvre beaucoup plus de terrain qu'une marche
        // purement aléatoire et pousse les éclaireurs vers les bords.
        let ahead = Pos::new(st.pos.x + st.dir.0, st.pos.y + st.dir.1);
        let walkable =
            |p: Pos| !w.is_obstacle(p) && !known_obstacles.contains(&p) && !w.is_occupied(p, st.id);
        // 15 % du temps on change quand même de cap, pour ne pas longer
        // indéfiniment un même couloir.
        if walkable(ahead) && rng.gen_bool(0.85) {
            st.pos = ahead;
        } else {
            let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            let mut choices: Vec<(i32, i32)> = dirs
                .into_iter()
                .filter(|&(dx, dy)| walkable(Pos::new(st.pos.x + dx, st.pos.y + dy)))
                .collect();
            if choices.is_empty() {
                // Encerclé par des obstacles connus : autorise les cases
                // réellement libres pour ne pas rester figé.
                choices = dirs
                    .into_iter()
                    .filter(|&(dx, dy)| {
                        let p = Pos::new(st.pos.x + dx, st.pos.y + dy);
                        !w.is_obstacle(p) && !w.is_occupied(p, st.id)
                    })
                    .collect();
            }
            if let Some(&d) = choices.choose(rng) {
                st.dir = d;
                st.pos = Pos::new(st.pos.x + d.0, st.pos.y + d.1);
            }
        }
        w.robots[st.id].pos = st.pos;
    }

    for m in msgs {
        let _ = tx.send(m);
    }
}

/// Collecteur : rejoint une ressource connue, collecte une unité à la fois,
/// puis rapporte à la base. Explore si rien n'est encore connu.
fn collector_tick(
    st: &mut State,
    world: &Arc<Mutex<World>>,
    knowledge: &Arc<RwLock<Knowledge>>,
    tx: &Sender<Message>,
    rng: &mut impl Rng,
) {
    let mut msgs = Vec::new();

    // --- Phase lecture : connaissance partagée -----------------------------
    let obstacles = {
        let k = knowledge.read().unwrap_or_else(|e| e.into_inner());

        // Invalide la cible si elle a disparu ou a été prise par un autre.
        if let Some(t) = st.target {
            let still_ok = k.resources.get(&t).is_some_and(|r| {
                !r.depleted && r.qty > 0 && (r.claimed_by.is_none() || r.claimed_by == Some(st.id))
            });
            if !still_ok {
                st.target = None;
            }
        }

        // Sans charge ni cible, en choisit une nouvelle et la réclame.
        if st.carrying.is_none() && st.target.is_none() {
            if let Some(p) = k.nearest_available(st.pos, st.id) {
                st.target = Some(p);
                msgs.push(Message::Claim { id: st.id, pos: p });
            }
        }

        k.obstacles.clone()
    };

    let base = world.lock().unwrap_or_else(|e| e.into_inner()).base;

    // --- Phase action : agit sur le monde ----------------------------------
    {
        let mut w = world.lock().unwrap_or_else(|e| e.into_inner());

        if st.carrying.is_some() {
            if st.pos == base {
                let kind = st.carrying.take().unwrap();
                msgs.push(Message::Collected { kind, amount: 1 });
            } else {
                advance(&w, st.id, &mut st.pos, base, &obstacles, &mut msgs, rng);
            }
        } else if let Some(t) = st.target {
            if st.pos == t {
                collect_here(&mut w, t, st, &mut msgs);
            } else {
                advance(&w, st.id, &mut st.pos, t, &obstacles, &mut msgs, rng);
            }
        } else {
            // Rien de connu : exploration aléatoire.
            if let Some(np) = random_walk(&w, st.id, st.pos, rng) {
                st.pos = np;
            }
        }

        sense(&w, st.pos, &mut msgs);
        w.robots[st.id].pos = st.pos;
        w.robots[st.id].carrying = st.carrying;
    }

    for m in msgs {
        let _ = tx.send(m);
    }
}

/// Collecte une unité sur la case courante (le collecteur est l'autorité sur
/// la quantité réelle restante).
fn collect_here(w: &mut World, t: Pos, st: &mut State, msgs: &mut Vec<Message>) {
    match w.resources.get_mut(&t) {
        Some(res) if res.remaining > 0 => {
            res.remaining -= 1;
            let kind = res.kind;
            let remaining = res.remaining;
            st.carrying = Some(kind);
            st.target = None;
            if remaining == 0 {
                w.resources.remove(&t);
                msgs.push(Message::Deplete { pos: t });
            } else {
                msgs.push(Message::FoundResource {
                    pos: t,
                    kind,
                    qty: remaining,
                });
            }
        }
        _ => {
            w.resources.remove(&t);
            msgs.push(Message::Deplete { pos: t });
            st.target = None;
        }
    }
}

/// Avance d'un pas vers `goal` en suivant le BFS sur les obstacles connus. Si le
/// pas idéal est un obstacle non encore découvert, on le signale (apprentissage)
/// et on ne s'y engage pas. S'il est occupé par un autre robot, on contourne
/// vers le but (voisin libre le plus proche de `goal`), avec un pas de côté
/// aléatoire en dernier recours : cela évite à la fois les superpositions et les
/// blocages tête-à-tête, sans faire dériver le robot loin de sa destination.
fn advance(
    w: &World,
    id: usize,
    pos: &mut Pos,
    goal: Pos,
    obstacles: &std::collections::HashSet<Pos>,
    msgs: &mut Vec<Message>,
    rng: &mut impl Rng,
) {
    let bfs_next = path::next_step(*pos, goal, w.width, w.height, &|p| obstacles.contains(&p));

    if let Some(np) = bfs_next {
        if w.is_obstacle(np) {
            msgs.push(Message::FoundObstacle { pos: np });
        }
    }

    let step = match bfs_next {
        Some(np) if !w.is_obstacle(np) && !w.is_occupied(np, id) => Some(np),
        _ => greedy_step(*pos, goal, id, w, obstacles).or_else(|| random_walk(w, id, *pos, rng)),
    };

    if let Some(np) = step {
        *pos = np;
    }
}

/// Signale les ressources et obstacles adjacents (rayon 1).
fn sense(w: &World, pos: Pos, msgs: &mut Vec<Message>) {
    for p in pos.neighbors4() {
        if !w.in_bounds(p) {
            continue;
        }
        if w.is_obstacle(p) {
            msgs.push(Message::FoundObstacle { pos: p });
        } else if let Some(r) = w.resources.get(&p) {
            msgs.push(Message::FoundResource {
                pos: p,
                kind: r.kind,
                qty: r.remaining,
            });
        }
    }
}

/// Voisin franchissable qui rapproche le plus du but (repli si le BFS échoue).
fn greedy_step(
    from: Pos,
    goal: Pos,
    id: usize,
    w: &World,
    obstacles: &std::collections::HashSet<Pos>,
) -> Option<Pos> {
    from.neighbors4()
        .into_iter()
        .filter(|&p| {
            w.in_bounds(p) && !w.is_obstacle(p) && !obstacles.contains(&p) && !w.is_occupied(p, id)
        })
        .min_by_key(|&p| p.manhattan(goal))
}

fn random_walk(w: &World, id: usize, pos: Pos, rng: &mut impl Rng) -> Option<Pos> {
    let options: Vec<Pos> = pos
        .neighbors4()
        .into_iter()
        .filter(|&p| !w.is_obstacle(p) && !w.is_occupied(p, id))
        .collect();
    options.choose(rng).copied()
}
