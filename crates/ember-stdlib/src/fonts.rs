//! Font loading and text-drawing helpers.
//!
//! Extracted after 3 games (FreeCell, ember-wars, jade-garden)
//! shared the same DejaVu Sans Mono pattern (Rule of Three).
//!
//! The **loading function is async** because on the web target
//! `macroquad::text::load_ttf_font` fetches the file via HTTP. On
//! native, the same API resolves synchronously under the hood.
//!
//! The `path` parameter must already be resolved by the caller:
//! - Native: absolute or relative filesystem path.
//! - Web: URL relative to the page.
//!
//! No `paths` helper is provided here because each game has its own
//! asset directory (and on web the correct base is the page URL, not
//! the manifest dir). Use `ember_stdlib::paths` or a game-local
//! `paths` module to resolve.

use macroquad::prelude::*;

/// Load a TTF font and set it as the global default font.
///
/// Returns the loaded `Font` on success, `None` on failure (in which
/// case macroquad's built-in ProggyClean is kept — ASCII-only, tofu
/// for ★☆ and accents).
///
/// Call this once in `main()` before any text rendering.
pub async fn load_and_set_default(path: &str) -> Option<Font> {
    match load_ttf_font(path).await {
        Ok(font) => {
            set_default_font(font.clone());
            Some(font)
        }
        Err(e) => {
            eprintln!("⚠️  Failed to load {path}: {e}. Falling back to default font.");
            None
        }
    }
}

// ---------- Text helpers ----------

/// Draw regular text at `size` px.
pub fn draw_text_regular(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(text, x, y, size, color);
}

/// Draw "bold" text — same glyphs as regular, drawn twice with a
/// 0.6px horizontal offset to fake weight. DejaVu Sans Mono has no
/// bold variant loaded, so this is the cheap approximation.
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