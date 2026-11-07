// games/taiji/src/render.rs
//! Rendering helpers (letterbox math, etc.).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_preserves_aspect_ratio() {
        let (_, _, w, h) = letterbox_rect(1920.0, 1080.0, 800.0, 600.0);
        assert!((w / h - 800.0 / 600.0).abs() < 1e-3);
    }

    #[test]
    fn letterbox_centers_on_wide_screen() {
        let (x, y, _, h) = letterbox_rect(1920.0, 1080.0, 800.0, 600.0);
        assert!(x > 0.0);
        assert_eq!(y, 0.0);
        assert_eq!(h, 1080.0);
    }
}