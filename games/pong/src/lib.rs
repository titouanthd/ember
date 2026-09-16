// games/pong/src/lib.rs
pub mod components;
pub mod config;
pub mod systems;

pub use components::{Ball, Paddle};
pub use config::GameContext;
pub use ember_core::app::GameState;
pub use systems::{MatchState, reset_game, update};
