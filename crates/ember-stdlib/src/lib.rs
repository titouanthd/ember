// ember_stdlib/src/lib.rs
//! Ember stdlib : briques de jeu réutilisables, dépendantes de macroquad.
//!
//! Ce crate ré-exporte `ember_core` pour simplifier les imports dans les jeux.
//! Il contient les composants de jeu (Transform, Sprite, Collider, config),
//! les briques réutilisables (grid, input, ui, audio).
//!
//! ⚠️ Ne PAS mettre ici de types spécifiques à un jeu (Level, BrickData…).

pub use ember_core::*;

pub mod transform;
pub use transform::Transform;

pub mod sprite;
pub use sprite::Sprite;

pub mod collider;
pub use collider::{Collider, Shape, collides};

pub mod config;
pub mod graphics;

pub mod grid;
pub use grid::Grid;

pub mod input;
pub use input::Input;

pub mod ui;

pub mod persistence;

pub mod audio;
pub use audio::AudioClip;
