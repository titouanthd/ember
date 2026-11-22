//! `Theme`: colours and sizes shared by every control under a `UiRoot`.

use ember_core::Color;

/// Spec §7.1. `Theme::default()` is [`Theme::dark`]. Loading a theme from
/// RON is deferred (ADR-034): `Color` has no serde support yet.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub font_size: f32,
    pub font_size_large: f32,
    pub text: Color,
    pub text_dim: Color,
    pub panel_bg: Color,
    pub panel_border: Color,
    pub button_bg_normal: Color,
    pub button_bg_hover: Color,
    pub button_bg_pressed: Color,
    pub button_bg_disabled: Color,
    pub button_text_normal: Color,
    pub button_text_disabled: Color,
    pub focus_border: Color,
    pub scrollbar: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

impl Theme {
    /// Light text on dark slate panels; amber focus ring.
    pub fn dark() -> Self {
        Self {
            font_size: 20.0,
            font_size_large: 32.0,
            text: rgb(0xEC, 0xEF, 0xF4),
            text_dim: rgb(0x8F, 0x98, 0xA8),
            panel_bg: rgb(0x1E, 0x23, 0x2D),
            panel_border: rgb(0x3B, 0x42, 0x52),
            button_bg_normal: rgb(0x34, 0x3B, 0x4A),
            button_bg_hover: rgb(0x43, 0x4C, 0x5E),
            button_bg_pressed: rgb(0x2A, 0x30, 0x3C),
            button_bg_disabled: rgb(0x26, 0x2B, 0x34),
            button_text_normal: rgb(0xEC, 0xEF, 0xF4),
            button_text_disabled: rgb(0x6B, 0x72, 0x80),
            focus_border: rgb(0xF2, 0xB1, 0x34),
            scrollbar: rgb(0x5E, 0x67, 0x79),
        }
    }

    /// Dark text on light panels; blue focus ring.
    pub fn light() -> Self {
        Self {
            font_size: 20.0,
            font_size_large: 32.0,
            text: rgb(0x1F, 0x24, 0x2E),
            text_dim: rgb(0x5C, 0x64, 0x73),
            panel_bg: rgb(0xF4, 0xF5, 0xF7),
            panel_border: rgb(0xC9, 0xCE, 0xD6),
            button_bg_normal: rgb(0xE2, 0xE6, 0xEC),
            button_bg_hover: rgb(0xD3, 0xD9, 0xE2),
            button_bg_pressed: rgb(0xC2, 0xC9, 0xD4),
            button_bg_disabled: rgb(0xEC, 0xEE, 0xF1),
            button_text_normal: rgb(0x1F, 0x24, 0x2E),
            button_text_disabled: rgb(0xA0, 0xA7, 0xB3),
            focus_border: rgb(0x25, 0x63, 0xEB),
            scrollbar: rgb(0xA8, 0xB0, 0xBD),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG relative luminance, enough to check text stays readable.
    fn luminance(c: Color) -> f32 {
        let f = |v: f32| {
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn default_is_dark() {
        assert_eq!(Theme::default(), Theme::dark());
    }

    #[test]
    fn text_is_readable_on_panels_and_buttons() {
        for t in [Theme::dark(), Theme::light()] {
            assert!(contrast(t.text, t.panel_bg) >= 4.5);
            assert!(contrast(t.button_text_normal, t.button_bg_normal) >= 4.5);
            assert!(contrast(t.button_text_normal, t.button_bg_hover) >= 4.5);
            assert!(contrast(t.focus_border, t.panel_bg) >= 3.0);
        }
    }

    #[test]
    fn sizes_are_positive_and_ordered() {
        for t in [Theme::dark(), Theme::light()] {
            assert!(t.font_size > 0.0 && t.font_size_large > t.font_size);
        }
    }
}
