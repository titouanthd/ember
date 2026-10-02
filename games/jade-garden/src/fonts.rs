//! Font loading.
//!
//! - **DejaVu Sans Mono**: the default font. Contains French accents,
//!   the stars ★☆, and Unicode punctuation.
//! - **Noto Sans SC (subset)**: only the CJK glyphs of the poem, its
//!   title, the author, and the jade names.
//!
//! Loading is async on every target because on the web the font files
//! are fetched via HTTP. On native the same API resolves synchronously
//! under the hood.

use macroquad::prelude::*;

use crate::paths;

/// Loads DejaVu Sans Mono and sets it as the default font.
pub async fn load_default_font() -> Option<Font> {
    let path = paths::asset_str("DejaVuSansMono.ttf");
    match load_ttf_font(&path).await {
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

/// The poem's CJK font. Loaded once at startup and passed explicitly
/// to rendering functions that need it.
pub struct PoemFont {
    font: Option<Font>,
}

impl PoemFont {
    /// Loads the CJK subset from `assets/NotoSansSC-JadeGarden.otf`.
    /// If the file is missing, returns a fallback instance.
    pub async fn load() -> Self {
        let path = paths::asset_str("NotoSansSC-JadeGarden.otf");
        match load_ttf_font(&path).await {
            Ok(font) => Self { font: Some(font) },
            Err(e) => {
                eprintln!(
                    "⚠️  Failed to load CJK font {path}: {e}. \
                     CJK chars will fall back to the default font. \
                     Run games/jade-garden/subset_font.sh to generate it."
                );
                Self { font: None }
            }
        }
    }

    /// Hermetic instance (no font) — handy for tests.
    pub fn hermetic() -> Self {
        Self { font: None }
    }

    pub fn is_available(&self) -> bool {
        self.font.is_some()
    }

    /// Draws `text` at `(x, y)` — `y` is the baseline.
    pub fn draw(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        match &self.font {
            Some(f) => {
                draw_text_ex(
                    text,
                    x,
                    y,
                    TextParams {
                        font: Some(f),
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

    /// Measures `text`.
    pub fn measure(&self, text: &str, size: f32) -> TextDimensions {
        match &self.font {
            Some(f) => measure_text(text, Some(f), size as u16, 1.0),
            None => measure_text(text, None, size as u16, 1.0),
        }
    }

    pub fn draw_centered(&self, text: &str, cx: f32, y: f32, size: f32, color: Color) {
        let dim = self.measure(text, size);
        self.draw(text, cx - dim.width * 0.5, y, size, color);
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