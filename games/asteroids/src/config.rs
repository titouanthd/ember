// games/asteroids/src/config.rs
use ember_stdlib::config::{env_f32, env_u32, load_dotenv_once};
use macroquad::prelude::Color;
use std::path::PathBuf;

/// Immutable per-run config, loaded once from `.env`.
///
/// Same convention as the other games: `GameContext` lives in `config.rs`
/// next to its loader. `systems.rs` imports it from here.
pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,

    pub ship_radius: f32,
    pub ship_rotation_speed: f32,
    pub ship_acceleration: f32,
    pub ship_max_speed: f32,
    pub invincibility_secs: f32,

    pub bullet_radius: f32,
    pub bullet_speed: f32,
    pub bullet_lifetime: f32,
    pub bullet_cooldown_ms: u32,

    pub asteroid_count_start: u32,
    pub asteroid_speed_min: f32,
    pub asteroid_speed_max: f32,

    pub lives_start: u32,
    pub max_waves: u32,
    pub wave_transition_secs: f32,

    pub ship_color: Color,
    pub bullet_color: Color,
    pub asteroid_color: Color,
    pub bg_color: Color,
    pub star_color: Color,
    pub ui_text_color: Color,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 900.0,
            screen_h: 700.0,
            ship_radius: 14.0,
            ship_rotation_speed: 4.0,
            ship_acceleration: 400.0,
            ship_max_speed: 400.0,
            invincibility_secs: 1.5,
            bullet_radius: 3.0,
            bullet_speed: 600.0,
            bullet_lifetime: 1.0,
            bullet_cooldown_ms: 250,
            asteroid_count_start: 4,
            asteroid_speed_min: 50.0,
            asteroid_speed_max: 150.0,
            lives_start: 3,
            max_waves: 5,
            wave_transition_secs: 1.5,
            ship_color: Color::new(0.9, 0.95, 1.0, 1.0),
            bullet_color: Color::new(1.0, 0.95, 0.5, 1.0),
            asteroid_color: Color::new(0.7, 0.65, 0.6, 1.0),
            bg_color: Color::new(0.04, 0.04, 0.07, 1.0),
            star_color: Color::new(0.5, 0.5, 0.6, 0.8),
            ui_text_color: Color::new(0.85, 0.88, 0.92, 1.0),
        }
    }
}

impl GameContext {
    /// Nombre d'astéroïdes pour une vague donnée (1-indexée).
    /// Vague 1 : 4, vague 2 : 6, vague 3 : 8, etc.
    pub fn asteroids_for_wave(&self, wave: u32) -> u32 {
        self.asteroid_count_start + (wave.saturating_sub(1)) * 2
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    load_dotenv_once(&manifest_dir, "Asteroids");

    GameContext {
        screen_w: env_f32("SCREEN_WIDTH", 900.0),
        screen_h: env_f32("SCREEN_HEIGHT", 700.0),

        ship_radius: env_f32("SHIP_RADIUS", 14.0),
        ship_rotation_speed: env_f32("SHIP_ROTATION_SPEED", 4.0),
        ship_acceleration: env_f32("SHIP_ACCELERATION", 400.0),
        ship_max_speed: env_f32("SHIP_MAX_SPEED", 400.0),
        invincibility_secs: env_u32("INVINCIBILITY_MS", 1500) as f32 / 1000.0,

        bullet_radius: env_f32("BULLET_RADIUS", 3.0),
        bullet_speed: env_f32("BULLET_SPEED", 600.0),
        bullet_lifetime: env_f32("BULLET_LIFETIME", 1.0),
        bullet_cooldown_ms: env_u32("BULLET_COOLDOWN_MS", 250),

        asteroid_count_start: env_u32("ASTEROID_COUNT_START", 4),
        asteroid_speed_min: env_f32("ASTEROID_SPEED_MIN", 50.0),
        asteroid_speed_max: env_f32("ASTEROID_SPEED_MAX", 150.0),

        lives_start: env_u32("LIVES_START", 3),
        max_waves: env_u32("MAX_WAVES", 5),
        wave_transition_secs: env_u32("WAVE_TRANSITION_MS", 1500) as f32 / 1000.0,

        ship_color: Color::new(0.9, 0.95, 1.0, 1.0),
        bullet_color: Color::new(1.0, 0.95, 0.5, 1.0),
        asteroid_color: Color::new(0.7, 0.65, 0.6, 1.0),
        bg_color: Color::new(0.04, 0.04, 0.07, 1.0),
        star_color: Color::new(0.5, 0.5, 0.6, 0.8),
        ui_text_color: Color::new(0.85, 0.88, 0.92, 1.0),
    }
}