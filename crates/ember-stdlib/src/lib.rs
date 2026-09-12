// ember_stdlib/src/lib.rs
//! Ember stdlib : briques de jeu réutilisables, dépendantes de macroquad.
//!
//! Ce crate ré-exporte `ember_core` pour simplifier les imports dans les jeux.
//! Il contient les composants de jeu (Transform, Sprite, Collider, config),
//! et depuis Session A de Minesweeper : grid, input, et ui.
//!
//! ⚠️ Ne PAS mettre ici de types spécifiques à un jeu (Level, BrickData…).
//! Chaque jeu définit son propre format de niveau.

pub use ember_core::*;

pub mod transform;
pub use transform::Transform;

pub mod sprite;
pub use sprite::Sprite;

pub mod collider;
pub use collider::{Collider, Shape, collides};

pub mod config;
pub mod graphics;

// --- Nouveaux modules (Session A Minesweeper) ---

pub mod grid;
pub use grid::Grid;

pub mod input;
pub use input::Input;

pub mod ui;
// `ui` ré-exporte déjà ses items depuis ui/mod.rs, pas besoin de pub use ici.