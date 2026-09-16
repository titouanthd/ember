use std::path::PathBuf;

use macroquad::prelude::{Color, Vec2};

use ember_stdlib::config::{env_color, env_f32};

/// Immutable per-run config, loaded once from `.env`.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub window_w: f32,
    pub window_h: f32,
    pub hud_h: f32,

    // --- Player ---
    pub player_speed: f32,
    pub player_focus_mult: f32,
    pub player_radius: f32,
    pub player_hitbox_radius: f32,
    pub player_graze_radius: f32,
    pub player_fire_rate: f32,

    // --- Bullets ---
    pub bullet_player_speed: f32,
    pub bullet_player_radius: f32,
    pub bullet_enemy_speed: f32,
    pub bullet_enemy_radius: f32,
    pub bullet_ttl: f32,

    // --- Colors ---
    pub color_bg: Color,
    pub color_player: Color,
    pub color_player_focus: Color,
    pub color_hitbox: Color,
    pub color_hud: Color,
    pub color_bullet_player: Color,
    pub color_bullet_enemy: Color,
    pub color_enemy: Color,
}

impl GameContext {
    /// Playfield width in playfield-space pixels.
    pub fn playfield_w(&self) -> f32 {
        self.window_w
    }

    /// Playfield height (window minus HUD strip).
    pub fn playfield_h(&self) -> f32 {
        self.window_h - self.hud_h
    }

    /// Convert a playfield-space point to window-space (adds HUD offset).
    pub fn to_window(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x, p.y + self.hud_h)
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ember_stdlib::config::load_dotenv_once(&manifest_dir, "Bullet Hell");

    GameContext {
        window_w: env_f32("WINDOW_W", 800.0),
        window_h: env_f32("WINDOW_H", 600.0),
        hud_h: env_f32("HUD_H", 20.0),

        player_speed: env_f32("PLAYER_SPEED", 280.0),
        player_focus_mult: env_f32("PLAYER_FOCUS_MULT", 0.45),
        player_radius: env_f32("PLAYER_RADIUS", 6.0),
        player_hitbox_radius: env_f32("PLAYER_HITBOX_RADIUS", 2.5),
        player_graze_radius: env_f32("PLAYER_GRAZE_RADIUS", 12.0),
        player_fire_rate: env_f32("PLAYER_FIRE_RATE", 0.12),

        bullet_player_speed: env_f32("BULLET_PLAYER_SPEED", 520.0),
        bullet_player_radius: env_f32("BULLET_PLAYER_RADIUS", 3.0),
        bullet_enemy_speed: env_f32("BULLET_ENEMY_SPEED", 90.0),
        bullet_enemy_radius: env_f32("BULLET_ENEMY_RADIUS", 5.0),
        bullet_ttl: env_f32("BULLET_TTL", 12.0),

        color_bg: env_color("COLOR_BG", Color::new(0.04, 0.04, 0.08, 1.0)),
        color_player: env_color("COLOR_PLAYER", Color::new(0.55, 0.95, 1.0, 1.0)),
        color_player_focus: env_color("COLOR_PLAYER_FOCUS", Color::new(1.0, 1.0, 1.0, 1.0)),
        color_hitbox: env_color("COLOR_HITBOX", Color::new(1.0, 0.2, 0.4, 1.0)),
        color_hud: env_color("COLOR_HUD", Color::new(0.85, 0.9, 1.0, 1.0)),
        color_bullet_player: env_color("COLOR_BULLET_PLAYER", Color::new(1.0, 1.0, 0.4, 1.0)),
        color_bullet_enemy: env_color("COLOR_BULLET_ENEMY", Color::new(1.0, 0.35, 0.6, 1.0)),
        color_enemy: env_color("COLOR_ENEMY", Color::new(0.9, 0.3, 0.5, 1.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playfield_dimensions() {
        let ctx = load_config();
        assert_eq!(ctx.playfield_w(), ctx.window_w);
        assert_eq!(ctx.playfield_h(), ctx.window_h - ctx.hud_h);
    }

    #[test]
    fn test_to_window_adds_hud_offset() {
        let ctx = load_config();
        let p = ctx.to_window(Vec2::new(10.0, 20.0));
        assert_eq!(p, Vec2::new(10.0, 20.0 + ctx.hud_h));
    }
}
