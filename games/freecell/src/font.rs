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
