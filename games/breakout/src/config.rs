// games/breakout/src/config.rs
use ember_stdlib::config::{env_f32, env_i32, load_dotenv_once};
use macroquad::prelude::Color;
use std::path::PathBuf;

/// Immutable per-run config, loaded once from `.env`.
///
/// Same convention as the other games: `GameContext` lives in `config.rs`
/// next to its loader. `systems.rs` imports it from here.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub paddle_width: f32,
    pub paddle_height: f32,
    pub ball_size: f32,
    pub ball_speed: f32,
    pub brick_rows: usize,
    pub brick_cols: usize,
    pub brick_width: f32,
    pub brick_height: f32,
    pub brick_padding: f32,
    pub win_score: i32,
    pub paddle_color: Color,
    pub ball_color: Color,
    pub brick_colors: Vec<Color>,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 800.0,
            screen_h: 600.0,
            paddle_width: 80.0,
            paddle_height: 20.0,
            ball_size: 20.0,
            ball_speed: 400.0,
            brick_rows: 5,
            brick_cols: 8,
            brick_width: 70.0,
            brick_height: 25.0,
            brick_padding: 10.0,
            win_score: 10,
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
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    load_dotenv_once(&manifest_dir, "Breakout");

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
