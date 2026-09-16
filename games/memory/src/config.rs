//! Immutable per-run config, loaded once from `.env`.

use std::path::PathBuf;

use macroquad::prelude::{Color, Vec2};

use ember_stdlib::config::{env_color, env_f32, load_dotenv_once};

/// Immutable per-run config, loaded once from `.env`.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub window_w: f32,
    pub window_h: f32,
    pub hud_h: f32,
    pub footer_h: f32,

    // Board layout
    pub board_margin: f32,
    pub card_gap: f32,
    pub card_radius: f32,
    pub no_match_delay: f32,

    // Colors — terminal theme
    pub color_bg: Color,
    pub color_hidden: Color,
    pub color_hidden_border: Color,
    pub color_hidden_hover: Color,
    pub color_flipped: Color,
    pub color_flipped_border: Color,
    pub color_matched: Color,
    pub color_matched_border: Color,
    pub color_symbol: Color,
    pub color_hidden_symbol: Color,
    pub color_text: Color,
    pub color_text_dim: Color,
    pub color_header_bg: Color,
    pub color_accent: Color,

    // UI buttons
    pub color_btn_idle: Color,
    pub color_btn_hover: Color,
    pub color_btn_pressed: Color,
    pub color_menu_selected: Color,
    pub color_menu_hover: Color,
}

impl GameContext {
    /// Top of the playfield (just below the header).
    pub fn playfield_y(&self) -> f32 {
        self.hud_h
    }

    /// Height of the playfield.
    pub fn playfield_h(&self) -> f32 {
        self.window_h - self.hud_h - self.footer_h
    }

    /// Top-left origin of a centered grid of size (w, h) in the playfield.
    pub fn grid_origin(&self, grid_w: f32, grid_h: f32) -> Vec2 {
        let x = (self.window_w - grid_w) * 0.5;
        let y = self.playfield_y() + (self.playfield_h() - grid_h) * 0.5;
        Vec2::new(x, y)
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    load_dotenv_once(&manifest_dir, "Memory");

    GameContext {
        window_w: env_f32("WINDOW_W", 900.0),
        window_h: env_f32("WINDOW_H", 700.0),
        hud_h: env_f32("HUD_H", 50.0),
        footer_h: env_f32("FOOTER_H", 30.0),

        board_margin: env_f32("BOARD_MARGIN", 20.0),
        card_gap: env_f32("CARD_GAP", 8.0),
        card_radius: env_f32("CARD_RADIUS", 6.0),
        no_match_delay: env_f32("NO_MATCH_DELAY", 0.8),

        color_bg: env_color("COLOR_BG", Color::new(0.06, 0.07, 0.09, 1.0)),
        color_hidden: env_color("COLOR_HIDDEN", Color::new(0.13, 0.15, 0.18, 1.0)),
        color_hidden_border: env_color("COLOR_HIDDEN_BORDER", Color::new(0.25, 0.30, 0.35, 1.0)),
        color_hidden_hover: env_color("COLOR_HIDDEN_HOVER", Color::new(0.20, 0.24, 0.28, 1.0)),
        color_flipped: env_color("COLOR_FLIPPED", Color::new(0.16, 0.20, 0.26, 1.0)),
        color_flipped_border: env_color("COLOR_FLIPPED_BORDER", Color::new(0.40, 0.70, 0.95, 1.0)),
        color_matched: env_color("COLOR_MATCHED", Color::new(0.10, 0.22, 0.14, 1.0)),
        color_matched_border: env_color("COLOR_MATCHED_BORDER", Color::new(0.30, 0.85, 0.45, 1.0)),
        color_symbol: env_color("COLOR_SYMBOL", Color::new(0.90, 0.95, 1.0, 1.0)),
        color_hidden_symbol: env_color("COLOR_HIDDEN_SYMBOL", Color::new(0.35, 0.40, 0.45, 1.0)),
        color_text: env_color("COLOR_TEXT", Color::new(0.85, 0.92, 1.0, 1.0)),
        color_text_dim: env_color("COLOR_TEXT_DIM", Color::new(0.45, 0.55, 0.65, 1.0)),
        color_header_bg: env_color("COLOR_HEADER_BG", Color::new(0.09, 0.11, 0.13, 1.0)),
        color_accent: env_color("COLOR_ACCENT", Color::new(0.40, 0.85, 0.65, 1.0)),

        color_btn_idle: env_color("COLOR_BTN_IDLE", Color::new(0.18, 0.22, 0.26, 1.0)),
        color_btn_hover: env_color("COLOR_BTN_HOVER", Color::new(0.28, 0.34, 0.40, 1.0)),
        color_btn_pressed: env_color("COLOR_BTN_PRESSED", Color::new(0.14, 0.17, 0.20, 1.0)),
        color_menu_selected: env_color("COLOR_MENU_SELECTED", Color::new(0.20, 0.50, 0.65, 1.0)),
        color_menu_hover: env_color("COLOR_MENU_HOVER", Color::new(0.18, 0.24, 0.30, 1.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playfield_geometry() {
        let ctx = load_config();
        assert_eq!(ctx.playfield_y(), ctx.hud_h);
        assert_eq!(ctx.playfield_h(), ctx.window_h - ctx.hud_h - ctx.footer_h);
    }

    #[test]
    fn test_grid_origin_centers() {
        let ctx = load_config();
        let origin = ctx.grid_origin(200.0, 100.0);
        assert!((origin.x - (ctx.window_w - 200.0) * 0.5).abs() < 0.01);
    }
}
