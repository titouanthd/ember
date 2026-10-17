//! Global context: colors, layout, tuning constants.
//!
//! Loaded once from `.env` via `ember_stdlib::config`. Colors have
//! sane defaults when the `.env` file is missing.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use ember_stdlib::config::{env_color, load_dotenv_once};

use macroquad::prelude::*;

/// Earthy palette per DESIGN §1: ochres, browns, stone gray, cyan
/// accents for crystal, amber for lanterns.
#[derive(Debug, Clone)]
pub struct Colors {
    pub bg_top: Color,
    pub bg_bottom: Color,
    pub panel_bg: Color,
    pub panel_border: Color,
    pub text: Color,
    pub text_dim: Color,
    pub gold: Color,
    pub crystal: Color,
    pub danger: Color,
    pub grass: Color,
    pub sky_top: Color,
    pub sky_bottom: Color,
}

impl Colors {
    pub fn defaults() -> Self {
        Self {
            // Deep mine-brown background.
            bg_top:       Color::new(0.102, 0.071, 0.031, 1.0), // #1A1208
            bg_bottom:    Color::new(0.051, 0.035, 0.016, 1.0), // #0D0904

            // Warm dark translucent panel.
            panel_bg:     Color::new(0.165, 0.122, 0.063, 0.90),

            // Aged ochre border.
            panel_border: Color::new(0.722, 0.573, 0.290, 1.0), // #B8924A

            // Warm ivory text.
            text:         Color::new(0.910, 0.851, 0.710, 1.0), // #E8D9B5
            text_dim:     Color::new(0.600, 0.502, 0.353, 1.0), // #99805A

            // Amber (lanterns, currency).
            gold:         Color::new(0.831, 0.647, 0.306, 1.0), // #D4A54E

            // Crystal cyan.
            crystal:      Color::new(0.373, 0.831, 0.831, 1.0), // #5FD4D4

            // Rust red for danger.
            danger:       Color::new(0.722, 0.290, 0.196, 1.0), // #B84A32

            // Muted forest green — grass tufts on exposed dirt.
            grass:        Color::new(0.353, 0.580, 0.247, 1.0), // #5A943F

            // Cozy daylight sky: soft blue at zenith, warm cream at
            // horizon.
            sky_top:      Color::new(0.478, 0.588, 0.741, 1.0), // #7A96BD
            sky_bottom:   Color::new(0.902, 0.827, 0.639, 1.0), // #E6D3A3
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_env() -> Self {
        let d = Self::defaults();
        Self {
            bg_top:       env_color("COLOR_BG_TOP", d.bg_top),
            bg_bottom:    env_color("COLOR_BG_BOTTOM", d.bg_bottom),
            panel_bg:     env_color("COLOR_PANEL_BG", d.panel_bg),
            panel_border: env_color("COLOR_PANEL_BORDER", d.panel_border),
            text:         env_color("COLOR_TEXT", d.text),
            text_dim:     env_color("COLOR_TEXT_DIM", d.text_dim),
            gold:         env_color("COLOR_GOLD", d.gold),
            crystal:      env_color("COLOR_CRYSTAL", d.crystal),
            danger:       env_color("COLOR_DANGER", d.danger),
            grass:        env_color("COLOR_GRASS", d.grass),
            sky_top:      env_color("COLOR_SKY_TOP", d.sky_top),
            sky_bottom:   env_color("COLOR_SKY_BOTTOM", d.sky_bottom),
        }
    }
}

/// Global context, resolved once at startup.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub colors: Colors,
    pub viewport_w: f32,
    pub viewport_h: f32,
}

impl GameContext {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_env() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_once(manifest_dir, "bore-inc");

        Self {
            colors: Colors::from_env(),
            viewport_w: screen_width(),
            viewport_h: screen_height(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_env() -> Self {
        Self {
            colors: Colors::defaults(),
            viewport_w: screen_width(),
            viewport_h: screen_height(),
        }
    }

    pub fn defaults() -> Self {
        Self {
            colors: Colors::defaults(),
            viewport_w: 1280.0,
            viewport_h: 720.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_default_colors_are_in_unit_range() {
        let c = Colors::defaults();
        for col in [
            c.bg_top, c.bg_bottom, c.panel_bg, c.panel_border,
            c.text, c.text_dim, c.gold, c.crystal, c.danger,
            c.grass, c.sky_top, c.sky_bottom,
        ] {
            for v in [col.r, col.g, col.b, col.a] {
                assert!((0.0..=1.0).contains(&v), "{col:?} has {v}");
            }
        }
    }

    #[test]
    fn defaults_context_has_expected_viewport() {
        let ctx = GameContext::defaults();
        assert_eq!(ctx.viewport_w, 1280.0);
        assert_eq!(ctx.viewport_h, 720.0);
    }
}