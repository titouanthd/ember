//! Wave definitions, loaded from `waves.ron` next to the crate root.
//!
//! Wave data is game-local (Breakout's `levels.rs` pattern): the schema is
//! specific to Bullet Hell and lives here, not in `ember_stdlib`.

use std::path::PathBuf;

use glam::Vec2;
use macroquad::prelude::Color;
use serde::Deserialize;

use crate::components::{Emitter, EntryMotion};

/// One enemy definition inside a wave.
#[derive(Debug, Clone, Deserialize)]
pub struct EnemyData {
    /// Free-form tag: "grunt", "tank", "boss", etc. Drives styling only.
    pub kind: String,
    pub entry: EntryMotion,
    /// Spawn position as `(x, y)` — RON positional tuple.
    pub pos: (f32, f32),
    pub hp: i32,
    pub emitter: Emitter,
}

/// One wave: a set of enemies to survive.
#[derive(Debug, Clone, Deserialize)]
pub struct WaveData {
    /// Nominal duration in seconds. Used for HUD display only — a wave ends
    /// when all its enemies are dead, not when this elapses.
    pub duration: f32,
    pub enemies: Vec<EnemyData>,
}

/// Load all waves from `waves.ron`. Falls back to a single hardcoded wave if
/// the file is missing or fails to parse, so the game is always playable.
pub fn load_waves() -> Vec<WaveData> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("waves.ron");
    match ember_core::io::load_from_file::<Vec<WaveData>>(&path) {
        Ok(waves) if !waves.is_empty() => waves,
        _ => fallback_wave(),
    }
}

/// Minimum viable content. Used only if `waves.ron` is absent or broken.
fn fallback_wave() -> Vec<WaveData> {
    vec![WaveData {
        duration: 30.0,
        enemies: vec![EnemyData {
            kind: "grunt".into(),
            entry: EntryMotion::Static,
            pos: (400.0, 120.0),
            hp: 10,
            emitter: Emitter::Radial {
                count: 8,
                speed: 90.0,
                cooldown: 1.6,
            },
        }],
    }]
}

/// `(radius, color)` for an enemy kind. Unknown kinds fall back to the grunt
/// style; "grunt" itself uses the config color (see `Enemy::from_data`).
pub fn enemy_style(kind: &str) -> (f32, Color) {
    match kind {
        "boss" => (36.0, Color::new(1.0, 0.4, 0.2, 1.0)),
        "tank" => (26.0, Color::new(0.7, 0.5, 0.9, 1.0)),
        _ => (18.0, Color::new(0.9, 0.3, 0.5, 1.0)),
    }
}

/// Convert a `(f32, f32)` tuple from the RON into a `Vec2`.
pub fn pos_vec(t: (f32, f32)) -> Vec2 {
    Vec2::new(t.0, t.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_wave_not_empty() {
        let waves = fallback_wave();
        assert_eq!(waves.len(), 1);
        assert!(!waves[0].enemies.is_empty());
    }

    #[test]
    fn test_enemy_style_boss_is_bigger() {
        let (r_boss, _) = enemy_style("boss");
        let (r_grunt, _) = enemy_style("grunt");
        assert!(r_boss > r_grunt);
    }

    #[test]
    fn test_enemy_style_unknown_falls_back_to_grunt() {
        let (r_unknown, _) = enemy_style("not_a_real_kind");
        let (r_grunt, _) = enemy_style("grunt");
        assert_eq!(r_unknown, r_grunt);
    }

    #[test]
    fn test_pos_vec_maps_tuple() {
        let v = pos_vec((12.0, 34.0));
        assert_eq!(v, Vec2::new(12.0, 34.0));
    }
}