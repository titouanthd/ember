// games/taiji/src/app.rs
use crate::config::{BLACK, DARKGRAY, GameContext};
use ember_core::Color;
use ember_input::{Input, Key};
use ember_servers::RenderingServer;
use glam::Vec2;

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

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::MainMenu,
            score_player: 0,
            score_ai: 0,
        }
    }

    /// Menu navigation. Returns `false` to quit (Escape on the main menu).
    pub fn update(&mut self, input: &Input) -> bool {
        match self.state {
            AppState::MainMenu => {
                if input.is_key_pressed(Key::Space) {
                    self.state = AppState::Playing;
                    self.score_player = 0;
                    self.score_ai = 0;
                }
                if input.is_key_pressed(Key::Escape) {
                    return false;
                }
            }
            AppState::Playing => {
                if input.is_key_pressed(Key::Escape) {
                    self.state = AppState::Paused;
                }
            }
            AppState::Paused => {
                if input.is_key_pressed(Key::Escape) {
                    self.state = AppState::Playing;
                }
            }
            AppState::GameOver => {
                if input.is_key_pressed(Key::Space) {
                    self.state = AppState::MainMenu;
                }
            }
        }
        true
    }

    pub fn draw(&self, ctx: &GameContext, r: &mut dyn RenderingServer) {
        let cx = ctx.screen_w / 2.0;

        match self.state {
            AppState::MainMenu => {
                centered(r, "TÀIJÍ", cx, 200.0, 80.0, BLACK);
                centered(r, "Press SPACE to start", cx, 300.0, 30.0, DARKGRAY);
                centered(r, "Press ESC to quit", cx, 340.0, 30.0, DARKGRAY);
            }
            AppState::Playing => {
                let score_text = format!("{}  |  {}", self.score_player, self.score_ai);
                centered(r, &score_text, cx, 60.0, 40.0, BLACK);
            }
            AppState::Paused => {
                centered(r, "PAUSED", cx, 250.0, 60.0, BLACK);
                centered(r, "Press ESC to resume", cx, 320.0, 30.0, DARKGRAY);
            }
            AppState::GameOver => {
                let winner = if self.score_player > self.score_ai {
                    "YANG WINS"
                } else {
                    "YIN WINS"
                };
                centered(r, winner, cx, 250.0, 60.0, BLACK);
                centered(r, "Press SPACE to return", cx, 320.0, 30.0, DARKGRAY);
            }
        }
    }
}

/// Text centred on `center_x`, with its baseline at `y` (as
/// `ember_stdlib::graphics::text::draw_centered` did).
pub fn centered(
    r: &mut dyn RenderingServer,
    text: &str,
    center_x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    let m = r.measure_text(text, size);
    r.draw_text(text, Vec2::new(center_x - m.width / 2.0, y), size, color);
}
