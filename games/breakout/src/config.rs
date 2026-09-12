// games/breakout/src/config.rs
use crate::systems::GameContext;
use ember_stdlib::config::{env_f32, env_i32, load_dotenv_from};
use macroquad::prelude::Color;
use std::path::PathBuf;
use std::sync::OnceLock;

pub fn load_config() -> GameContext {
    static ENV_LOADED: OnceLock<()> = OnceLock::new();
    ENV_LOADED.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_from(&manifest_dir, "Breakout");
    });

    GameContext {
        screen_w: env_f32("SCREEN_WIDTH", 800.0),
        screen_h: env_f32("SCREEN_HEIGHT", 600.0),
        paddle_width: env_f32("PADDLE_WIDTH", 80.0),
        paddle_height: env_f32("PADDLE_HEIGHT", 20.0),
        ball_size: env_f32("BALL_SIZE", 20.0),
        ball_speed: env_f32("BALL_SPEED", 400.0),
        brick_rows: env_i32("BRICK_ROWS", 5) as usize,
        brick_cols: env_i32("BRICK_COLS", 8) as usize,
        brick_width: env_f32("BRICK_WIDTH", 70.0),
        brick_height: env_f32("BRICK_HEIGHT", 25.0),
        brick_padding: env_f32("BRICK_PADDING", 10.0),
        win_score: env_i32("WIN_SCORE", 10),
        paddle_color: Color::new(1.0, 1.0, 1.0, 1.0),
        ball_color: Color::new(1.0, 0.65, 0.0, 1.0),
        brick_colors: vec![
            Color::new(1.0, 0.0, 0.0, 1.0),
            Color::new(1.0, 0.5, 0.0, 1.0),
            Color::new(1.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.0, 1.0, 1.0),
        ],
    }
}