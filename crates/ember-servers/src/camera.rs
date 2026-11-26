//! `Camera2d`: which part of the world the screen shows (ADR-040).

use ember_core::math::Rect;
use glam::Vec2;

/// A 2D camera: `offset` is the world point at the screen's top-left,
/// `zoom` the screen pixels per world unit. A world point `p` lands at
/// `(p - offset) * zoom` on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2d {
    pub offset: Vec2,
    /// `1.0` = one pixel per unit; `0.5` shows twice as much. Must be > 0.
    pub zoom: f32,
}

impl Default for Camera2d {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Camera2d {
    /// World = screen.
    pub const IDENTITY: Self = Self {
        offset: Vec2::ZERO,
        zoom: 1.0,
    };

    /// A camera that scrolls only (zoom `1.0`).
    pub const fn at(offset: Vec2) -> Self {
        Self { offset, zoom: 1.0 }
    }

    pub const fn with_zoom(self, zoom: f32) -> Self {
        Self { zoom, ..self }
    }

    /// The camera that draws world `(0, 0)` at screen `origin`, scaled
    /// by `zoom`.
    pub fn placing_origin_at(origin: Vec2, zoom: f32) -> Self {
        Self {
            offset: -origin / zoom,
            zoom,
        }
    }

    /// The screen rect that shows a `size` canvas scaled to fit `window`,
    /// aspect kept and centred; the rest is bars (ADR-041). A degenerate
    /// size gives the canvas at the top-left, unscaled.
    pub fn letterbox_rect(window: Vec2, size: Vec2) -> Rect {
        if window.x <= 0.0 || window.y <= 0.0 || size.x <= 0.0 || size.y <= 0.0 {
            return Rect::new(0.0, 0.0, size.x.max(0.0), size.y.max(0.0));
        }
        let scale = (window.x / size.x).min(window.y / size.y);
        let shown = size * scale;
        let corner = (window - shown) / 2.0;
        Rect::new(corner.x, corner.y, shown.x, shown.y)
    }

    /// The camera that draws a `size` canvas into its letterbox rect.
    pub fn letterbox(window: Vec2, size: Vec2) -> Self {
        let r = Self::letterbox_rect(window, size);
        let zoom = if size.x > 0.0 { r.w / size.x } else { 1.0 };
        Self::placing_origin_at(Vec2::new(r.x, r.y), zoom.max(f32::MIN_POSITIVE))
    }

    /// This camera with the world scaled by `factor` about `world`
    /// first (a "punch" zoom): `world` stays where it was on screen.
    pub fn zoomed_about(self, world: Vec2, factor: f32) -> Self {
        Self {
            offset: world - (world - self.offset) / factor,
            zoom: self.zoom * factor,
        }
    }

    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        (world - self.offset) * self.zoom
    }

    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        screen / self.zoom + self.offset
    }

    /// A world length (size, radius, thickness, font size) on screen.
    pub fn length(&self, world: f32) -> f32 {
        world * self.zoom
    }

    pub fn rect_to_screen(&self, world: Rect) -> Rect {
        let p = self.world_to_screen(Vec2::new(world.x, world.y));
        Rect::new(p.x, p.y, self.length(world.w), self.length(world.h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_scroll_keep_the_old_offset_rule() {
        let p = Vec2::new(15.0, 25.0);
        assert_eq!(Camera2d::IDENTITY.world_to_screen(p), p);
        assert_eq!(
            Camera2d::at(Vec2::new(10.0, 20.0)).world_to_screen(p),
            Vec2::new(5.0, 5.0)
        );
    }

    #[test]
    fn zoom_scales_around_the_offset_and_round_trips() {
        let c = Camera2d::at(Vec2::new(100.0, 0.0)).with_zoom(0.5);
        let w = Vec2::new(300.0, 40.0);
        assert_eq!(c.world_to_screen(w), Vec2::new(100.0, 20.0));
        assert_eq!(c.screen_to_world(c.world_to_screen(w)), w);
        assert_eq!(
            c.rect_to_screen(Rect::new(100.0, 0.0, 800.0, 600.0)),
            Rect::new(0.0, 0.0, 400.0, 300.0)
        );
        assert_eq!(c.length(48.0), 24.0);
    }

    // The two cases below were taiji's `render::letterbox_rect` tests.
    #[test]
    fn letterbox_preserves_aspect_ratio() {
        let r = Camera2d::letterbox_rect(Vec2::new(1920.0, 1080.0), Vec2::new(800.0, 600.0));
        assert!((r.w / r.h - 800.0 / 600.0).abs() < 1e-3);
    }

    #[test]
    fn letterbox_centers_on_wide_screen() {
        let r = Camera2d::letterbox_rect(Vec2::new(1920.0, 1080.0), Vec2::new(800.0, 600.0));
        assert!(r.x > 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.h, 1080.0);
    }

    #[test]
    fn the_letterbox_camera_maps_the_canvas_onto_its_rect() {
        let (window, size) = (Vec2::new(1000.0, 1000.0), Vec2::new(800.0, 600.0));
        let c = Camera2d::letterbox(window, size);
        assert_eq!(c.zoom, 1.25);
        assert_eq!(c.world_to_screen(Vec2::ZERO), Vec2::new(0.0, 125.0));
        assert_eq!(c.world_to_screen(size), Vec2::new(1000.0, 875.0));
        assert_eq!(
            c.screen_to_world(Vec2::new(500.0, 500.0)),
            Vec2::new(400.0, 300.0)
        );
        let same = Camera2d::letterbox(size, size);
        assert_eq!(same, Camera2d::IDENTITY);
    }

    #[test]
    fn zooming_about_a_point_keeps_it_in_place() {
        let base = Camera2d::letterbox(Vec2::new(1000.0, 1000.0), Vec2::new(800.0, 600.0));
        let centre = Vec2::new(400.0, 300.0);
        let punched = base.zoomed_about(centre, 1.03);
        assert!((punched.world_to_screen(centre) - base.world_to_screen(centre)).length() < 1e-3);
        assert!((punched.zoom - base.zoom * 1.03).abs() < 1e-6);
        // A corner moves outward by 3 % of its distance to the centre.
        let corner = punched.world_to_screen(Vec2::ZERO);
        let expect = base.world_to_screen(centre)
            - (base.world_to_screen(centre) - base.world_to_screen(Vec2::ZERO)) * 1.03;
        assert!((corner - expect).length() < 1e-3);
    }

    #[test]
    fn placing_origin_puts_world_zero_there() {
        let c = Camera2d::placing_origin_at(Vec2::new(260.0, 80.0), 0.625);
        assert_eq!(c.world_to_screen(Vec2::ZERO), Vec2::new(260.0, 80.0));
        assert_eq!(
            c.world_to_screen(Vec2::new(800.0, 600.0)),
            Vec2::new(760.0, 455.0)
        );
    }
}
