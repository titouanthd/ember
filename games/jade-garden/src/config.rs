//! Global context: colors, layout, tuning constants.
//!
//! Loaded once from `.env` via `ember_stdlib::config`. The colors
//! have sane defaults when the `.env` file is missing.

use std::path::Path;

use ember_stdlib::config::{env_color, load_dotenv_once};
use macroquad::prelude::*;

use crate::components::{GRID_H, GRID_W};
use crate::tile_render::{TILE_GAP, TILE_SIZE, TILE_STRIDE};

#[derive(Debug, Clone)]
pub struct Colors {
    pub paper_top: Color,
    pub paper_bottom: Color,
    pub panel_bg: Color,
    pub panel_border: Color,
    pub text: Color,
    pub text_dim: Color,
    pub gold: Color,
    pub select: Color,
    pub danger: Color,
}

impl Colors {
    /// Default palette — dark lacquer + imperial gold + cinnabar red.
    ///
    /// Inspired by a lacquered Chinese box seen under warm lamplight:
    /// the paper is near-black brown, the accents are aged gold leaf,
    /// the highlights are cinnabar red.
    pub fn defaults() -> Self {
        Self {
            // Dark lacquer background.
            paper_top:    Color::new(0.09, 0.075, 0.060, 1.0),  // #17130F
            paper_bottom: Color::new(0.05, 0.040, 0.035, 1.0),  // #0D0A08

            // Warm dark translucent panel.
            panel_bg:     Color::new(0.14, 0.11, 0.085, 0.90),

            // Aged gold leaf border.
            panel_border: Color::new(0.72, 0.55, 0.26, 1.0),   // #B88A42

            // Warm ivory text.
            text:         Color::new(0.94, 0.89, 0.78, 1.0),   // #EDE3C7
            text_dim:     Color::new(0.60, 0.53, 0.42, 1.0),   // #9A876B

            // Imperial gold.
            gold:         Color::new(0.87, 0.70, 0.34, 1.0),   // #DDB357

            // Cinnabar red for selection.
            select:       Color::new(0.86, 0.31, 0.20, 1.0),   // #DB4F33

            // Deep cinnabar for danger.
            danger:       Color::new(0.80, 0.24, 0.16, 1.0),   // #CC3D28
        }
    }

    fn from_env() -> Self {
        let d = Self::defaults();
        Self {
            paper_top: env_color("COLOR_PAPER_TOP", d.paper_top),
            paper_bottom: env_color("COLOR_PAPER_BOTTOM", d.paper_bottom),
            panel_bg: env_color("COLOR_PANEL_BG", d.panel_bg),
            panel_border: env_color("COLOR_PANEL_BORDER", d.panel_border),
            text: env_color("COLOR_TEXT", d.text),
            text_dim: env_color("COLOR_TEXT_DIM", d.text_dim),
            gold: env_color("COLOR_GOLD", d.gold),
            select: env_color("COLOR_SELECT", d.select),
            danger: env_color("COLOR_DANGER", d.danger),
        }
    }
}

/// Global context, resolved once at startup.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub colors: Colors,
    pub viewport_w: f32,
    pub viewport_h: f32,
}

impl GameContext {
    pub fn from_env() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_once(manifest_dir, "jade-garden");

