//! Pathfinding par BFS sur les obstacles *connus*. Renvoie le prochain pas
//! vers le but, ou None si aucun chemin connu n'existe.

use std::collections::{HashMap, VecDeque};

use crate::types::Pos;

pub fn next_step(
    start: Pos,
    goal: Pos,
    width: i32,
    height: i32,
    blocked: &dyn Fn(Pos) -> bool,
) -> Option<Pos> {
    if start == goal {
        return None;
    }

    let mut frontier = VecDeque::new();
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    frontier.push_back(start);
    came_from.insert(start, start);

    while let Some(current) = frontier.pop_front() {
        if current == goal {
            break;
        }
        for n in current.neighbors4() {
            if n.x < 0 || n.y < 0 || n.x >= width || n.y >= height {
                continue;
            }
            if came_from.contains_key(&n) {
                continue;
            }
            // Le but reste accessible même s'il porte une ressource ;
            // on ne bloque que les cases intermédiaires.
            if n != goal && blocked(n) {
                continue;
            }
            came_from.insert(n, current);
            frontier.push_back(n);
        }
    }

    if !came_from.contains_key(&goal) {
        return None;
    }

    // Remonte le chemin du but vers le départ, retourne le premier pas.
    let mut step = goal;
    while came_from[&step] != start {
        step = came_from[&step];
    }
    Some(step)
}
