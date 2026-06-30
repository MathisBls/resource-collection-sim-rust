//! Types partagés par toute la simulation.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn manhattan(&self, other: Pos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Les 4 voisins orthogonaux (haut, bas, gauche, droite).
    pub fn neighbors4(&self) -> [Pos; 4] {
        [
            Pos::new(self.x + 1, self.y),
            Pos::new(self.x - 1, self.y),
            Pos::new(self.x, self.y + 1),
            Pos::new(self.x, self.y - 1),
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceKind {
    Energy,
    Crystal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RobotKind {
    Scout,
    Collector,
}
