// crates/ember-core/src/app/mod.rs
//! Briques d'application génériques.
//!
//! Contient `GameState`, l'énumération d'états de jeu partagée par tous
//! les jeux Ember : Start, Playing, LevelCleared, GameOver, Win.

pub mod state;

pub use state::GameState;