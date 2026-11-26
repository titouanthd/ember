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
