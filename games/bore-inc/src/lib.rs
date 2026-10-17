//! Bore Inc. — 14th Ember game.
//!
//! An incremental mining game. Archie the cat drills a tunnel through
//! a mountain with robot employees, so the Bore Inc. truck can cross.
//! Restriction: movement input only. See DESIGN.md §2.

pub mod atlas;
pub mod audio;
pub mod background;
pub mod bag;
pub mod components;
pub mod config;
pub mod drop;
pub mod fonts;
pub mod hq;
pub mod hud;
pub mod input;
pub mod levelgen;
pub mod paths;
pub mod pixel;
pub mod player;
pub mod progress;
pub mod rng;
pub mod sprites;
pub mod systems;
pub mod truck;
pub mod upgrade;
pub mod world;

pub use atlas::{Atlas, AtlasBuilder, BakedAtlas, SpriteRect};
pub use background::Background;
pub use bag::Bag;
pub use components::{ResourceKind, Tile, TileKind, TILE_SIZE};
pub use config::GameContext;
pub use drop::Drop;
pub use hq::HqState;
pub use input::{Facing, GameInput};
pub use levelgen::{generate, LevelKind, LevelParams};
pub use pixel::{Canvas, Palette, Rgba};
pub use player::{Archie, BreakResult};
pub use progress::Progress;
pub use rng::Rng;
pub use sprites::archie::ArchieSprites;
pub use sprites::truck::TRUCK_CELL;
pub use systems::{AppAction, AppScreen, AppState, Game};
pub use truck::{BlockReason, Truck};
pub use upgrade::{UpgradeDef, UPGRADES};
pub use world::World;