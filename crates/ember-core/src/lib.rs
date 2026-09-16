// crates/ember-core/src/lib.rs
pub mod app;
pub mod io;
pub mod math;
pub mod rng;
pub mod time;

pub use app::GameState;
pub use math::{Aabb, Circle};
pub use rng::Rng;
pub use time::TickTimer;
