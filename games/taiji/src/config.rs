// games/taiji/src/config.rs
use macroquad::prelude::*;

#[derive(Debug, Clone)]
pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub paddle_w: f32,
    pub paddle_h: f32,
    pub ball_radius: f32,
    pub color_player: Color,
    pub color_ai: Color,
    pub color_ball: Color,
    pub win_score: i32,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 800.0,
            screen_h: 600.0,
            paddle_w: 20.0,
            paddle_h: 120.0,
            ball_radius: 12.0,  
            color_player: BLACK,
            color_ai: DARKGRAY,
            color_ball: BLACK,
            win_score: 5,
        }
    }
}