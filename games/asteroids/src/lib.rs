// games/asteroids/src/lib.rs
pub mod components;
pub mod config;
pub mod persistence;
pub mod systems; // ← celui-ci doit être présent

// Ré-export de GameState du core pour cohérence avec les autres jeux.
pub use components::{Asteroid, Bullet, Ship};
pub use config::GameContext;
pub use ember_core::app::GameState;
pub use systems::{GameWorld, ShipInput};
