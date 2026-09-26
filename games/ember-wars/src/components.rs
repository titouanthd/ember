//! Types de base du jeu : équipes, formes, identifiants, entités.

use glam::Vec2;
use macroquad::prelude::Color;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Team {
    Player,
    Enemy,
}

impl Team {
    pub fn opponent(self) -> Team {
        match self {
            Team::Player => Team::Enemy,
            Team::Enemy => Team::Player,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Shape {
    Rect,
    Circle,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub struct UnitId(pub u32);

#[derive(Debug, Default)]
pub struct UnitIdGen {
    next: u32,
}

impl UnitIdGen {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn next_id(&mut self) -> UnitId {
        let id = UnitId(self.next);
        self.next += 1;
        id
    }
}

#[derive(Debug, Clone)]
pub struct Unit {
    pub id: UnitId,
    pub kind: String,
    pub team: Team,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub damage: f32,
    pub attack_cd: f32,
    pub heal_cd: f32,
    /// Compteur de flash blanc après un hit. > 0 = flash actif.
    pub hit_flash: f32,
    pub target: Option<UnitId>,
}

impl Unit {
    pub fn is_alive(&self) -> bool {
        self.hp > 0.0
    }

    pub fn hp_fraction(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileKind {
    Basic,
    Archer,
    Explosive,
}

#[derive(Debug, Clone)]
pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub radius: f32,
    pub team: Team,
    pub kind: ProjectileKind,
    pub pierce_remaining: u8,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct Tower {
    pub team: Team,
    pub hp: f32,
    pub max_hp: f32,
    pub x: f32,
}

impl Tower {
    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0.0
    }

    pub fn hp_fraction(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }
}

pub fn rgba(c: (f32, f32, f32, f32)) -> Color {
    Color::new(c.0, c.1, c.2, c.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_opponent_round_trips() {
        assert_eq!(Team::Player.opponent(), Team::Enemy);
        assert_eq!(Team::Enemy.opponent(), Team::Player);
        assert_eq!(Team::Player.opponent().opponent(), Team::Player);
    }

    #[test]
    fn unit_id_gen_is_monotonic() {
        let mut id_gen = UnitIdGen::new();
        let a = id_gen.next_id();
        let b = id_gen.next_id();
        let c = id_gen.next_id();
        assert_eq!(a, UnitId(0));
        assert_eq!(b, UnitId(1));
        assert_eq!(c, UnitId(2));
        assert!(a < b && b < c);
    }

    #[test]
    fn tower_hp_fraction_clamps() {
        let t = Tower {
            team: Team::Player,
            hp: 600.0,
            max_hp: 500.0,
            x: 0.0,
        };
        assert_eq!(t.hp_fraction(), 1.0);
    }

    #[test]
    fn rgba_conversion_preserves_channels() {
        let c = rgba((0.1, 0.2, 0.3, 0.4));
        assert!((c.r - 0.1).abs() < 1e-6);
        assert!((c.g - 0.2).abs() < 1e-6);
        assert!((c.b - 0.3).abs() < 1e-6);
        assert!((c.a - 0.4).abs() < 1e-6);
    }
}