//! Facade crate: prelude re-exports, MainLoop, run(). Games depend on this, not on the backends.

pub use ember_core::MainLoop;
pub use ember_render_macroquad::{run, run_window};

pub mod prelude {
    pub use ember_audio_macroquad::MacroquadAudio;
    pub use ember_core::math::Rect;
    pub use ember_core::{Color, MainLoop, Object};
    pub use ember_input::prelude::*;
    pub use ember_macros::Node;
    pub use ember_render_macroquad::{MacroquadRenderer, run, run_window};
    pub use ember_scene::prelude::*;
    pub use ember_servers::{AudioServer, RenderingServer};
    pub use glam::Vec2;
}
