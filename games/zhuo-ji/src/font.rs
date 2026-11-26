//! CJK font loading with hermetic fallback.
//!
//! The bundled subset contains only the glyphs used by Zhuo Ji
//! (numerals, suit chars, winds, dragons, title). It is installed through
//! `RenderingServer::create_font` in the root node's `ready`. If it fails
//! to load (tests, headless CI), every method falls back to the default
//! font so nothing panics.

use ember::prelude::*;
use ember_servers::{FontId, TextMetrics};

const FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSansSC-ZhuoJi.ttf");

pub struct TileFont {
    font: Option<FontId>,
}

impl TileFont {
    /// Load the bundled subset font.
    pub fn load(gfx: &mut dyn RenderingServer) -> Self {
        Self {
            font: gfx.create_font(FONT_BYTES).ok(),
        }
    }

    /// No-op font for tests and headless contexts.
    pub fn hermetic() -> Self {
        Self { font: None }
    }

    pub fn is_available(&self) -> bool {
        self.font.is_some()
    }

    /// Draw text at (x, y) — y is the baseline.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        gfx: &mut dyn RenderingServer,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let pos = Vec2::new(x, y);
        match self.font {
            Some(f) => gfx.draw_text_with_font(f, text, pos, size, color),
            None => gfx.draw_text(text, pos, size, color),
        }
    }

    pub fn measure(&self, gfx: &dyn RenderingServer, text: &str, size: f32) -> TextMetrics {
        match self.font {
            Some(f) => gfx.measure_text_with_font(f, text, size),
            None => gfx.measure_text(text, size),
        }
    }

    pub fn draw_centered(
        &self,
        gfx: &mut dyn RenderingServer,
        text: &str,
        cx: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let dim = self.measure(gfx, text, size);
        self.draw(gfx, text, cx - dim.width * 0.5, y, size, color);
    }

    pub fn draw_right_aligned(
        &self,
        gfx: &mut dyn RenderingServer,
        text: &str,
        right_x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let dim = self.measure(gfx, text, size);
        self.draw(gfx, text, right_x - dim.width, y, size, color);
    }
}
