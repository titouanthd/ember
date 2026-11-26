//! Widgets UI spécifiques à Zhuo Ji.
//!
//! Regroupe les helpers de rendu qui ne sont pas dans la stdlib mais qui
//! sont utilisés par plusieurs modules de Zhuo Ji (menu, help, modals).

use ember::prelude::*;

pub mod panel;

/// Center a `w × h` rect inside a `vw × vh` viewport (was
/// `ember_stdlib::layout::centered_rect`, which returns a macroquad rect).
pub fn centered_rect(vw: f32, vh: f32, w: f32, h: f32) -> Rect {
    Rect::new((vw - w) * 0.5, (vh - h) * 0.5, w, h)
}

/// Default-font text centred on `cx`; `y` is the baseline (was
/// `ember_stdlib::graphics::text::draw_centered`).
pub fn draw_centered(
    gfx: &mut dyn RenderingServer,
    text: &str,
    cx: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    let width = gfx.measure_text(text, size).width;
    gfx.draw_text(text, Vec2::new(cx - width / 2.0, y), size, color);
}

/// Default-font text whose right edge sits on `rx` (was
/// `ember_stdlib::graphics::text::draw_right`).
pub fn draw_right(
    gfx: &mut dyn RenderingServer,
    text: &str,
    rx: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    let width = gfx.measure_text(text, size).width;
    gfx.draw_text(text, Vec2::new(rx - width, y), size, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_is_symmetric() {
        let r = centered_rect(1000.0, 800.0, 600.0, 400.0);
        assert_eq!(r, Rect::new(200.0, 200.0, 600.0, 400.0));
    }
}
