//! Facade crate: prelude re-exports, MainLoop, run(). Games depend on this, not on the backends.

pub use ember_core::MainLoop;
pub use ember_render_macroquad::{WindowConfig, WindowIcon, run, run_window, run_window_with};
pub use ember_ui;

pub mod prelude {
    pub use ember_audio_macroquad::MacroquadAudio;
    pub use ember_core::math::Rect;
    pub use ember_core::{Color, MainLoop, Object};
    pub use ember_input::prelude::*;
    pub use ember_macros::Node;
    pub use ember_render_macroquad::{
        MacroquadRenderer, WindowConfig, WindowIcon, run, run_window, run_window_with,
    };
    pub use ember_scene::prelude::*;
    pub use ember_servers::{AudioServer, Camera2d, RenderingServer};
    pub use ember_ui::prelude::*;
    pub use glam::Vec2;
}
