//! Game configuration resolved from `.env`.

use std::path::Path;

use ember_stdlib::config::{env_color, env_u32, load_dotenv_once};
use macroquad::prelude::Color;

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub table_bg: Color,
    pub table_border: Color,
    pub tile_face: Color,
    pub tile_edge: Color,
    pub tile_text: Color,
    pub text: Color,
    pub text_dim: Color,
    pub highlight: Color,
    pub danger: Color,
}

impl Colors {
    pub const fn default_hermetic() -> Self {
        Self {
            table_bg: Color::new(0.10, 0.14, 0.10, 1.0),
            table_border: Color::new(0.24, 0.32, 0.24, 1.0),
            tile_face: Color::new(0.94, 0.93, 0.86, 1.0),
            tile_edge: Color::new(0.55, 0.52, 0.42, 1.0),
            tile_text: Color::new(0.10, 0.10, 0.10, 1.0),
            text: Color::new(0.92, 0.94, 0.92, 1.0),
            text_dim: Color::new(0.60, 0.65, 0.60, 1.0),
            highlight: Color::new(1.0, 0.85, 0.30, 0.95),
            danger: Color::new(0.95, 0.45, 0.45, 1.0),
        }
    }

    fn from_env() -> Self {
        let d = Self::default_hermetic();
        Self {
            table_bg: env_color("COLOR_TABLE_BG", d.table_bg),
            table_border: env_color("COLOR_TABLE_BORDER", d.table_border),
            tile_face: env_color("COLOR_TILE_FACE", d.tile_face),
            tile_edge: env_color("COLOR_TILE_EDGE", d.tile_edge),
            tile_text: env_color("COLOR_TILE_TEXT", d.tile_text),
            text: env_color("COLOR_TEXT", d.text),
            text_dim: env_color("COLOR_TEXT_DIM", d.text_dim),
            highlight: env_color("COLOR_HIGHLIGHT", d.highlight),
            danger: env_color("COLOR_DANGER", d.danger),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub window_w: f32,
    pub window_h: f32,

    pub deal_duration: f32,
    pub draw_duration: f32,
    pub ai_think_duration: f32,
    /// How long the human has to discard on their turn.
    pub turn_window: f32,
    /// How long the human has to claim a discard.
    pub claim_window: f32,
    pub claim_duration: f32,

    pub hands_per_match: u32,
}

impl Layout {
    pub const fn defaults() -> Self {
        Self {
            window_w: 1280.0,
            window_h: 720.0,
            deal_duration: 0.5,
            draw_duration: 0.25,
            ai_think_duration: 0.6,
            turn_window: 30.0,
            claim_window: 10.0,
            claim_duration: 0.4,
            hands_per_match: 16,
        }
    }

    fn from_env() -> Self {
        let d = Self::defaults();
        Self {
            hands_per_match: env_u32("AH_HANDS_PER_MATCH", d.hands_per_match).max(1),
            ..d
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameContext {
    pub colors: Colors,
    pub layout: Layout,
}

impl GameContext {
    pub fn default_hermetic() -> Self {
        Self {
            colors: Colors::default_hermetic(),
            layout: Layout::defaults(),
        }
    }

    pub fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_once(manifest_dir, "zhuo-ji");
        Self {
            colors: Colors::from_env(),
            layout: Layout::from_env(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermetic_context_defaults() {
        let ctx = GameContext::default_hermetic();
        assert_eq!(ctx.layout.window_w, 1280.0);
        assert_eq!(ctx.layout.hands_per_match, 16);
    }

    #[test]
    fn test_colors_in_range() {
        let c = Colors::default_hermetic();
        for col in [
            c.table_bg, c.table_border, c.tile_face, c.tile_edge, c.tile_text,
            c.text, c.text_dim, c.highlight, c.danger,
        ] {
            for v in [col.r, col.g, col.b, col.a] {
                assert!((0.0..=1.0).contains(&v));
            }
        }
    }

    #[test]
    fn test_turn_window_is_thirty_seconds() {
        assert!((Layout::defaults().turn_window - 30.0).abs() < 1e-3);
    }

    #[test]
    fn test_claim_window_is_ten_seconds() {
        assert!((Layout::defaults().claim_window - 10.0).abs() < 1e-3);
    }

    #[test]
    fn test_hands_per_match_is_sixteen() {
        assert_eq!(Layout::defaults().hands_per_match, 16);
    }
}