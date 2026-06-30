//! Génération procédurale de la carte (bruit de Perlin) et état "physique"
//! autoritatif du monde : terrain, ressources vivantes, robots, totaux.

use std::collections::HashMap;

use noise::{NoiseFn, Perlin};
use rand::Rng;

use crate::types::{Pos, ResourceKind, RobotKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Obstacle,
    Base,
}

#[derive(Clone, Copy)]
pub struct Resource {
    pub kind: ResourceKind,
    pub remaining: u32,
}

/// Vue d'un robot stockée dans le monde, uniquement pour le rendu.
#[derive(Clone)]
pub struct RobotView {
    pub kind: RobotKind,
    pub pos: Pos,
    pub carrying: Option<ResourceKind>,
}

pub struct World {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    pub resources: HashMap<Pos, Resource>,
    pub base: Pos,
    pub robots: Vec<RobotView>,
    pub collected_energy: u32,
    pub collected_crystals: u32,
}

impl World {
    fn idx(&self, p: Pos) -> usize {
        (p.y * self.width + p.x) as usize
    }

    pub fn in_bounds(&self, p: Pos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    pub fn tile(&self, p: Pos) -> Tile {
        self.tiles[self.idx(p)]
    }

    /// Un obstacle réel : hors carte ou tuile bloquée.
    pub fn is_obstacle(&self, p: Pos) -> bool {
        !self.in_bounds(p) || self.tile(p) == Tile::Obstacle
    }

    /// Génère une carte : obstacles via Perlin, base au centre,
    /// puis ressources dispersées sur les cases libres.
    pub fn generate(width: i32, height: i32, energy_count: u32, crystal_count: u32) -> World {
        let mut rng = rand::thread_rng();
        let perlin = Perlin::new(rng.gen());
        let scale = 0.16;
        let threshold = 0.45;

        let mut tiles = vec![Tile::Empty; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let n = perlin.get([x as f64 * scale, y as f64 * scale]);
                if n > threshold {
                    tiles[(y * width + x) as usize] = Tile::Obstacle;
                }
            }
        }

        let base = Pos::new(width / 2, height / 2);

        let mut world = World {
            width,
            height,
            tiles,
            resources: HashMap::new(),
            base,
            robots: Vec::new(),
            collected_energy: 0,
            collected_crystals: 0,
        };

        // Dégage une zone 3x3 autour de la base.
        for dy in -1..=1 {
            for dx in -1..=1 {
                let p = Pos::new(base.x + dx, base.y + dy);
                if world.in_bounds(p) {
                    let i = world.idx(p);
                    world.tiles[i] = Tile::Empty;
                }
            }
        }
        let bi = world.idx(base);
        world.tiles[bi] = Tile::Base;

        world.scatter_resources(ResourceKind::Energy, energy_count, &mut rng);
        world.scatter_resources(ResourceKind::Crystal, crystal_count, &mut rng);

        world
    }

    fn scatter_resources(&mut self, kind: ResourceKind, count: u32, rng: &mut impl Rng) {
        let mut placed = 0;
        let mut attempts = 0;
        while placed < count && attempts < count * 200 {
            attempts += 1;
            let p = Pos::new(rng.gen_range(0..self.width), rng.gen_range(0..self.height));
            if self.tile(p) == Tile::Empty && !self.resources.contains_key(&p) && p != self.base {
                self.resources.insert(
                    p,
                    Resource {
                        kind,
                        remaining: rng.gen_range(50..=200),
                    },
                );
                placed += 1;
            }
        }
    }
}
