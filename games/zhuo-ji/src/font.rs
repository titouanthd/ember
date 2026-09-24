//! CJK font loading with hermetic fallback.
//!
//! The bundled subset contains only the glyphs used by Zhuo Ji
//! (numerals, suit chars, winds, dragons, title). If the font fails
//! to load (tests, headless CI), every method falls back to the
//! default macroquad font so nothing panics.

use macroquad::prelude::*;

pub struct TileFont {
    font: Option<Font>,
}

impl TileFont {
    /// Load the bundled subset font.
    pub fn load() -> Self {
        let bytes = include_bytes!("../assets/NotoSansSC-ZhuoJi.ttf");
        let font = load_ttf_font_from_bytes(bytes).ok();
        Self { font }
    }

    /// No-op font for tests and headless contexts.
    pub fn hermetic() -> Self {
        Self { font: None }
    }

    pub fn is_available(&self) -> bool {
        self.font.is_some()
    }

    /// Draw text at (x, y) — y is the baseline.
    pub fn draw(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        match &self.font {
            Some(f) => {
                draw_text_ex(
                    text,
                    x,
                    y,
                    TextParams {
                        font: Some(f),
                        font_size: size as u16,
                        color,
                        ..Default::default()
                    },
                );
            }
            None => {
                draw_text(text, x, y, size, color);
            }
        }
    }

    pub fn measure(&self, text: &str, size: f32) -> TextDimensions {
        match &self.font {
            Some(f) => measure_text(text, Some(f), size as u16, 1.0),
            None => measure_text(text, None, size as u16, 1.0),
        }
    }

    pub fn draw_centered(&self, text: &str, cx: f32, y: f32, size: f32, color: Color) {
        let dim = self.measure(text, size);
        self.draw(text, cx - dim.width * 0.5, y, size, color);
    }

    pub fn draw_right_aligned(&self, text: &str, right_x: f32, y: f32, size: f32, color: Color) {
        let dim = self.measure(text, size);
        self.draw(text, right_x - dim.width, y, size, color);
    }
}