use std::path::PathBuf;

use glam::Vec2;
use macroquad::prelude::Color;

use ember_stdlib::config::{env_color, env_f32};

#[derive(Debug, Clone)]
pub struct GameContext {
    pub window_w: f32,
    pub window_h: f32,
    pub hud_h: f32,

    pub button_radius: f32,
    pub button_gap: f32,

    pub color_bg: Color,

    pub color_green_lit: Color,
    pub color_green_dim: Color,
    pub color_red_lit: Color,
    pub color_red_dim: Color,
    pub color_yellow_lit: Color,
    pub color_yellow_dim: Color,
    pub color_blue_lit: Color,
    pub color_blue_dim: Color,

    pub color_header_bg: Color,
    pub color_text: Color,
}

impl GameContext {
    pub fn playfield_y(&self) -> f32 {
        self.hud_h
    }

    pub fn playfield_h(&self) -> f32 {
        self.window_h - self.hud_h
    }

    pub fn playfield_center(&self) -> Vec2 {
        Vec2::new(
            self.window_w * 0.5,
            self.playfield_y() + self.playfield_h() * 0.5,
        )
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ember_stdlib::config::load_dotenv_once(&manifest_dir, "Simon");

    GameContext {
        window_w: env_f32("WINDOW_W", 700.0),
        window_h: env_f32("WINDOW_H", 700.0),
        hud_h: env_f32("HUD_H", 60.0),

        button_radius: env_f32("BUTTON_RADIUS", 100.0),
        button_gap: env_f32("BUTTON_GAP", 20.0),

        color_bg: env_color("COLOR_BG", Color::new(0.08, 0.09, 0.11, 1.0)),

        color_green_lit: env_color("COLOR_GREEN_LIT", Color::new(0.30, 0.90, 0.40, 1.0)),
        color_green_dim: env_color("COLOR_GREEN_DIM", Color::new(0.10, 0.28, 0.14, 1.0)),
        color_red_lit: env_color("COLOR_RED_LIT", Color::new(0.95, 0.25, 0.25, 1.0)),
        color_red_dim: env_color("COLOR_RED_DIM", Color::new(0.30, 0.10, 0.10, 1.0)),
        color_yellow_lit: env_color("COLOR_YELLOW_LIT", Color::new(0.98, 0.85, 0.25, 1.0)),
        color_yellow_dim: env_color("COLOR_YELLOW_DIM", Color::new(0.30, 0.26, 0.10, 1.0)),
        color_blue_lit: env_color("COLOR_BLUE_LIT", Color::new(0.30, 0.55, 1.0, 1.0)),
        color_blue_dim: env_color("COLOR_BLUE_DIM", Color::new(0.10, 0.18, 0.30, 1.0)),

        color_header_bg: env_color("COLOR_HEADER_BG", Color::new(0.14, 0.15, 0.18, 1.0)),
        color_text: env_color("COLOR_TEXT", Color::new(0.92, 0.94, 0.97, 1.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playfield_dimensions() {
        let ctx = load_config();
        assert_eq!(ctx.playfield_y(), ctx.hud_h);
        assert_eq!(ctx.playfield_h(), ctx.window_h - ctx.hud_h);
    }

    #[test]
    fn test_playfield_center() {
        let ctx = load_config();
        let c = ctx.playfield_center();
        assert!((c.x - ctx.window_w * 0.5).abs() < 0.01);
    }
}