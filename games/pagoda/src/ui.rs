// games/pagoda/src/ui.rs
//! Interface chrome with a Chinese accent: lacquer signboards, red
//! seals, gold dividers and ink panels.

use crate::art::{draw_diamond, with_alpha};
use crate::config::{CINNABAR, IVORY, LACQUER_RED, PERFECT_GOLD};
use crate::fonts::CjkFont;
use ember_core::Color;
use ember_core::math::Rect;
use ember_servers::RenderingServer;
use glam::{Vec2, vec2};

/// Ink panel with a double gold border and a soft drop shadow.
pub fn draw_panel(
    r: &mut dyn RenderingServer,
    cx: f32,
    y: f32,
    w: f32,
    h: f32,
    fill: Color,
    border: Color,
) {
    let x = cx - w / 2.0;

    r.draw_rect(
        Rect::new(x + 4.0, y + 5.0, w, h),
        Color::new(0.0, 0.0, 0.0, 0.35),
    );
    r.draw_rect(Rect::new(x, y, w, h), fill);
    r.draw_rect_outline(Rect::new(x, y, w, h), 2.0, border);
    r.draw_rect_outline(
        Rect::new(x + 6.0, y + 6.0, w - 12.0, h - 12.0),
        1.0,
        with_alpha(border, 0.42),
    );
}

/// Horizontal signboard (匾额): lacquer red, gold double border and
/// small corner diamonds.
pub fn draw_signboard(r: &mut dyn RenderingServer, cx: f32, y: f32, w: f32, h: f32) {
    draw_panel(r, cx, y, w, h, with_alpha(LACQUER_RED, 0.94), PERFECT_GOLD);

    let half_w = w / 2.0;
    let half_h = h / 2.0;
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        draw_diamond(
            r,
            cx + dx * (half_w - 16.0),
            y + half_h + dy * (half_h - 16.0),
            3.0,
            PERFECT_GOLD,
        );
    }
}

/// Low-contrast ink plaque used behind the best score.
pub fn draw_plaque(r: &mut dyn RenderingServer, cx: f32, y: f32, w: f32, h: f32) {
    draw_panel(
        r,
        cx,
        y,
        w,
        h,
        Color::new(0.035, 0.035, 0.055, 0.58),
        with_alpha(PERFECT_GOLD, 0.50),
    );
}

/// Gold rule with a diamond in the middle.
pub fn draw_divider(r: &mut dyn RenderingServer, cx: f32, y: f32, half_width: f32, color: Color) {
    let gap = 10.0;
    r.draw_line(
        Vec2::new(cx - half_width, y),
        Vec2::new(cx - gap, y),
        1.5,
        color,
    );
    r.draw_line(
        Vec2::new(cx + gap, y),
        Vec2::new(cx + half_width, y),
        1.5,
        color,
    );
    draw_diamond(r, cx, y, 4.0, color);
}

/// Square red seal with a single CJK character, or a gold diamond when
/// the CJK font is unavailable.
pub fn draw_seal(
    r: &mut dyn RenderingServer,
    cx: f32,
    cy: f32,
    size: f32,
    glyph: &str,
    cjk: &CjkFont,
) {
    let half = size / 2.0;
    let x = cx - half;
    let y = cy - half;

    r.draw_rect(
        Rect::new(x + 2.0, y + 2.0, size, size),
        Color::new(0.0, 0.0, 0.0, 0.30),
    );
    r.draw_rect(Rect::new(x, y, size, size), with_alpha(CINNABAR, 0.96));
    r.draw_rect_outline(
        Rect::new(x, y, size, size),
        2.0,
        with_alpha(PERFECT_GOLD, 0.92),
    );
    r.draw_rect_outline(
        Rect::new(x + 3.0, y + 3.0, size - 6.0, size - 6.0),
        1.0,
        with_alpha(IVORY, 0.38),
    );

    if cjk.is_available() {
        cjk.draw_centered(r, glyph, cx, cy + size * 0.24, size * 0.60, IVORY);
    } else {
        draw_diamond(r, cx, cy, size * 0.20, IVORY);
    }
}

/// Top-left sound toggle.
///
/// Click or tap only: there is no keyboard mute. The plaque is 44 px in
/// virtual space, and the invisible hit area is much larger so the
/// control stays comfortable on a phone. While muted the icon keeps a
/// cinnabar slash and a persistent `SOUND OFF` label, since touch users
/// never get a hover tooltip.
#[derive(Debug, Clone, Copy)]
pub struct MuteButton {
    pub center: Vec2,
}

impl Default for MuteButton {
    fn default() -> Self {
        Self {
            center: vec2(56.0, 56.0),
        }
    }
}

impl MuteButton {
    pub const SIZE: f32 = 44.0;
    /// Invisible padding around the plaque, in virtual pixels.
    ///
    /// The full target is 96 virtual px. Scaled into a 390 px wide
    /// portrait phone that is still ~47 CSS px, above the usual 44 px
    /// minimum touch target.
    pub const HIT_PADDING: f32 = 26.0;

    pub fn contains(&self, point: Vec2) -> bool {
        let half = (Self::SIZE + Self::HIT_PADDING * 2.0) * 0.5;
        (point.x - self.center.x).abs() <= half && (point.y - self.center.y).abs() <= half
    }

