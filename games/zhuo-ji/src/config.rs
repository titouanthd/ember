//! Game configuration resolved from `.env`.

use std::path::Path;

use ember_stdlib::config::{env_color, env_u32, load_dotenv_once};
use macroquad::prelude::Color;

use crate::font::TileFont;

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

    // ─── Mountain layers (far → near) ────────────────────────────────
    pub mountain_far: Color,
    pub mountain_mid: Color,
    pub mountain_near: Color,
    pub mountain_fore: Color,
    pub mountain_shade: Color,
    pub mountain_rim: Color,
    pub mist: Color,

    // ─── Wall ────────────────────────────────────────────────────────
    pub wall_top: Color,
    pub wall_top_dark: Color,
    pub wall_edge: Color,
    pub wall_edge_shadow: Color,
    pub wall_gold: Color,

    // ─── Premium border ──────────────────────────────────────────────
    pub gold_dark: Color,
    pub gold_light: Color,
}

impl Colors {
    pub const fn default_hermetic() -> Self {
        Self {
            table_bg: Color::new(0.08, 0.16, 0.11, 1.0),
            table_border: Color::new(0.55, 0.43, 0.23, 1.0),
            tile_face: Color::new(0.96, 0.94, 0.86, 1.0),
            tile_edge: Color::new(0.62, 0.58, 0.48, 1.0),
            tile_text: Color::new(0.10, 0.10, 0.10, 1.0),
            text: Color::new(0.94, 0.92, 0.86, 1.0),
            text_dim: Color::new(0.65, 0.68, 0.60, 1.0),
            highlight: Color::new(0.95, 0.85, 0.35, 1.0),
            danger: Color::new(0.85, 0.35, 0.30, 1.0),

            mountain_far:  Color::new(0.24, 0.34, 0.30, 1.0),
            mountain_mid:  Color::new(0.17, 0.29, 0.23, 1.0),
            mountain_near: Color::new(0.12, 0.23, 0.17, 1.0),
            mountain_fore: Color::new(0.08, 0.16, 0.11, 1.0),
            mountain_shade: Color::new(0.05, 0.12, 0.09, 1.0),
            mountain_rim: Color::new(0.42, 0.55, 0.48, 0.55),
            mist: Color::new(0.85, 0.88, 0.86, 0.15),

            wall_top:         Color::new(0.20, 0.42, 0.30, 1.0),
            wall_top_dark:    Color::new(0.11, 0.28, 0.19, 1.0),
            wall_edge:        Color::new(0.94, 0.92, 0.86, 1.0),
            wall_edge_shadow: Color::new(0.70, 0.68, 0.60, 1.0),
            wall_gold:        Color::new(0.83, 0.69, 0.22, 0.9),

            gold_dark:  Color::new(0.55, 0.43, 0.23, 1.0),
            gold_light: Color::new(0.83, 0.69, 0.22, 1.0),
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
            mountain_far: env_color("COLOR_MOUNTAIN_FAR", d.mountain_far),
            mountain_mid: env_color("COLOR_MOUNTAIN_MID", d.mountain_mid),
            mountain_near: env_color("COLOR_MOUNTAIN_NEAR", d.mountain_near),
            mountain_fore: env_color("COLOR_MOUNTAIN_FORE", d.mountain_fore),
            mountain_shade: env_color("COLOR_MOUNTAIN_SHADE", d.mountain_shade),
            mountain_rim: env_color("COLOR_MOUNTAIN_RIM", d.mountain_rim),
            mist: env_color("COLOR_MIST", d.mist),
            wall_top: env_color("COLOR_WALL_TOP", d.wall_top),
            wall_top_dark: env_color("COLOR_WALL_TOP_DARK", d.wall_top_dark),
            wall_edge: env_color("COLOR_WALL_EDGE", d.wall_edge),
            wall_edge_shadow: env_color("COLOR_WALL_EDGE_SHADOW", d.wall_edge_shadow),
            wall_gold: env_color("COLOR_WALL_GOLD", d.wall_gold),
            gold_dark: env_color("COLOR_GOLD_DARK", d.gold_dark),
            gold_light: env_color("COLOR_GOLD_LIGHT", d.gold_light),
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
    pub turn_window: f32,
    pub claim_window: f32,
    pub claim_duration: f32,
    pub hands_per_match: u32,
}

impl Layout {
    pub const fn defaults() -> Self {
        Self {
            window_w: 1440.0,
            window_h: 900.0,
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

pub struct GameContext {
    pub colors: Colors,
    pub layout: Layout,
    pub font: TileFont,
}

impl GameContext {
    pub fn default_hermetic() -> Self {
        Self {
            colors: Colors::default_hermetic(),
            layout: Layout::defaults(),
            font: TileFont::hermetic(),
        }
    }

    pub fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_once(manifest_dir, "zhuo-ji");
        Self {
            colors: Colors::from_env(),
            layout: Layout::from_env(),
            font: TileFont::load(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermetic_context_defaults() {
        let ctx = GameContext::default_hermetic();
        assert_eq!(ctx.layout.window_w, 1440.0);
        assert_eq!(ctx.layout.hands_per_match, 16);
        assert!(!ctx.font.is_available());
    }

    #[test]
    fn test_colors_in_range() {
        let c = Colors::default_hermetic();
        for col in [
            c.table_bg, c.table_border, c.tile_face, c.tile_edge, c.tile_text,
            c.text, c.text_dim, c.highlight, c.danger,
            c.mountain_far, c.mountain_mid, c.mountain_near, c.mountain_fore,
            c.mountain_shade, c.mountain_rim, c.mist,
            c.wall_top, c.wall_top_dark, c.wall_edge, c.wall_edge_shadow, c.wall_gold,
            c.gold_dark, c.gold_light,
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
}