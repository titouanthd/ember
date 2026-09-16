// components.rs
use glam::IVec2;
use macroquad::prelude::Color;

/// Une case sur la grille logique (colonne, ligne).
/// On utilise IVec2 (entiers) pour éviter les erreurs de float.
pub type Cell = IVec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Vecteur unitaire correspondant (attention : Y descend à l'écran).
    pub fn delta(self) -> Cell {
        match self {
            Direction::Up => Cell::new(0, -1),
            Direction::Down => Cell::new(0, 1),
            Direction::Left => Cell::new(-1, 0),
            Direction::Right => Cell::new(1, 0),
        }
    }

    /// Deux directions sont opposées si leur somme est nulle.
    pub fn is_opposite(self, other: Direction) -> bool {
        self.delta() + other.delta() == Cell::ZERO
    }
}

/// Le serpent : une tête + un corps.
/// `body[0]` est la tête, les suivants sont les segments.
#[derive(Debug, Clone)]
pub struct Snake {
    pub body: std::collections::VecDeque<Cell>,
    pub direction: Direction,
    /// Direction en attente (pour empêcher le retournement en un tick).
    pub pending_direction: Option<Direction>,
    pub color_head: Color,
    pub color_body: Color,
}

impl Snake {
    pub fn new(
        start: Cell,
        length: u32,
        direction: Direction,
        color_head: Color,
        color_body: Color,
    ) -> Self {
        let mut body = std::collections::VecDeque::new();
        // La tête est à `start`, le corps s'étend derrière selon la direction opposée.
        let back = match direction {
            Direction::Up => Direction::Down.delta(),
            Direction::Down => Direction::Up.delta(),
            Direction::Left => Direction::Right.delta(),
            Direction::Right => Direction::Left.delta(),
        };
        for i in 0..length as i32 {
            body.push_back(start + back * i);
        }
        Self {
            body,
            direction,
            pending_direction: None,
            color_head,
            color_body,
        }
    }

    pub fn head(&self) -> Cell {
        *self.body.front().expect("snake has at least one segment")
    }

    /// Nombre de segments.
    ///
    /// Un serpent a toujours au moins une tête (`Snake::new` et `advance`
    /// garantissent `len() >= 1`), donc `is_empty()` n'a pas de sens ici.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.body.len()
    }

    /// Ajoute une direction demandée par le joueur.
    /// On stocke une seule direction en attente (la dernière gagne),
    /// et on refuse le demi-tour immédiat.
    pub fn request_direction(&mut self, dir: Direction) {
        if self.direction.is_opposite(dir) {
            return;
        }
        self.pending_direction = Some(dir);
    }

    /// Applique la direction en attente (appelé une fois par tick).
    pub fn apply_pending_direction(&mut self) {
        if let Some(dir) = self.pending_direction.take()
            && !self.direction.is_opposite(dir)
        {
            self.direction = dir;
        }
    }

    /// Avance d'une case. Retourne la nouvelle tête.
    /// Ne gère PAS la nourriture ni les collisions (c'est le rôle du système).
    pub fn advance(&mut self, grow: bool) -> Cell {
        let new_head = self.head() + self.direction.delta();
        self.body.push_front(new_head);
        if !grow {
            self.body.pop_back();
        }
        new_head
    }
}

#[derive(Debug, Clone)]
pub struct Food {
    pub cell: Cell,
    pub color: Color,
}

impl Food {
    pub fn new(cell: Cell, color: Color) -> Self {
        Self { cell, color }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    pub cell: Cell,
    pub color: Color,
}

impl Wall {
    pub fn new(cell: Cell, color: Color) -> Self {
        Self { cell, color }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: i32, y: i32) -> Cell {
        Cell::new(x, y)
    }

    #[test]
    fn test_direction_delta() {
        assert_eq!(Direction::Up.delta(), c(0, -1));
        assert_eq!(Direction::Down.delta(), c(0, 1));
        assert_eq!(Direction::Left.delta(), c(-1, 0));
        assert_eq!(Direction::Right.delta(), c(1, 0));
    }

    #[test]
    fn test_direction_opposite() {
        assert!(Direction::Up.is_opposite(Direction::Down));
        assert!(Direction::Left.is_opposite(Direction::Right));
        assert!(!Direction::Up.is_opposite(Direction::Left));
    }

    #[test]
    fn test_snake_initial_length_and_head() {
        let s = Snake::new(
            c(5, 5),
            3,
            Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        assert_eq!(s.len(), 3);
        assert_eq!(s.head(), c(5, 5));
        // Le corps s'étend vers la gauche.
        let cells: Vec<Cell> = s.body.iter().copied().collect();
        assert_eq!(cells, vec![c(5, 5), c(4, 5), c(3, 5)]);
    }

    #[test]
    fn test_snake_advance_without_grow_keeps_length() {
        let mut s = Snake::new(
            c(5, 5),
            3,
            Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        s.advance(false);
        assert_eq!(s.len(), 3);
        assert_eq!(s.head(), c(6, 5));
    }

    #[test]
    fn test_snake_advance_with_grow_increases_length() {
        let mut s = Snake::new(
            c(5, 5),
            3,
            Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        s.advance(true);
        assert_eq!(s.len(), 4);
        assert_eq!(s.head(), c(6, 5));
    }

    #[test]
    fn test_snake_cannot_reverse_instantly() {
        let mut s = Snake::new(
            c(5, 5),
            3,
            Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        s.request_direction(Direction::Left); // opposé → refusé
        s.apply_pending_direction();
        assert_eq!(s.direction, Direction::Right);
    }

    #[test]
    fn test_snake_accepts_perpendicular_turn() {
        let mut s = Snake::new(
            c(5, 5),
            3,
            Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        s.request_direction(Direction::Up);
        s.apply_pending_direction();
        assert_eq!(s.direction, Direction::Up);
    }
}
