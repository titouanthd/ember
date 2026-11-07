//! src/fonts.rs
//! Font loading.
//!
//! Tries to load `assets/DejaVuSansMono.ttf` and set it as the default
//! font. If the file is missing, the game still runs — macroquad's
//! built-in font is used, with a warning in the console.

use macroquad::prelude::*;

use crate::paths;

/// Loads DejaVu Sans Mono and sets it as the default font.
///
/// Returns `true` on success, `false` if the file is missing or
/// malformed (the game continues with macroquad's default font).
pub async fn load_default_font() -> bool {
    let path = paths::asset_str("DejaVuSansMono.ttf");
    match load_ttf_font(&path).await {
        Ok(font) => {
            set_default_font(font);
            true
        }
        Err(e) => {
            eprintln!(
                "⚠️  failed to load {path}: {e}. \
                 Using macroquad's default font. \
                 Copy DejaVuSansMono.ttf into games/bore-inc/assets/."
            );
            false
        }
    }
}