//! Font loading: DejaVu Sans Mono for accents and typographic dashes.
//!
//! macroquad's default font (ProggyClean.ttf) is a bitmap that only covers
//! basic ASCII up to ~32 px. French accents (é, è, ê) and typographic
//! dashes (—, –) render as tofu (□), and large sizes refuse to draw at all.
//! We load DejaVu Sans Mono, which covers Latin-1 and general punctuation,
//! and set it as the global default font.
//!
//! Call `load_default_font()` once in `main()` before any text rendering.

use std::path::PathBuf;

use macroquad::prelude::*;

/// Load DejaVu Sans Mono from `assets/DejaVuSansMono.ttf` and set it as
/// the global default font. Returns the font for later use, or `None`
/// if loading failed (in which case the default font is kept).
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