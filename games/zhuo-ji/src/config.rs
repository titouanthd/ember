//! Game configuration resolved from `.env`.
//!
//! `GameContext` is the single source of truth for layout, palette, and
//! tuning. Built once in `main.rs` and passed by reference everywhere.

use std::path::Path;

use ember_stdlib::config::{env_color, load_dotenv_once};
use macroquad::prelude::Color;

/// Colours used by the V1 renderer.
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
}

impl Colors {
    /// Defaults for hermetic tests — do not read `.env`.
    pub const fn default_hermetic() -> Self {
        Self {
            table_bg: Color::new(0.10, 0.14, 0.10, 1.0), // dark felt
            table_border: Color::new(0.24, 0.32, 0.24, 1.0),
            tile_face: Color::new(0.94, 0.93, 0.86, 1.0), // ivory
            tile_edge: Color::new(0.55, 0.52, 0.42, 1.0),
            tile_text: Color::new(0.10, 0.10, 0.10, 1.0),
            text: Color::new(0.92, 0.94, 0.92, 1.0),
            text_dim: Color::new(0.60, 0.65, 0.60, 1.0),
            highlight: Color::new(1.0, 0.85, 0.30, 0.85),
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
        }
    }
}

/// Layout and timing.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub window_w: f32,
    pub window_h: f32,

    pub tile_w: f32,
    pub tile_h: f32,
    pub tile_gap: f32,

    pub deal_duration: f32,
    pub draw_duration: f32,
    pub ai_think_duration: f32,

    pub claim_window: f32,     // how long the human has to decide
    pub claim_duration: f32,   // short animation after a claim resolves
}

impl Layout {
    pub const fn defaults() -> Self {
        Self {
            window_w: 1280.0,
            window_h: 720.0,
            tile_w: 56.0,
            tile_h: 78.0,
            tile_gap: 4.0,
            deal_duration: 0.5,
            draw_duration: 0.25,
            ai_think_duration: 0.6,
            claim_window: 2.0,
            claim_duration: 0.4,
        }
    }
}

/// Everything the game needs. Built once, passed by reference.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub colors: Colors,
    pub layout: Layout,
}

impl GameContext {
    /// Hermetic context for tests — no `.env` read.
    pub fn default_hermetic() -> Self {
        Self {
            colors: Colors::default_hermetic(),
            layout: Layout::defaults(),
        }
    }

    /// Load from `.env` in this crate's manifest directory.
    pub fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_once(manifest_dir, "zhuo-ji");
        Self {
            colors: Colors::from_env(),
            layout: Layout::defaults(),
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
        assert_eq!(ctx.layout.tile_w, 56.0);
        assert!(ctx.layout.deal_duration > 0.0);
    }

    #[test]
    fn test_colors_in_range() {
        let c = Colors::default_hermetic();
        for col in [
            c.table_bg,
            c.table_border,
            c.tile_face,
            c.tile_edge,
            c.tile_text,
            c.text,
            c.text_dim,
            c.highlight,
        ] {
            for v in [col.r, col.g, col.b, col.a] {
                assert!((0.0..=1.0).contains(&v), "component out of range: {v}");
            }
        }
    }

    #[test]
    fn test_ai_think_duration_is_positive() {
        let ctx = GameContext::default_hermetic();
        assert!(ctx.layout.ai_think_duration > 0.0);
        assert!(ctx.layout.ai_think_duration < 2.0, "AI shouldn't take forever");
    }

    #[test]
    fn test_claim_timings_are_reasonable() {
        let l = Layout::defaults();
        assert!(l.claim_window >= 1.0, "too short for a human to react");
        assert!(l.claim_window <= 5.0, "too long to wait");
        assert!(l.claim_duration > 0.0 && l.claim_duration < 1.0);
    }
}