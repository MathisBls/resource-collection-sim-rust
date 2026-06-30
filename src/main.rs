//! Simulation de collecte de ressources : robots autonomes concurrents
//! (un thread par robot + un thread de base) communiquant par canal mpsc,
//! rendus en temps réel avec Ratatui.

mod base;
mod knowledge;
mod map;
mod message;
mod path;
mod robot;
mod types;
mod ui;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event};

use knowledge::Knowledge;
use map::{RobotView, World};
use types::RobotKind;

const MAP_WIDTH: i32 = 60;
const MAP_HEIGHT: i32 = 30;
const NUM_SCOUTS: usize = 4;
const NUM_COLLECTORS: usize = 4;
const ENERGY_DEPOSITS: u32 = 12;
const CRYSTAL_DEPOSITS: u32 = 12;

fn main() -> io::Result<()> {
    // --- État partagé ------------------------------------------------------
    let mut world = World::generate(MAP_WIDTH, MAP_HEIGHT, ENERGY_DEPOSITS, CRYSTAL_DEPOSITS);
    let base_pos = world.base;

    // Enregistre les robots (l'index dans le Vec == id du robot).
    let mut kinds = Vec::new();
    for _ in 0..NUM_SCOUTS {
        kinds.push(RobotKind::Scout);
    }
    for _ in 0..NUM_COLLECTORS {
        kinds.push(RobotKind::Collector);
    }
    for &kind in &kinds {
        world.robots.push(RobotView {
            kind,
            pos: base_pos,
            carrying: None,
        });
    }

    let world = Arc::new(Mutex::new(world));
    let knowledge = Arc::new(RwLock::new(Knowledge::default()));
    let running = Arc::new(AtomicBool::new(true));
    let (tx, rx) = mpsc::channel();

    // --- Threads -----------------------------------------------------------
    let mut handles = Vec::new();

    {
        let world = Arc::clone(&world);
        let knowledge = Arc::clone(&knowledge);
        let running = Arc::clone(&running);
        handles.push(thread::spawn(move || base::run_base(rx, world, knowledge, running)));
    }

    for (id, kind) in kinds.into_iter().enumerate() {
        let world = Arc::clone(&world);
        let knowledge = Arc::clone(&knowledge);
        let running = Arc::clone(&running);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            robot::run_robot(id, kind, base_pos, world, knowledge, tx, running)
        }));
    }
    // Le tx restant fermera le canal une fois les robots terminés.
    drop(tx);

    // --- Boucle de rendu / entrées ----------------------------------------
    let mut terminal = ratatui::init();
    let result = run_ui(&mut terminal, &world, &running);
    ratatui::restore();

    // Arrêt propre de tous les threads.
    running.store(false, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }

    // Résumé final.
    let w = world.lock().unwrap_or_else(|e| e.into_inner());
    println!(
        "Simulation terminée — Énergie collectée : {} | Cristaux collectés : {}",
        w.collected_energy, w.collected_crystals
    );

    result
}

fn run_ui(
    terminal: &mut ratatui::DefaultTerminal,
    world: &Arc<Mutex<World>>,
    running: &Arc<AtomicBool>,
) -> io::Result<()> {
    while running.load(Ordering::Relaxed) {
        // Instantané sous verrou (rapide), puis rendu hors verrou : l'affichage
        // ne bloque jamais les threads robots, même sur un terminal lent.
        let snapshot = world.lock().unwrap_or_else(|e| e.into_inner()).clone();
        terminal.draw(|f| ui::draw(f, &snapshot))?;

        // Toute pression de touche quitte.
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(_) = event::read()? {
                running.store(false, Ordering::Relaxed);
            }
        }
    }
    Ok(())
}
