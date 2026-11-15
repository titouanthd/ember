//! Font loading.
//!
//! - **DejaVu Sans Mono** is the default UI font. It scales cleanly to
//!   the large title and covers Latin punctuation.
//! - **Noto Sans SC (subset)** supplies the handful of Chinese glyphs
//!   used by the signboard, seals and perfect-combo label. The subset
//!   also keeps ASCII so mixed strings such as `完美 x3` render in one
//!   pass.
//!
//! Loading is async because the web build fetches the files over HTTP;
//! native builds resolve them from disk under the same API.

use macroquad::prelude::*;

use crate::paths;

/// Font file bundled in `assets/`.
pub const FONT_FILE: &str = "DejaVuSansMono.ttf";

/// CJK subset used for decorative Chinese text.
pub const CJK_FONT_FILE: &str = "NotoSansSC-Pagoda.otf";

/// Load Pagoda's UI font and set it as the default.
///
/// Returns `None` when the file is missing or malformed; the game then
/// keeps macroquad's built-in font.
pub async fn load_default_font() -> Option<Font> {
    let path = paths::asset_str(FONT_FILE);
    ember_stdlib::fonts::load_and_set_default(&path).await
}

/// The Chinese accent font, loaded once and passed explicitly to the
/// renderers that need it.
pub struct CjkFont {
    font: Option<Font>,
}

impl CjkFont {
    /// Load the bundled subset. A missing file is non-fatal: Chinese
    /// accents become inert and the English UI keeps working.
    pub async fn load() -> Self {
        let path = paths::asset_str(CJK_FONT_FILE);
        match load_ttf_font(&path).await {
            Ok(font) => Self { font: Some(font) },
            Err(e) => {
                eprintln!(
                    "⚠️  Failed to load CJK font {path}: {e}. \
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
    pub fn draw(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        match &self.font {
            Some(font) => {
                draw_text_ex(
                    text,
                    x,
                    y,
                    TextParams {
                        font: Some(font),
                        font_size: size as u16,
                        color,
                        ..Default::default()
                    },
                );
            }
            None => {
                draw_text(text, x, y, size, color);
            }
        }
    }

    pub fn measure(&self, text: &str, size: f32) -> TextDimensions {
        match &self.font {
            Some(font) => measure_text(text, Some(font), size as u16, 1.0),
            None => measure_text(text, None, size as u16, 1.0),
        }
    }

    /// Draw horizontally centred around `center_x`.
    pub fn draw_centered(&self, text: &str, center_x: f32, y: f32, size: f32, color: Color) {
        let dim = self.measure(text, size);
        self.draw(text, center_x - dim.width * 0.5, y, size, color);
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
