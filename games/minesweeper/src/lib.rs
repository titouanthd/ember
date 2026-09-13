//! Minesweeper — a classic puzzle game.

pub mod components;
pub mod config;
pub mod difficulties;
pub mod persistence;
pub mod systems;

pub use ember_core::app::GameState;
pub use systems::Game;