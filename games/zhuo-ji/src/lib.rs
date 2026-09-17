//! Zhuo Ji — Guiyang Mahjong (捉鸡麻将).
//!
//! Layout follows the Ember convention (Phases 14–18):
//! - State struct `Game` in `systems.rs`
//! - `GameContext` in `config.rs`, re-exported here
//! - `main.rs` builds the `Input` snapshot and drives `Game::update`

pub mod ai;
pub mod components;
pub mod config;
pub mod hand;
pub mod scoring;
pub mod systems;
pub mod wall;

pub use config::GameContext;
pub use systems::{Game, GameEvent, HuMethod, Phase};