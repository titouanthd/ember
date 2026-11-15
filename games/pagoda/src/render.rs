// games/pagoda/src/render.rs
//! Rendering helpers (letterbox math and virtual-canvas mapping).

use glam::Vec2;

/// Rect of the virtual canvas inside the window: `(x, y, width, height)`.
pub fn letterbox_rect(
    win_w: f32,
    win_h: f32,
    virtual_w: f32,
    virtual_h: f32,
) -> (f32, f32, f32, f32) {
    if win_w <= 0.0 || win_h <= 0.0 || virtual_w <= 0.0 || virtual_h <= 0.0 {
        return (0.0, 0.0, virtual_w.max(0.0), virtual_h.max(0.0));
    }

    let scale = (win_w / virtual_w).min(win_h / virtual_h);
    let dst_w = virtual_w * scale;
    let dst_h = virtual_h * scale;
    let dst_x = (win_w - dst_w) / 2.0;
    let dst_y = (win_h - dst_h) / 2.0;
    (dst_x, dst_y, dst_w, dst_h)
}

/// Convert a window-space point to the 800×600 virtual canvas.
///
/// Returns `None` when the point lies in the letterbox bars, so a click
/// outside the play area can never toggle a UI control.
pub fn window_to_virtual(
    point: Vec2,
    win_w: f32,
    win_h: f32,
    virtual_w: f32,
    virtual_h: f32,
) -> Option<Vec2> {
    if win_w <= 0.0 || win_h <= 0.0 || virtual_w <= 0.0 || virtual_h <= 0.0 {
        return None;
    }

    let (dst_x, dst_y, dst_w, dst_h) = letterbox_rect(win_w, win_h, virtual_w, virtual_h);
    if dst_w <= 0.0 || dst_h <= 0.0 {
        return None;
    }
    if point.x < dst_x || point.x > dst_x + dst_w || point.y < dst_y || point.y > dst_y + dst_h {
        return None;
    }

    Some(Vec2::new(
        (point.x - dst_x) / dst_w * virtual_w,
        (point.y - dst_y) / dst_h * virtual_h,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn letterbox_centers_a_matching_aspect_ratio() {
        let (x, y, w, h) = letterbox_rect(800.0, 600.0, 800.0, 600.0);
        assert_eq!((x, y, w, h), (0.0, 0.0, 800.0, 600.0));
    }

    #[test]
    fn letterbox_pillarboxes_a_wide_window() {
        let (x, y, w, h) = letterbox_rect(1600.0, 600.0, 800.0, 600.0);
        assert_eq!((x, y), (400.0, 0.0));
        assert_eq!((w, h), (800.0, 600.0));
    }

    #[test]
    fn window_point_maps_to_virtual_space() {
        // 1920×1080 is a 1.8× scale with 240 px pillarboxes.
        let center = window_to_virtual(Vec2::new(960.0, 540.0), 1920.0, 1080.0, 800.0, 600.0)
            .expect("center is inside");
        assert!(close(center.x, 400.0));
        assert!(close(center.y, 300.0));

        let top_left = window_to_virtual(Vec2::new(240.0, 0.0), 1920.0, 1080.0, 800.0, 600.0)
            .expect("corner is inside");
        assert!(close(top_left.x, 0.0));
        assert!(close(top_left.y, 0.0));

        let bottom_right =
            window_to_virtual(Vec2::new(1680.0, 1080.0), 1920.0, 1080.0, 800.0, 600.0)
                .expect("corner is inside");
        assert!(close(bottom_right.x, 800.0));
        assert!(close(bottom_right.y, 600.0));
    }

    #[test]
    fn letterbox_bar_is_outside_the_game() {
        assert!(window_to_virtual(Vec2::new(120.0, 540.0), 1920.0, 1080.0, 800.0, 600.0).is_none());
        assert!(
            window_to_virtual(Vec2::new(1800.0, 540.0), 1920.0, 1080.0, 800.0, 600.0).is_none()
        );
    }

    #[test]
    fn degenerate_window_does_not_divide_by_zero() {
        assert!(window_to_virtual(Vec2::ZERO, 0.0, 0.0, 800.0, 600.0).is_none());
    }
}
