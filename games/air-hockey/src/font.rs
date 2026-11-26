//! Font loading: DejaVu Sans Mono for accents and typographic dashes.
//!
//! The backend's built-in font is a bitmap that only covers basic ASCII.
//! French accents (é, è, ê) and typographic dashes (—, –) would render as
//! tofu (□). We load DejaVu Sans Mono, which covers Latin-1 and general
//! punctuation, and make it the renderer's default font.
//!
//! Called once from the root node's `ready`.

use std::path::PathBuf;

use ember::prelude::RenderingServer;

/// `assets/DejaVuSansMono.ttf` next to Air Hockey's manifest.
pub fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("DejaVuSansMono.ttf")
}

/// Load DejaVu Sans Mono and make it the default font. Returns `false`
/// (keeping the built-in font) if loading failed.
pub fn load_default_font(r: &mut dyn RenderingServer) -> bool {
    let path = font_path();
    let loaded = std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| r.create_font(&bytes).map_err(|e| e.to_string()));
    match loaded {
        Ok(id) => {
            r.set_default_font(Some(id));
            true
        }
        Err(e) => {
            eprintln!(
                "⚠️  Failed to load {}: {e}. Falling back to default font.",
                path.display()
            );
            false
        }
    }
}
