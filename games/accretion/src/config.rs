//! Palette and layout constants. Color evolves with the paradigm.

use macroquad::prelude::*;

use crate::components::Paradigm;

pub const BG: Color = Color::new(0.02, 0.02, 0.05, 1.0);
pub const TEXT: Color = Color::new(0.92, 0.92, 0.88, 1.0);
pub const DIM: Color = Color::new(0.45, 0.45, 0.50, 1.0);
pub const PANEL: Color = Color::new(0.07, 0.07, 0.11, 1.0);
pub const PANEL_BORDER: Color = Color::new(0.20, 0.20, 0.28, 1.0);
pub const BUTTON: Color = Color::new(0.14, 0.14, 0.20, 1.0);
pub const BUTTON_HOVER: Color = Color::new(0.24, 0.24, 0.34, 1.0);
pub const BUTTON_DISABLED: Color = Color::new(0.10, 0.10, 0.12, 1.0);

/// Width of the right panel (used by both main.rs and disk.rs).
pub const RIGHT_PANEL_W: f32 = 320.0;

/// Color used to accent the UI at a given paradigm. Also used for the
/// central star's tint in later sessions.
pub fn paradigm_color(p: Paradigm) -> Color {
    match p {
        Paradigm::Dust => Color::new(0.60, 0.45, 0.30, 1.0),
        Paradigm::Pebbles => Color::new(0.75, 0.55, 0.30, 1.0),
        Paradigm::Planetesimals => Color::new(0.90, 0.55, 0.20, 1.0),
        Paradigm::Planets => Color::new(0.95, 0.80, 0.25, 1.0),
        Paradigm::GasGiant => Color::new(0.40, 0.75, 0.85, 1.0),
        Paradigm::Star => Color::new(0.85, 0.95, 1.00, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paradigm_color_is_opaque() {
        for p in Paradigm::ALL {
            assert_eq!(paradigm_color(p).a, 1.0);
        }
    }

    #[test]
    fn paradigm_colors_are_distinct() {
        let all: Vec<_> = Paradigm::ALL.iter().map(|p| paradigm_color(*p)).collect();
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "duplicate color at {:?} / {:?}", i, j);
            }
        }
    }
}