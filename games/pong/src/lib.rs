// games/pong/src/lib.rs
pub mod components;
pub mod systems;
pub mod config;

pub use components::{Paddle, Ball};
pub use ember_core::app::GameState;
pub use config::GameContext;
pub use systems::{update, reset_game, MatchState};