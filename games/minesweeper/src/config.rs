use std::path::PathBuf;

use macroquad::prelude::{Color, Vec2};

use ember_stdlib::config::{env_color, env_f32};

/// Immutable per-run config, loaded once from `.env`.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub window_w: f32,
    pub window_h: f32,
    pub hud_h: f32,
    pub footer_h: f32,

    // Cells
    pub color_bg: Color,
    pub color_cell_hidden: Color,
    pub color_cell_hover: Color,
    pub color_cell_revealed: Color,
    pub color_cell_mine: Color,
    pub color_flag: Color,
    pub color_grid_line: Color,

    // Chrome
    pub color_header_bg: Color,
    pub color_text: Color,
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

    /// Top-left origin of a grid of size (width * cell, height * cell),
    /// centered in the playfield.
    pub fn grid_origin(&self, grid_w: f32, grid_h: f32) -> Vec2 {
        let x = (self.window_w - grid_w) * 0.5;
        let y = self.playfield_y() + (self.playfield_h() - grid_h) * 0.5;
        Vec2::new(x, y)
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ember_stdlib::config::load_dotenv_once(&manifest_dir, "Minesweeper");

    GameContext {
        window_w: env_f32("WINDOW_W", 900.0),
        window_h: env_f32("WINDOW_H", 600.0),
        hud_h: env_f32("HUD_H", 50.0),
        footer_h: env_f32("FOOTER_H", 30.0),

        color_bg: env_color("COLOR_BG", Color::new(0.10, 0.11, 0.13, 1.0)),
        color_cell_hidden: env_color("COLOR_CELL_HIDDEN", Color::new(0.30, 0.32, 0.36, 1.0)),
        color_cell_hover: env_color("COLOR_CELL_HOVER", Color::new(0.42, 0.44, 0.48, 1.0)),
        color_cell_revealed: env_color("COLOR_CELL_REVEALED", Color::new(0.18, 0.19, 0.22, 1.0)),
        color_cell_mine: env_color("COLOR_CELL_MINE", Color::new(0.85, 0.20, 0.25, 1.0)),
        color_flag: env_color("COLOR_FLAG", Color::new(0.95, 0.55, 0.20, 1.0)),
        color_grid_line: env_color("COLOR_GRID_LINE", Color::new(0.08, 0.09, 0.11, 1.0)),

        color_header_bg: env_color("COLOR_HEADER_BG", Color::new(0.15, 0.16, 0.19, 1.0)),
        color_text: env_color("COLOR_TEXT", Color::new(0.90, 0.92, 0.95, 1.0)),
        color_btn_idle: env_color("COLOR_BTN_IDLE", Color::new(0.25, 0.27, 0.32, 1.0)),
        color_btn_hover: env_color("COLOR_BTN_HOVER", Color::new(0.35, 0.38, 0.44, 1.0)),
        color_btn_pressed: env_color("COLOR_BTN_PRESSED", Color::new(0.20, 0.22, 0.26, 1.0)),

        color_menu_selected: env_color("COLOR_MENU_SELECTED", Color::new(0.30, 0.55, 0.90, 1.0)),
        color_menu_hover: env_color("COLOR_MENU_HOVER", Color::new(0.25, 0.30, 0.38, 1.0)),
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
        // Should be horizontally centered.
        assert!((origin.x - (ctx.window_w - 200.0) * 0.5).abs() < 0.01);
    }
}