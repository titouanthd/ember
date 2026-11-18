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
pub use run::{run, run_window};

// Re-export macroquad so games don't need their own dependency (until
// Phase 13 removes the last `use macroquad::` from game code — then
// this re-export becomes the only path).
pub use macroquad;
