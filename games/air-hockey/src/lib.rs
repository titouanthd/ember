//! Air Hockey — 2-player couch game.
//!
//! Layout follows the Ember convention (Phases 14–18):
//! - State struct `Game` in `systems.rs`
//! - `GameContext` in `config.rs`, re-exported here
//! - `main.rs` builds the `Input` snapshot and drains `GameEvent`s

pub mod ai;
pub mod arena;
pub mod audio;
pub mod components;
pub mod config;
pub mod effects;
pub mod font;
pub mod physics;
pub mod systems;

pub use config::GameContext;
pub use systems::{Game, GameEvent, GameMode, Phase};