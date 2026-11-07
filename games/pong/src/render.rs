// games/pong/src/render.rs
//! Rendering helpers (letterbox math, etc.).
//!
//! Pure math — no macroquad dependency. Testable without a graphics context.

/// Compute the destination rect for a virtual `virtual_w × virtual_h`
/// canvas displayed in a `win_w × win_h` window, preserving aspect ratio
/// and centering with letterbox bars.
///
/// Returns `(x, y, w, h)` — the destination rect in window coordinates.
///
/// - If the window aspect ratio matches the virtual one → full window, no bars.
/// - If wider than virtual → horizontal bars on left and right.
/// - If taller than virtual → horizontal bars on top and bottom.
/// - If smaller than virtual → scaled down.
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

    const VW: f32 = 800.0;
    const VH: f32 = 600.0;

    #[test]
    fn window_wider_than_virtual_16_9() {
        // 800×600 (4:3) game on a 1920×1080 (16:9) window → side bars.
        let (x, y, w, h) = letterbox_rect(1920.0, 1080.0, VW, VH);
        // scale = min(2.4, 1.8) = 1.8 ; w = 1440, h = 1080.
        assert!((w - 1440.0).abs() < 1e-3);
        assert!((h - 1080.0).abs() < 1e-3);
        assert!((x - 240.0).abs() < 1e-3);
        assert!((y - 0.0).abs() < 1e-3);
    }

    #[test]
    fn window_matches_virtual_aspect() {
        // Same aspect ratio → no letterbox.
        let (x, y, w, h) = letterbox_rect(1600.0, 1200.0, VW, VH);
        assert!((w - 1600.0).abs() < 1e-3);
        assert!((h - 1200.0).abs() < 1e-3);
        assert!((x - 0.0).abs() < 1e-3);
        assert!((y - 0.0).abs() < 1e-3);
    }

    #[test]
    fn window_taller_than_virtual() {
        // 1080×1920 (portrait) → top and bottom bars.
        let (x, y, w, h) = letterbox_rect(1080.0, 1920.0, VW, VH);
        // scale = min(1.35, 3.2) = 1.35 ; w = 1080, h = 810.
        assert!((w - 1080.0).abs() < 1e-3);
        assert!((h - 810.0).abs() < 1e-3);
        assert!((x - 0.0).abs() < 1e-3);
        assert!((y - 555.0).abs() < 1e-3);
    }

    #[test]
    fn window_smaller_than_virtual() {
        // Downscale case.
        let (x, y, w, h) = letterbox_rect(400.0, 300.0, VW, VH);
        assert!((w - 400.0).abs() < 1e-3);
        assert!((h - 300.0).abs() < 1e-3);
        assert!((x - 0.0).abs() < 1e-3);
        assert!((y - 0.0).abs() < 1e-3);
    }

    #[test]
    fn preserves_aspect_ratio() {
        let (_, _, w, h) = letterbox_rect(1920.0, 1080.0, VW, VH);
        assert!((w / h - VW / VH).abs() < 1e-3);
    }

    #[test]
    fn degenerate_window_does_not_panic() {
        let (_, _, w, h) = letterbox_rect(0.0, 0.0, VW, VH);
        // Fallback: virtual size, no crash.
        assert!((w - VW).abs() < 1e-3);
        assert!((h - VH).abs() < 1e-3);
    }
}