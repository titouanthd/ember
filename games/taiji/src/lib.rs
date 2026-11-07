// games/taiji/src/lib.rs
pub mod ai;
pub mod app;
pub mod components;
pub mod config;
pub mod render;
pub mod systems;

pub use ai::AiController;
pub use app::{App, AppState};
pub use components::{Paddle, QiBall, Side};
pub use config::GameContext;