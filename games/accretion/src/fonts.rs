//! Font loading.
//!
//! DejaVu Sans Mono becomes the default font. It covers `→` (U+2192) and
//! the other UI glyphs, unlike the backend's built-in font. The file is
//! built into the binary and installed through the `RenderingServer` in
//! the root node's `ready`, so native and web load it the same way (was
//! `load_ttf_font` from `assets/fonts/`). If it fails to parse, the game
//! still runs with the built-in font.

use ember::prelude::*;

const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");

/// Installs DejaVu Sans Mono as the default font.
///
/// Returns `true` on success, `false` if the font does not parse (the
/// game continues with the built-in font).
pub fn load_default_font(gfx: &mut dyn RenderingServer) -> bool {
    match gfx.create_font(FONT_BYTES) {
        Ok(font) => {
            gfx.set_default_font(Some(font));
            true
        }
        Err(e) => {
            eprintln!(
                "⚠️  failed to load DejaVuSansMono.ttf: {e}. \
                 Using the built-in font (→ will show as tofu)."
            );
            false
        }
    }
}
