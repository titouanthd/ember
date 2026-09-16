// games/breakout/src/lib.rs
pub mod components;
pub mod config;
pub mod levels;
pub mod systems;

pub use components::{Ball, Brick, Paddle};
pub use config::GameContext;
pub use ember_core::app::GameState;
pub use levels::{BrickData, Level};
pub use systems::{BreakoutWorld, UpdateEvent};
