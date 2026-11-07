// games/pong/src/config.rs
use ember_stdlib::config::{env_color, env_f32, env_i32, load_dotenv_once};
use macroquad::prelude::Color;
use std::path::PathBuf;

/// Immutable per-run config, loaded once from `.env`.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub paddle_w: f32,
    pub paddle_h: f32,
    pub ball_size: f32,
    pub base_speed: f32,
    /// Hard cap on ball speed magnitude. Prevents runaway acceleration
    /// from repeated paddle hits (`ball_speed_increment` compounding).
    pub max_ball_speed: f32,
    pub win_score: i32,
    pub player_color: Color,
    pub ai_color: Color,
    pub ball_color: Color,
    pub ball_speed_increment: f32,
    pub player_speed: f32,
    pub ai_speed: f32,
    /// How long the AI waits between two re-targets of the ball's Y.
    /// `0.0` = instant (cheating) tracking. Default `0.08` = ~12 Hz.
    pub ai_reaction_time: f32,
    /// Maximum serve angle off horizontal, in radians. `0.0` = always
    /// horizontal serve. Default `0.4` (~23°).
    pub serve_angle_max: f32,
    /// Fraction of the paddle's Y-velocity transferred to the ball on
    /// impact (the arcade "english"). `0.0` = no influence, `0.3` = 30%
    /// of paddle speed is added to the ball's Y-velocity.
    ///
    /// Note: with pure mirror reflection (the paddle is a flat surface),
    /// this is the **only** player-controlled way to change the ball's
    /// angle. Bump to `0.5` for stronger placement, lower to `0.15` for
    /// subtlety.
    pub paddle_velocity_factor: f32,
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
            max_ball_speed: 1000.0,
            win_score: 5,
            player_color: Color::new(1.0, 1.0, 1.0, 1.0),
            ai_color: Color::new(1.0, 1.0, 1.0, 1.0),
            ball_color: Color::new(1.0, 0.65, 0.0, 1.0),
            ball_speed_increment: 1.05,
            player_speed: 350.0,
            ai_speed: 300.0,
            ai_reaction_time: 0.08,
            serve_angle_max: 0.4,
            paddle_velocity_factor: 0.3,
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
        max_ball_speed: env_f32("BALL_MAX_SPEED", 1000.0),
        win_score: env_i32("WIN_SCORE", 5),
        player_color: env_color("PLAYER_COLOR", Color::new(1.0, 1.0, 1.0, 1.0)),
        ai_color: env_color("AI_COLOR", Color::new(1.0, 1.0, 1.0, 1.0)),
        ball_color: env_color("BALL_COLOR", Color::new(1.0, 0.65, 0.0, 1.0)),
        ball_speed_increment: env_f32("BALL_SPEED_INCREMENT", 1.05),
        player_speed: env_f32("PLAYER_SPEED", 350.0),
        ai_speed: env_f32("AI_SPEED", 300.0),
        ai_reaction_time: env_f32("AI_REACTION_TIME", 0.08),
        serve_angle_max: env_f32("SERVE_ANGLE_MAX", 0.4),
        paddle_velocity_factor: env_f32("PADDLE_VELOCITY_FACTOR", 0.3),
    }
}