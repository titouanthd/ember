// games/breakout/src/levels.rs
//! Format de niveau spécifique à Breakout.
//!
//! Ces types vivaient dans `ember_stdlib::level` mais sont spécifiques
//! à ce jeu (briques, couleur RGBA, santé). Chaque jeu définit son
//! propre format, comme Snake le fait déjà avec `SnakeLevel`.

use serde::{Deserialize, Serialize};

/// Une brique dans un niveau Breakout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrickData {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub color_a: f32,
    pub health: i32,
}

/// Un niveau de Breakout : une liste de briques.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub bricks: Vec<BrickData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_ron() {
        let level = Level {
            bricks: vec![BrickData {
                x: 10.0,
                y: 20.0,
                width: 70.0,
                height: 25.0,
                color_r: 1.0,
                color_g: 0.0,
                color_b: 0.0,
                color_a: 1.0,
                health: 2,
            }],
        };
        let s = ron::ser::to_string(&level).unwrap();
        let back: Level = ron::de::from_str(&s).unwrap();
        assert_eq!(back.bricks.len(), 1);
        assert_eq!(back.bricks[0].x, 10.0);
        assert_eq!(back.bricks[0].health, 2);
    }
}
