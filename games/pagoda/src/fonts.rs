//! Font loading.
//!
//! - **DejaVu Sans Mono** is the default UI font. It scales cleanly to
//!   the large title and covers Latin punctuation.
//! - **Noto Sans SC (subset)** supplies the handful of Chinese glyphs
//!   used by the signboard, seals and perfect-combo label. The subset
//!   also keeps ASCII so mixed strings such as `完美 x3` render in one
//!   pass.
//!
//! Both are built into the binary (`include_bytes!`), so native and web
//! load them the same way through `RenderingServer::create_font`; the
//! web build no longer fetches them.

use ember_core::Color;
use ember_servers::{FontId, RenderingServer, TextMetrics};
use glam::Vec2;

#[cfg(test)]
use crate::paths;

/// Font file bundled in `assets/`.
pub const FONT_FILE: &str = "DejaVuSansMono.ttf";

/// CJK subset used for decorative Chinese text.
pub const CJK_FONT_FILE: &str = "NotoSansSC-Pagoda.otf";

const FONT_BYTES: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
const CJK_FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSansSC-Pagoda.otf");

/// Load Pagoda's UI font and set it as the default.
///
/// Returns `None` when the font cannot be parsed; the game then keeps
/// the backend's built-in font.
pub fn load_default_font(r: &mut dyn RenderingServer) -> Option<FontId> {
    match r.create_font(FONT_BYTES) {
        Ok(font) => {
            r.set_default_font(Some(font));
            Some(font)
        }
        Err(e) => {
            eprintln!("⚠️  Failed to load {FONT_FILE}: {e}. Using the built-in font.");
            None
        }
    }
}

/// The Chinese accent font, loaded once and passed explicitly to the
/// renderers that need it.
pub struct CjkFont {
    font: Option<FontId>,
}

impl CjkFont {
    /// Load the bundled subset. A font that does not parse is non-fatal:
    /// Chinese accents become inert and the English UI keeps working.
    pub fn load(r: &mut dyn RenderingServer) -> Self {
        match r.create_font(CJK_FONT_BYTES) {
            Ok(font) => Self { font: Some(font) },
            Err(e) => {
                eprintln!(
                    "⚠️  Failed to load CJK font {CJK_FONT_FILE}: {e}. \
                     Chinese accents will be skipped. \
                     Run games/pagoda/tools/subset-font.sh to generate it."
                );
                Self { font: None }
            }
        }
    }

    /// Font-less instance, handy for tests and fallbacks.
    pub fn hermetic() -> Self {
        Self { font: None }
    }

    pub fn is_available(&self) -> bool {
        self.font.is_some()
    }

    /// Draw `text` at `(x, y)`; `y` is the baseline.
    pub fn draw(
        &self,
        r: &mut dyn RenderingServer,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let pos = Vec2::new(x, y);
        match self.font {
            Some(font) => r.draw_text_with_font(font, text, pos, size, color),
            None => r.draw_text(text, pos, size, color),
        }
    }

    pub fn measure(&self, r: &dyn RenderingServer, text: &str, size: f32) -> TextMetrics {
        match self.font {
            Some(font) => r.measure_text_with_font(font, text, size),
            None => r.measure_text(text, size),
        }
    }

    /// Draw horizontally centred around `center_x`.
    pub fn draw_centered(
        &self,
        r: &mut dyn RenderingServer,
        text: &str,
        center_x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let dim = self.measure(r, text, size);
        self.draw(r, text, center_x - dim.width * 0.5, y, size, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermetic_font_reports_unavailable() {
        let font = CjkFont::hermetic();
        assert!(!font.is_available());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn bundled_cjk_font_is_present() {
        assert!(paths::asset(CJK_FONT_FILE).is_file());
    }
}
