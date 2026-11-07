// games/pong/src/lib.rs
pub mod components;
pub mod config;
pub mod render;
pub mod systems;

pub use components::{Ball, Paddle};
pub use config::GameContext;
pub use ember_stdlib::app::GameState;
pub use systems::{MatchState, reset_match, update};