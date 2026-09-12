// crates/ember-core/src/math/mod.rs
//! Primitives mathématiques du moteur.
//!
//! Contient les types géométriques (Aabb, Circle) utilisés pour les
//! collisions et les tests d'intersection. Aucune dépendance graphique.

pub mod aabb;
pub mod circle;

pub use aabb::Aabb;
pub use circle::Circle;