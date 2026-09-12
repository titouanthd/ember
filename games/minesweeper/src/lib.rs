//! Minesweeper — a classic puzzle game.
//!
//! Session A : scaffold + inert grid + working UI widgets.
//! No game logic yet — the grid renders and responds to hover, but clicking
//! does nothing. Session B adds the real rules.

pub mod components;
pub mod config;

pub use ember_core::app::GameState;