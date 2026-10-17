//! Layout helpers: panel/modal centering.
//!
//! Extracted after 3 games (ember-wars, jade-garden, zhuo-ji) each
//! had 2-3 copies of the same "center a fixed-size panel in the
//! viewport" arithmetic. Rule of Three satisfied.

use macroquad::prelude::Rect;

/// Center a `w × h` rect inside a `vw × vh` viewport.
///
/// Returns the rect positioned so its **top-left corner** is at
/// `((vw - w) / 2, (vh - h) / 2)`. The caller clamps `w`/`h` to
/// `vw`/`vh` if the panel can be larger than the screen.
pub fn centered_rect(vw: f32, vh: f32, w: f32, h: f32) -> Rect {
    Rect::new((vw - w) * 0.5, (vh - h) * 0.5, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_is_symmetric() {
        let r = centered_rect(1000.0, 800.0, 600.0, 400.0);
        assert!((r.x - 200.0).abs() < 1e-6);
        assert!((r.y - 200.0).abs() < 1e-6);
        assert!((r.w - 600.0).abs() < 1e-6);
        assert!((r.h - 400.0).abs() < 1e-6);
    }

    #[test]
    fn centered_rect_left_margin_equals_right_margin() {
        let r = centered_rect(1000.0, 800.0, 600.0, 400.0);
        let left = r.x;
        let right = 1000.0 - (r.x + r.w);
        assert!((left - right).abs() < 1e-6);
    }

    #[test]
    fn centered_rect_top_margin_equals_bottom_margin() {
        let r = centered_rect(1000.0, 800.0, 600.0, 400.0);
        let top = r.y;
        let bottom = 800.0 - (r.y + r.h);
        assert!((top - bottom).abs() < 1e-6);
    }

    #[test]
    fn centered_rect_same_size_is_at_origin() {
        let r = centered_rect(800.0, 600.0, 800.0, 600.0);
        assert!((r.x).abs() < 1e-6);
        assert!((r.y).abs() < 1e-6);
    }

    #[test]
    fn centered_rect_odd_difference_rounds_to_half_pixel() {
        let r = centered_rect(1001.0, 801.0, 600.0, 400.0);
        assert!((r.x - 200.5).abs() < 1e-6);
        assert!((r.y - 200.5).abs() < 1e-6);
    }
}