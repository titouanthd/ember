// games/snake/src/lib.rs
pub mod components;
pub mod config;
pub mod systems;

pub use components::{Cell, Direction, Food, Snake, Wall};
pub use ember_core::app::GameState;
pub use systems::{GameContext, SnakeState, SnakeWorld, TickEvent};
