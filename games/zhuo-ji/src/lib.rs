//! Zhuo Ji — Guiyang Mahjong (捉鸡麻将).

pub mod ai;
pub mod components;
pub mod config;
pub mod font;
pub mod hand;
pub mod hand_summary;
pub mod help;
pub mod layout;
pub mod menu;
pub mod scoring;
pub mod systems;
pub mod tiles;
pub mod ui;
pub mod wall;

pub use config::GameContext;
pub use font::TileFont;
pub use layout::{Seat, TableLayout, TileSize, TileSizes};
pub use systems::{Game, GameEvent, HuMethod, Phase};