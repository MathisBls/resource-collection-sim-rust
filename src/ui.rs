//! Rendu Ratatui : grille colorée + compteurs de ressources collectées.

use std::collections::HashMap;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::map::{Tile, World};
use crate::types::{Pos, RobotKind};

pub fn draw(f: &mut Frame, world: &World) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(f.area());

    // Position des robots, dessinés par-dessus le terrain.
    let mut robots: HashMap<Pos, RobotKind> = HashMap::new();
    for r in &world.robots {
        robots.insert(r.pos, r.kind);
    }

    let mut lines = Vec::with_capacity(world.height as usize);
    for y in 0..world.height {
        let mut spans = Vec::with_capacity(world.width as usize);
        for x in 0..world.width {
            let p = Pos::new(x, y);
            let (ch, color) = match robots.get(&p) {
                Some(RobotKind::Scout) => ('x', Color::Red),
                Some(RobotKind::Collector) => ('o', Color::Magenta),
                None => {
                    if let Some(r) = world.resources.get(&p) {
                        match r.kind {
                            crate::types::ResourceKind::Energy => ('E', Color::Green),
                            crate::types::ResourceKind::Crystal => ('C', Color::LightMagenta),
                        }
                    } else {
                        match world.tile(p) {
                            Tile::Obstacle => ('O', Color::LightCyan),
                            Tile::Base => ('#', Color::LightGreen),
                            Tile::Empty => (' ', Color::Black),
                        }
                    }
                }
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }

    let scouts = world
        .robots
        .iter()
        .filter(|r| r.kind == RobotKind::Scout)
        .count();
    let collectors = world.robots.len() - scouts;
    let title = format!(
        " Collecte de Ressources — Énergie: {}  Cristaux: {}  | Éclaireurs: {}  Collecteurs: {} ",
        world.collected_energy, world.collected_crystals, scouts, collectors
    );

    let map = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );
    f.render_widget(map, chunks[0]);

    let footer = Paragraph::new(Line::from(Span::styled(
        " Appuyez sur n'importe quelle touche pour quitter ",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(footer, chunks[1]);
}
