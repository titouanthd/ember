// crates/ember-core/src/time/mod.rs
//! Primitives temporelles du moteur.
//!
//! Contient `TickTimer`, un accumulateur de temps pour les simulations
//! à pas fixe (Snake, Dolly Dot, Tower Defense, etc.).

pub mod tick;

pub use tick::TickTimer;