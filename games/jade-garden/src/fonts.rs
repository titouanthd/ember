//! Fonts: DejaVu Sans Mono as the default font, and a subset of Noto
//! Sans SC for the poem's Chinese characters.
//!
//! Both are built into the binary and installed through the
//! `RenderingServer` in the root node's `ready`, so native and web load
//! them the same way (was `load_ttf_font` from `assets/`).

use ember::prelude::*;
use ember_servers::{FontId, TextMetrics};

const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
const POEM_FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSansSC-JadeGarden.otf");

/// Install DejaVu Sans Mono as the default font. A font that does not
/// parse is non-fatal: the backend's built-in font stays.
pub fn load_default_font(gfx: &mut dyn RenderingServer) -> Option<FontId> {
    match gfx.create_font(DEFAULT_FONT_BYTES) {
        Ok(font) => {
            gfx.set_default_font(Some(font));
            Some(font)
        }
        Err(e) => {
            eprintln!("⚠️  Failed to load DejaVuSansMono.ttf: {e}");
            None
        }
    }
}

pub struct PoemFont {
    font: Option<FontId>,
}

impl PoemFont {
    pub fn load(gfx: &mut dyn RenderingServer) -> Self {
        match gfx.create_font(POEM_FONT_BYTES) {
            Ok(font) => Self { font: Some(font) },
            Err(e) => {
                eprintln!(
                    "⚠️  Failed to load CJK font NotoSansSC-JadeGarden.otf: {e}. \
                     CJK chars will fall back to the default font. \
                     Run games/jade-garden/subset_font.sh to generate it."
                );
                Self { font: None }
            }
        }
    }

    pub fn hermetic() -> Self {
        Self { font: None }
    }

    pub fn is_available(&self) -> bool {
        self.font.is_some()
    }

    /// Draw `text` at `(x, y)`; `y` is the baseline.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        gfx: &mut dyn RenderingServer,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let pos = Vec2::new(x, y);
        match self.font {
            Some(f) => gfx.draw_text_with_font(f, text, pos, size, color),
            None => gfx.draw_text(text, pos, size, color),
        }
    }

    pub fn measure(&self, gfx: &dyn RenderingServer, text: &str, size: f32) -> TextMetrics {
        match self.font {
            Some(f) => gfx.measure_text_with_font(f, text, size),
            None => gfx.measure_text(text, size),
        }
    }

    pub fn draw_centered(
        &self,
        gfx: &mut dyn RenderingServer,
        text: &str,
        cx: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let dim = self.measure(gfx, text, size);
        self.draw(gfx, text, cx - dim.width * 0.5, y, size, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermetic_font_reports_unavailable() {
        let f = PoemFont::hermetic();
        assert!(!f.is_available());
    }
}
