// crates/ember-core/src/lib.rs
pub mod app;
pub mod hash;
pub mod io;
pub mod math;
pub mod rng;
pub mod time;

pub use app::GameState;
pub use hash::{hash_u32, hash_u32_2, hash_u32_3};
pub use math::{Aabb, Circle, reflect};
pub use rng::Rng;
pub use time::{Cooldown, TickTimer};