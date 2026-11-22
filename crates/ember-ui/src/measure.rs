//! `Measure`: what a control may consult when computing its desired size.

use crate::Theme;
use ember_servers::RenderingServer;
use glam::Vec2;

/// Passed to [`crate::Control::measure`] during layout. Text size depends
/// on the backend's font, so measuring goes through the renderer.
pub struct Measure<'a> {
    render: &'a dyn RenderingServer,
    theme: &'a Theme,
}

impl<'a> Measure<'a> {
    pub fn new(render: &'a dyn RenderingServer, theme: &'a Theme) -> Self {
        Self { render, theme }
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// Width and height of `text` drawn at `size` with the default font.
    pub fn text(&self, text: &str, size: f32) -> Vec2 {
        let m = self.render.measure_text(text, size);
        Vec2::new(m.width, m.height)
    }
}
