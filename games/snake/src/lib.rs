// games/snake/src/lib.rs
pub mod components;
pub mod systems;
pub mod config;

pub use ember_core::app::GameState;
pub use components::{Cell, Direction, Food, Snake, Wall};
pub use systems::{GameContext, TickEvent, SnakeState, SnakeWorld};