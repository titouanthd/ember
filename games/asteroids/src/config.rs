// games/asteroids/src/config.rs
use crate::systems::GameContext;
use ember_stdlib::config::{env_f32, env_u32, load_dotenv_from};
use macroquad::prelude::Color;
use std::path::PathBuf;
use std::sync::OnceLock;

pub fn load_config() -> GameContext {
    static ENV_LOADED: OnceLock<()> = OnceLock::new();
    ENV_LOADED.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_from(&manifest_dir, "Asteroids");
    });

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