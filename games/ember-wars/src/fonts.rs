//! Fonts: DejaVu Sans Mono (full Unicode: ★☆, accents, extended
//! symbols) as the default font, plus the regular / bold text helpers.
//!
//! The font is built into the binary and installed through the
//! `RenderingServer` in the root node's `ready` (was
//! `ember_stdlib::fonts::load_and_set_default` from `assets/`). The
//! helpers are local twins of `ember_stdlib::fonts`' macroquad ones.

use ember::prelude::*;
use ember_servers::TextMetrics;

const FONT_BYTES: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");

/// Install DejaVu Sans Mono as the default font. Returns `false` if it
/// does not parse (the built-in font stays).
pub fn load_default_font(gfx: &mut dyn RenderingServer) -> bool {
    match gfx.create_font(FONT_BYTES) {
        Ok(font) => {
            gfx.set_default_font(Some(font));
            true
        }
        Err(e) => {
            eprintln!("⚠️  Failed to load DejaVuSansMono.ttf: {e}. Falling back to default font.");
            false
        }
    }
}

/// Default-font text; `y` is the baseline.
pub fn draw_text_regular(
    gfx: &mut dyn RenderingServer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    gfx.draw_text(text, Vec2::new(x, y), size, color);
}

/// Fake bold: the text drawn twice, 0.6 px apart.
pub fn draw_text_bold(
    gfx: &mut dyn RenderingServer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    gfx.draw_text(text, Vec2::new(x, y), size, color);
    gfx.draw_text(text, Vec2::new(x + 0.6, y), size, color);
}

pub fn measure_regular(gfx: &dyn RenderingServer, text: &str, size: f32) -> TextMetrics {
    gfx.measure_text(text, size)
}

pub fn measure_bold(gfx: &dyn RenderingServer, text: &str, size: f32) -> TextMetrics {
    gfx.measure_text(text, size)
}
