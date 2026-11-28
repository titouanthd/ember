// ember_stdlib/src/lib.rs
//! Ember stdlib : briques de jeu réutilisables, sur les types du moteur
//! (`ember_core`). Aucune dépendance à macroquad (Phase 8, ADR-033).
//!
//! Ce crate ré-exporte `ember_core` pour simplifier les imports dans les jeux.
//! Il contient les composants de jeu (Transform, Sprite, Collider, config),
//! les briques réutilisables (grid, fx, layout, clock, music, persistence).
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
pub mod layout;

pub mod grid;
pub use grid::Grid;

// Engine-typed juice, drawn through a RenderingServer (pagoda round).
pub mod fx;

pub mod clock;

pub mod persistence;

pub mod music;

// Aligned, shadowed and bold text on a RenderingServer (v0.2 V1.4).
pub mod text;
