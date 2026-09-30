//! jade-garden — 13th Ember game.
//!
//! A contemplative match-3 set in an abandoned imperial garden.
//! The player restores the jades and recomposes a Tang poem.

pub mod animation;
pub mod audio;
pub mod components;
pub mod config;
pub mod fonts;
pub mod grid;
pub mod hud;
pub mod juice;
pub mod level;
pub mod menu;
pub mod poem;
pub mod progress;
pub mod scoring;
pub mod scroll_painting;
pub mod systems;
pub mod tile_render;
pub mod help;
pub mod paths;

pub use animation::{Easing, Tween};
pub use components::{Jade, JadeShape, Tile, GRID_H, GRID_W, JADE_TYPES};
pub use grid::Grid;
pub use level::{ChapterConfig, LevelConfig, Objective};
pub use progress::Progress;
pub use scoring::{compute_stars, score_for_match};
pub use systems::{Game, Phase};