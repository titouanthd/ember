//! Font loading: DejaVu Sans Mono for the card symbols.
//!
//! macroquad's default font (ProggyClean.ttf) only covers basic ASCII.
//! The card suits (♠ ♥ ♦ ♣, U+2660..U+2667) are not in it, so they
//! render as tofu (□). We load DejaVu Sans Mono, which includes them,
//! and set it as the global default font.

use std::path::PathBuf;

use macroquad::prelude::*;

/// Load DejaVu Sans Mono from `assets/DejaVuSansMono.ttf` and set it as
/// the global default font. Returns the font for later use, or `None`
/// if loading failed (in which case the default font is kept).
///
/// Call this once in `main()` before any text rendering.
pub async fn load_default_font() -> Option<Font> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("DejaVuSansMono.ttf");

    let path_str = path.to_str()?;
    match load_ttf_font(path_str).await {
        Ok(font) => {
            set_default_font(font.clone());
            Some(font)
        }
        Err(e) => {
            eprintln!("⚠️  Failed to load {path:?}: {e}. Falling back to default font.");
            None
        }
    }
}

// ---------------------------------------------------------------------
// Text helpers — thin wrappers around macroquad's `draw_text` /
// `measure_text`, tuned for the menu UI.
//
// They take `size: f32` (macroquad uses f32 for `draw_text` but u16 for
// `measure_text`), so call sites stay consistent. DejaVu Sans Mono has
// no bold variant loaded, so "bold" is simulated by double-drawing with
// a sub-pixel offset — cheap and good enough for the menu chrome.
// ---------------------------------------------------------------------

/// Draw regular text at `size` px.
pub fn draw_text_regular(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(text, x, y, size, color);
}

/// Draw "bold" text — same glyphs as regular, drawn twice with a
/// 0.6px horizontal offset to fake weight.
pub fn draw_text_bold(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(text, x, y, size, color);
    draw_text(text, x + 0.6, y, size, color);
}

/// Measure regular text at `size` px. Uses the current default font.
pub fn measure_regular(text: &str, size: f32) -> TextDimensions {
    measure_text(text, None, size as u16, 1.0)
}

/// Measure "bold" text. Uses the same metrics as regular — the 0.6px
/// fake-bold offset is negligible for layout purposes.
pub fn measure_bold(text: &str, size: f32) -> TextDimensions {
    measure_text(text, None, size as u16, 1.0)
}