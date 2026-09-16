//! FreeCell — a solitaire card game with a developer terminal theme.

pub mod components;
pub mod config;
pub mod deck;
pub mod drag;
pub mod font;
pub mod layout;
pub mod persistence;
pub mod systems;

pub use ember_core::app::GameState;
pub use systems::Game;
