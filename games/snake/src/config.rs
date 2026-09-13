// games/snake/src/config.rs
use crate::systems::GameContext;
use ember_stdlib::config::{env_f32, env_u32};
use macroquad::prelude::Color;
use std::path::PathBuf;

pub fn load_config() -> GameContext {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ember_stdlib::config::load_dotenv_once(&manifest_dir, "Snake");

    GameContext {
        grid_cols: env_u32("GRID_COLS", 20),
        grid_rows: env_u32("GRID_ROWS", 15),
        cell_size: env_f32("CELL_SIZE", 30.0),
        initial_tick_ms: env_u32("INITIAL_TICK_MS", 180),
        min_tick_ms: env_u32("MIN_TICK_MS", 60),
        tick_speedup_ms: env_u32("TICK_SPEEDUP_MS", 8),
        food_per_speedup: env_u32("FOOD_PER_SPEEDUP", 5),
        bg_color: Color::new(0.05, 0.05, 0.08, 1.0),
        grid_color: Color::new(0.12, 0.12, 0.16, 1.0),
        snake_head_color: Color::new(0.3, 0.95, 0.4, 1.0),
        snake_body_color: Color::new(0.15, 0.65, 0.25, 1.0),
        food_color: Color::new(1.0, 0.35, 0.35, 1.0),
        wall_color: Color::new(0.5, 0.5, 0.55, 1.0),
    }
}