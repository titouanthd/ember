//! Font loading: DejaVu Sans Mono for the card symbols.
//!
//! Delegates to `ember_stdlib::fonts`. The wrapper resolves the
//! game-local path from `CARGO_MANIFEST_DIR / assets`.

use std::path::PathBuf;

use macroquad::prelude::*;

/// Load DejaVu Sans Mono from `assets/DejaVuSansMono.ttf` and set it
/// as the global default font. Returns the font for later use, or
/// `None` if loading failed (in which case the default font is kept).
///
/// Call this once in `main()` before any text rendering.
pub async fn load_default_font() -> Option<Font> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("DejaVuSansMono.ttf");
    let path_str = path.to_str()?;
    ember_stdlib::fonts::load_and_set_default(path_str).await
}

// Re-export the text helpers so existing call sites (menu.rs,
// upgrade_modal.rs) keep working without changing their imports.
pub use ember_stdlib::fonts::{
    draw_text_bold, draw_text_regular, measure_bold, measure_regular,
};