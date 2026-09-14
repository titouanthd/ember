//! Immutable per-run config, loaded once from `.env`.

use std::path::PathBuf;

use macroquad::prelude::Color;

use ember_stdlib::config::{env_color, env_f32, load_dotenv_once};

/// Immutable per-run config, loaded once from `.env`.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub window_w: f32,
    pub window_h: f32,
    pub hud_h: f32,
    pub footer_h: f32,

    // Card layout
    pub card_w: f32,
    pub card_h: f32,
    pub column_gap: f32,
    pub table_offset_y: f32,
    pub card_stack_offset: f32,
    pub use_unicode: bool,

    // Colors
    pub color_bg: Color,
    pub color_card_bg: Color,
    pub color_card_border: Color,
    pub color_card_red: Color,
    pub color_card_black: Color,
    pub color_card_back: Color,
    pub color_slot_empty: Color,
    pub color_slot_border: Color,
    pub color_slot_hover: Color,
    pub color_drag_shadow: Color,
    pub color_text: Color,
    pub color_text_dim: Color,
    pub color_header_bg: Color,
    pub color_accent: Color,

    // Buttons
    pub color_btn_idle: Color,
    pub color_btn_hover: Color,
    pub color_btn_pressed: Color,
    pub color_menu_selected: Color,
    pub color_menu_hover: Color,
}

impl GameContext {
    pub fn playfield_y(&self) -> f32 {
        self.hud_h
    }

    pub fn playfield_h(&self) -> f32 {
        self.window_h - self.hud_h - self.footer_h
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    load_dotenv_once(&manifest_dir, "FreeCell");

    GameContext {
        window_w: env_f32("WINDOW_W", 1000.0),
        window_h: env_f32("WINDOW_H", 700.0),
        hud_h: env_f32("HUD_H", 50.0),
        footer_h: env_f32("FOOTER_H", 30.0),

        card_w: env_f32("CARD_W", 80.0),
        card_h: env_f32("CARD_H", 110.0),
        column_gap: env_f32("COLUMN_GAP", 10.0),
        table_offset_y: env_f32("TABLE_OFFSET_Y", 140.0),
        card_stack_offset: env_f32("CARD_STACK_OFFSET", 30.0),
        use_unicode: true,

        color_bg: env_color("COLOR_BG", Color::new(0.06, 0.07, 0.09, 1.0)),
        color_card_bg: env_color("COLOR_CARD_BG", Color::new(0.92, 0.93, 0.95, 1.0)),
        color_card_border: env_color("COLOR_CARD_BORDER", Color::new(0.10, 0.12, 0.15, 1.0)),
        color_card_red: env_color("COLOR_CARD_RED", Color::new(0.80, 0.15, 0.20, 1.0)),
        color_card_black: env_color("COLOR_CARD_BLACK", Color::new(0.10, 0.12, 0.15, 1.0)),
        color_card_back: env_color("COLOR_CARD_BACK", Color::new(0.20, 0.30, 0.45, 1.0)),
        color_slot_empty: env_color("COLOR_SLOT_EMPTY", Color::new(0.10, 0.12, 0.15, 1.0)),
        color_slot_border: env_color("COLOR_SLOT_BORDER", Color::new(0.25, 0.30, 0.35, 1.0)),
        color_slot_hover: env_color("COLOR_SLOT_HOVER", Color::new(0.20, 0.35, 0.45, 1.0)),
        color_drag_shadow: env_color("COLOR_DRAG_SHADOW", Color::new(0.0, 0.0, 0.0, 0.5)),
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
}