// games/pong/src/config.rs
use ember_stdlib::config::{env_color, env_f32, env_i32, load_dotenv_once};
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
    pub paddle_w: f32,
    pub paddle_h: f32,
    pub ball_size: f32,
    pub base_speed: f32,
    pub win_score: i32,
    pub player_color: Color,
    pub ai_color: Color,
    pub ball_color: Color,
    pub ball_speed_increment: f32,
    pub player_speed: f32,
    pub ai_speed: f32,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 800.0,
            screen_h: 600.0,
            paddle_w: 15.0,
            paddle_h: 80.0,
            ball_size: 20.0,
            base_speed: 400.0,
            win_score: 5,
            player_color: Color::new(1.0, 1.0, 1.0, 1.0),
            ai_color: Color::new(1.0, 1.0, 1.0, 1.0),
            ball_color: Color::new(1.0, 0.65, 0.0, 1.0),
            ball_speed_increment: 1.05,
            player_speed: 350.0,
            ai_speed: 300.0,
        }
    }
}

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    load_dotenv_once(&manifest_dir, "Pong");

    GameContext {
        screen_w: env_f32("SCREEN_WIDTH", 800.0),
        screen_h: env_f32("SCREEN_HEIGHT", 600.0),
        paddle_w: env_f32("PADDLE_WIDTH", 15.0),
        paddle_h: env_f32("PADDLE_HEIGHT", 80.0),
        ball_size: env_f32("BALL_SIZE", 20.0),
        base_speed: env_f32("BALL_SPEED", 400.0),
        win_score: env_i32("WIN_SCORE", 5),
        player_color: env_color("PLAYER_COLOR", Color::new(1.0, 1.0, 1.0, 1.0)),
        ai_color: env_color("AI_COLOR", Color::new(1.0, 1.0, 1.0, 1.0)),
        ball_color: env_color("BALL_COLOR", Color::new(1.0, 0.65, 0.0, 1.0)),
        ball_speed_increment: env_f32("BALL_SPEED_INCREMENT", 1.05),
        player_speed: env_f32("PLAYER_SPEED", 350.0),
        ai_speed: env_f32("AI_SPEED", 300.0),
    }
}
