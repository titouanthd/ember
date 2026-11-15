// games/pagoda/src/lib.rs
pub mod app;
pub mod art;
pub mod audio;
pub mod components;
pub mod config;
pub mod controls;
pub mod fonts;
pub mod juice;
pub mod paths;
pub mod persistence;
pub mod render;
pub mod scene;
pub mod systems;
pub mod ui;

pub use app::{App, AppState};
pub use components::{Block, SlidingBlock};
pub use config::GameContext;