        let vw = screen_width();
        let vh = screen_height();
        Self {
            colors: Colors::from_env(),
            viewport_w: vw,
            viewport_h: vh,
        }
    }

    pub fn defaults() -> Self {
        Self {
            colors: Colors::defaults(),
            viewport_w: 1200.0,
            viewport_h: 900.0,
        }
    }

    /// Top-left corner of the grid (centered horizontally and
    /// vertically in the viewport).
    pub fn grid_origin(&self) -> (f32, f32) {
        let grid_w = GRID_W as f32 * TILE_STRIDE - TILE_GAP;
        let grid_h = GRID_H as f32 * TILE_STRIDE - TILE_GAP;
        (
            (self.viewport_w - grid_w) * 0.5,
            (self.viewport_h - grid_h) * 0.5,
        )
    }

    /// Convert a grid coordinate (row, col) to screen coordinates
    /// (top-left corner of the tile).
    pub fn tile_origin(&self, row: f32, col: f32) -> (f32, f32) {
        let (gx, gy) = self.grid_origin();
        (gx + col * TILE_STRIDE, gy + row * TILE_STRIDE)
    }

    /// Hit test: returns the (row, col) under the mouse, or `None`.
    pub fn tile_at(&self, mx: f32, my: f32) -> Option<(usize, usize)> {
        let (gx, gy) = self.grid_origin();
        if mx < gx || my < gy {
            return None;
        }
        let lx = mx - gx;
        let ly = my - gy;
        let col = (lx / TILE_STRIDE).floor() as isize;
        let row = (ly / TILE_STRIDE).floor() as isize;
        if row < 0 || col < 0 || row >= GRID_H as isize || col >= GRID_W as isize {
            return None;
        }
        // Confirm we're inside the tile and not in the gap.
        let tile_local_x = lx - col as f32 * TILE_STRIDE;
        let tile_local_y = ly - row as f32 * TILE_STRIDE;
        if tile_local_x > TILE_SIZE || tile_local_y > TILE_SIZE {
            return None;
        }
        Some((row as usize, col as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> GameContext {
        GameContext::defaults()
    }

    #[test]
    fn grid_origin_centered() {
        let c = ctx();
        let (gx, gy) = c.grid_origin();
        let grid_w = GRID_W as f32 * TILE_STRIDE - TILE_GAP;
        let grid_h = GRID_H as f32 * TILE_STRIDE - TILE_GAP;
        assert!((gx - (1200.0 - grid_w) * 0.5).abs() < 1e-3);
        assert!((gy - (900.0 - grid_h) * 0.5).abs() < 1e-3);
    }

    #[test]
    fn tile_origin_matches_grid_origin_at_zero_zero() {
        let c = ctx();
        let (gx, gy) = c.grid_origin();
        let (tx, ty) = c.tile_origin(0.0, 0.0);
        assert!((tx - gx).abs() < 1e-6);
        assert!((ty - gy).abs() < 1e-6);
    }

    #[test]
    fn tile_origin_shifts_by_stride() {
        let c = ctx();
        let (tx1, ty1) = c.tile_origin(0.0, 0.0);
        let (tx2, ty2) = c.tile_origin(2.0, 3.0);
        assert!((tx2 - (tx1 + 3.0 * TILE_STRIDE)).abs() < 1e-3);
        assert!((ty2 - (ty1 + 2.0 * TILE_STRIDE)).abs() < 1e-3);
    }

    #[test]
    fn tile_at_center_of_first_tile() {
        let c = ctx();
        let (gx, gy) = c.grid_origin();
        let mx = gx + TILE_SIZE * 0.5;
        let my = gy + TILE_SIZE * 0.5;
        assert_eq!(c.tile_at(mx, my), Some((0, 0)));
    }

    #[test]
    fn tile_at_center_of_middle_tile() {
        let c = ctx();
        let (gx, gy) = c.grid_origin();
        let mx = gx + 4.0 * TILE_STRIDE + TILE_SIZE * 0.5;
        let my = gy + 5.0 * TILE_STRIDE + TILE_SIZE * 0.5;
        assert_eq!(c.tile_at(mx, my), Some((5, 4)));
    }

    #[test]
    fn tile_at_outside_returns_none() {
        let c = ctx();
        assert!(c.tile_at(5.0, 5.0).is_none());
        assert!(c.tile_at(1e6, 1e6).is_none());
        assert!(c.tile_at(-100.0, -100.0).is_none());
    }

    #[test]
    fn tile_at_gap_returns_none() {
        let c = ctx();
        let (gx, gy) = c.grid_origin();
        let mx = gx + TILE_SIZE + TILE_GAP * 0.5;
        let my = gy + TILE_SIZE * 0.5;
        let hit = c.tile_at(mx, my);
        assert!(hit.is_none() || hit == Some((0, 1)));
    }

    #[test]
    fn tile_at_bottom_right_corner() {
        let c = ctx();
        let (mx, my) = c.tile_origin(GRID_H as f32 - 1.0, GRID_W as f32 - 1.0);
        let cx = mx + TILE_SIZE * 0.5;
        let cy = my + TILE_SIZE * 0.5;
        assert_eq!(c.tile_at(cx, cy), Some((GRID_H - 1, GRID_W - 1)));
    }

    #[test]
    fn colors_default_in_range() {
        let c = Colors::defaults();
        for col in [
            c.paper_top,
            c.paper_bottom,
            c.panel_bg,
            c.panel_border,
            c.text,
            c.text_dim,
            c.gold,
            c.select,
            c.danger,
        ] {
            for v in [col.r, col.g, col.b, col.a] {
                assert!((0.0..=1.0).contains(&v));
            }
        }
    }
}