    pub fn draw(&self, r: &mut dyn RenderingServer, muted: bool, hovered: bool) {
        let half = Self::SIZE * 0.5;
        let x = self.center.x - half;
        let y = self.center.y - half;

        r.draw_rect(
            Rect::new(x + 2.0, y + 3.0, Self::SIZE, Self::SIZE),
            Color::new(0.0, 0.0, 0.0, 0.35),
        );
        r.draw_rect(
            Rect::new(x, y, Self::SIZE, Self::SIZE),
            Color::new(0.035, 0.035, 0.055, if hovered { 0.88 } else { 0.64 }),
        );
        r.draw_rect_outline(
            Rect::new(x, y, Self::SIZE, Self::SIZE),
            if hovered { 2.0 } else { 1.5 },
            with_alpha(PERFECT_GOLD, if hovered { 1.0 } else { 0.48 }),
        );

        // Speaker: small magnet plus a horn that opens to the right.
        let cx = self.center.x - 2.0;
        let cy = self.center.y;
        let tint = with_alpha(
            if hovered { PERFECT_GOLD } else { IVORY },
            if muted { 0.52 } else { 1.0 },
        );

        r.draw_rect(Rect::new(cx - 11.0, cy - 4.0, 5.0, 8.0), tint);
        r.draw_triangle(
            vec2(cx - 6.0, cy - 4.0),
            vec2(cx + 2.0, cy - 10.0),
            vec2(cx + 2.0, cy + 10.0),
            tint,
        );
        r.draw_triangle(
            vec2(cx - 6.0, cy - 4.0),
            vec2(cx - 6.0, cy + 4.0),
            vec2(cx + 2.0, cy + 10.0),
            tint,
        );

        if muted {
            // Cinnabar slash: the classic mute mark, drawn over the horn.
            r.draw_line(
                Vec2::new(cx - 12.0, cy + 11.0),
                Vec2::new(cx + 13.0, cy - 11.0),
                4.2,
                Color::new(0.04, 0.01, 0.01, 0.75),
            );
            r.draw_line(
                Vec2::new(cx - 12.0, cy + 11.0),
                Vec2::new(cx + 13.0, cy - 11.0),
                2.2,
                CINNABAR,
            );
        } else {
            for (radius, alpha) in [(7.0, 0.88), (12.0, 0.55)] {
                r.draw_arc(
                    Vec2::new(cx + 4.0, cy),
                    radius,
                    f32::to_radians(-50.0),
                    f32::to_radians(100.0),
                    1.6,
                    14,
                    with_alpha(IVORY, alpha),
                );
            }
        }

        if muted {
            r.draw_text(
                "SOUND OFF",
                Vec2::new(self.center.x + 34.0, self.center.y + 5.0),
                13.0,
                with_alpha(CINNABAR, 0.95),
            );
        } else if hovered {
            r.draw_text(
                "MUTE",
                Vec2::new(self.center.x + 34.0, self.center.y + 5.0),
                14.0,
                with_alpha(IVORY, 0.95),
            );
        }
    }
}

/// Text centred on `center_x`, baseline at `y` (as
/// `ember_stdlib::graphics::text::draw_centered` did).
pub fn draw_centered(
    r: &mut dyn RenderingServer,
    text: &str,
    center_x: f32,
    y: f32,
    font_size: u16,
    color: Color,
) {
    let size = r.measure_text(text, font_size as f32);
    r.draw_text(
        text,
        Vec2::new(center_x - size.width / 2.0, y),
        font_size as f32,
        color,
    );
}

/// [`draw_centered`] with a drop shadow `shadow_offset` px down-right.
#[allow(clippy::too_many_arguments)]
pub fn draw_centered_shadowed(
    r: &mut dyn RenderingServer,
    text: &str,
    center_x: f32,
    y: f32,
    font_size: u16,
    color: Color,
    shadow_color: Color,
    shadow_offset: f32,
) {
    let size = r.measure_text(text, font_size as f32);
    let x = center_x - size.width / 2.0;
    let shadow = Vec2::new(x + shadow_offset, y + shadow_offset);
    r.draw_text(text, shadow, font_size as f32, shadow_color);
    r.draw_text(text, Vec2::new(x, y), font_size as f32, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_glyph_falls_back_without_a_cjk_font() {
        // Smoke test: the fallback path must be chosen when no font is
        // loaded. Rendering itself needs a macroquad context, so this
        // only covers the availability predicate used by `draw_seal`.
        let cjk = CjkFont::hermetic();
        assert!(!cjk.is_available());
    }

    #[test]
    fn mute_button_hit_area_covers_the_plaque_and_its_padding() {
        let button = MuteButton::default();
        assert!(button.contains(button.center));
        assert!(button.contains(button.center + vec2(45.0, 0.0)));
        assert!(button.contains(button.center + vec2(0.0, -45.0)));
        assert!(!button.contains(button.center + vec2(50.0, 0.0)));
        assert!(!button.contains(button.center + vec2(0.0, 50.0)));
    }

    #[test]
    fn mute_button_meets_a_mobile_touch_target() {
        // A 390×844 portrait phone scales the 800×600 canvas down to
        // 0.4875. The invisible hit area must still be at least the
        // usual 44 CSS px recommended for touch controls.
        let button = MuteButton::default();
        let target_virtual = MuteButton::SIZE + MuteButton::HIT_PADDING * 2.0;
        let target_css = target_virtual * 390.0 / 800.0;
        assert!(target_css >= 44.0, "touch target was {target_css} CSS px");
        assert!(button.contains(button.center + vec2(45.0, 45.0)));
    }

    #[test]
    fn mute_button_sits_inside_the_play_field() {
        let button = MuteButton::default();
        let half = MuteButton::SIZE * 0.5;
        assert!(button.center.x - half > 0.0 && button.center.y - half > 0.0);
        assert!(button.center.x + half < 800.0);
        assert!(button.center.y + half < 600.0);
    }
}
