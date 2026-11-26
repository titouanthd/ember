// games/pagoda/src/config.rs
use ember_core::Color;

/// Pure white (macroquad's `WHITE` has the same value).
pub const WHITE: Color = Color::WHITE;

/// Shared gold used by the perfect-drop effects, seals and HUD.
pub const PERFECT_GOLD: Color = Color::new(1.0, 0.82, 0.25, 1.0);

/// Warm paper white for body text and highlights.
pub const IVORY: Color = Color::new(0.95, 0.91, 0.80, 1.0);

/// Deep lacquer red, used for signboards and seal shadows.
pub const LACQUER_RED: Color = Color::new(0.46, 0.08, 0.06, 1.0);

/// Bright cinnabar, used for seals and the game-over title.
pub const CINNABAR: Color = Color::new(0.86, 0.24, 0.17, 1.0);

/// Pale gold moon, warm rather than blue-white.
pub const MOON: Color = Color::new(1.0, 0.95, 0.78, 1.0);

/// Color stops cycled by tower height: celadon, turquoise, indigo,
/// cinnabar and gold instead of the old synthwave rainbow.
const BLOCK_STOPS: [Color; 5] = [
    Color::new(0.36, 0.80, 0.66, 1.0), // 青瓷 celadon
    Color::new(0.24, 0.70, 0.78, 1.0), // 碧 turquoise
    Color::new(0.34, 0.45, 0.82, 1.0), // 靛 indigo
    CINNABAR,                          // 朱砂 cinnabar
    PERFECT_GOLD,                      // 金 gold
];

/// How far the palette advances per block. A full cycle takes ~33 blocks.
const BLOCK_COLOR_STEP: f32 = 0.03;

#[derive(Debug, Clone)]
pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub block_height: f32,
    pub base_block_width: f32,
    pub base_speed: f32,
    pub speed_increment: f32,
    pub max_speed: f32,
    pub perfect_threshold: f32,
    pub perfect_growth: f32,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 800.0,
            screen_h: 600.0,
            block_height: 40.0,
            base_block_width: 200.0,
            base_speed: 500.0,
            speed_increment: 20.0,
            max_speed: 800.0,
            perfect_threshold: 4.0,
            perfect_growth: 2.0,
        }
    }
}

/// Linearly interpolate between two colors.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Sample a looping color gradient. `t` wraps around `1.0`.
fn sample_cycle(stops: &[Color], t: f32) -> Color {
    if stops.is_empty() {
        return WHITE;
    }
    if stops.len() == 1 {
        return stops[0];
    }

    let scaled = t.rem_euclid(1.0) * stops.len() as f32;
    let index = (scaled.floor() as usize) % stops.len();
    let next = (index + 1) % stops.len();
    lerp_color(stops[index], stops[next], scaled.fract())
}

/// Color of the block at `height` (base block = 0).
pub fn block_color(height: u32) -> Color {
    sample_cycle(&BLOCK_STOPS, height as f32 * BLOCK_COLOR_STEP)
}

/// Dark background tinted by the current tower height.
pub fn background_color(height: u32) -> Color {
    let c = block_color(height);
    Color::new(
        0.040 + c.r * 0.070,
        0.032 + c.g * 0.060,
        0.080 + c.b * 0.085,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_has_sane_tuning() {
        let ctx = GameContext::default();
        assert!(ctx.max_speed > ctx.base_speed);
        assert!(ctx.perfect_growth > 0.0);
    }

    #[test]
    fn block_palette_starts_at_first_stop() {
        assert_eq!(block_color(0), BLOCK_STOPS[0]);
    }

    #[test]
    fn block_palette_changes_with_height() {
        assert_ne!(block_color(0), block_color(11));
    }

    #[test]
    fn palette_cycle_wraps() {
        assert_eq!(
            sample_cycle(&BLOCK_STOPS, 0.0),
            sample_cycle(&BLOCK_STOPS, 1.0)
        );
    }

    #[test]
    fn lerp_color_interpolates_channels() {
        let a = Color::new(0.0, 0.0, 0.0, 1.0);
        let b = Color::new(1.0, 0.5, 0.25, 1.0);
        let mid = lerp_color(a, b, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6);
        assert!((mid.g - 0.25).abs() < 1e-6);
        assert!((mid.b - 0.125).abs() < 1e-6);
        assert_eq!(mid.a, 1.0);
    }

    #[test]
    fn background_is_dark_and_shifts_with_height() {
        let low = background_color(0);
        let high = background_color(20);

        for c in [low, high] {
            assert!(c.r < 0.3);
            assert!(c.g < 0.3);
            assert!(c.b < 0.4);
            assert_eq!(c.a, 1.0);
        }

        assert_ne!(low, high);
    }
}
