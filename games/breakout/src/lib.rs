// games/breakout/src/lib.rs
pub mod components;
pub mod systems;
pub mod config;
pub mod levels;

pub use ember_core::app::GameState;
pub use components::{Ball, Paddle, Brick};
pub use config::GameContext;
pub use levels::{Level, BrickData};
pub use systems::{BreakoutWorld, UpdateEvent};