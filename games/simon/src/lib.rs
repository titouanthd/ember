//! Simon — a memory game with 4 colored buttons, running on the Ember
//! node tree (`scene`).

pub mod components;
pub mod config;
pub mod persistence;
pub mod scene;
pub mod systems;

pub use ember_core::app::GameState;
