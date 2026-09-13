// games/asteroids/src/lib.rs
pub mod components;
pub mod systems;
pub mod config;
pub mod persistence;   // ← celui-ci doit être présent

// Ré-export de GameState du core pour cohérence avec les autres jeux.
pub use ember_core::app::GameState;
pub use components::{Asteroid, Bullet, Ship};
pub use config::GameContext;
pub use systems::{GameWorld, ShipInput};