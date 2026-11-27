//! Reference `RenderingServer` implementation on macroquad.
//!
//! The only crate that imports `macroquad` for rendering. Everything
//! above it talks through `ember_servers::RenderingServer`.
//!
//! ## Usage
//!
//! ```ignore
//! use ember_render_macroquad::MacroquadRenderer;
//! use ember_servers::RenderingServer;
//!
//! let mut renderer = MacroquadRenderer::new();
//! renderer.begin_frame(Rect::new(0.0, 0.0, 800.0, 600.0));
//! // ... draw calls ...
//! renderer.end_frame();
//! macroquad::window::next_frame().await;
//! ```

mod convert;
mod input;
mod renderer;
mod run;

pub use ember_servers as servers;
pub use input::poll_input;
pub use renderer::MacroquadRenderer;
pub use run::{WindowConfig, WindowIcon, run, run_window, run_window_with};

// Re-export macroquad, the only path to it outside the two backend
// crates. No game imports it since Phase 7 (`make macroquad`).
pub use macroquad;
