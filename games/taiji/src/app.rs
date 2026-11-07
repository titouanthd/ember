// games/taiji/src/app.rs
use macroquad::prelude::*;
use ember_stdlib::input::Input;
use ember_stdlib::graphics::text::draw_centered;
use crate::config::GameContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    MainMenu,
    Playing,
    Paused,
    GameOver,
}

pub struct App {
    pub state: AppState,
    pub score_player: i32,
    pub score_ai: i32,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::MainMenu,
            score_player: 0,
            score_ai: 0,
        }
    }

    pub fn update(&mut self, input: &Input) {
        match self.state {
            AppState::MainMenu => {
                if input.is_key_pressed(KeyCode::Space) {
                    self.state = AppState::Playing;
                    self.score_player = 0;
                    self.score_ai = 0;
                }
                if input.is_key_pressed(KeyCode::Escape) {
                    std::process::exit(0);
                }
            }
            AppState::Playing => {
                if input.is_key_pressed(KeyCode::Escape) {
                    self.state = AppState::Paused;
                }
            }
            AppState::Paused => {
                if input.is_key_pressed(KeyCode::Escape) {
                    self.state = AppState::Playing;
                }
            }
            AppState::GameOver => {
                if input.is_key_pressed(KeyCode::Space) {
                    self.state = AppState::MainMenu;
                }
            }
        }
    }

    pub fn draw(&self, ctx: &GameContext) {
        let cx = ctx.screen_w / 2.0;

        match self.state {
            AppState::MainMenu => {
                draw_centered("TÀIJÍ", cx, 200.0, 80, BLACK);
                draw_centered("Press SPACE to start", cx, 300.0, 30, DARKGRAY);
                draw_centered("Press ESC to quit", cx, 340.0, 30, DARKGRAY);
            }
            AppState::Playing => {
                let score_text = format!("{}  |  {}", self.score_player, self.score_ai);
                draw_centered(&score_text, cx, 60.0, 40, BLACK);
            }
            AppState::Paused => {
                draw_centered("PAUSED", cx, 250.0, 60, BLACK);
                draw_centered("Press ESC to resume", cx, 320.0, 30, DARKGRAY);
            }
            AppState::GameOver => {
                let winner = if self.score_player > self.score_ai { "YANG WINS" } else { "YIN WINS" };
                draw_centered(winner, cx, 250.0, 60, BLACK);
                draw_centered("Press SPACE to return", cx, 320.0, 30, DARKGRAY);
            }
        }
    }
}