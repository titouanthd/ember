//! `Margins`: padding inside a container, in pixels.

use ember_core::math::Rect;
use glam::Vec2;

/// Space kept free inside a container's rect, per side. The spec writes
/// padding as a `Rect`; separate sides read better (ADR-034).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Margins {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Margins {
    pub const ZERO: Self = Self::all(0.0);

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn all(v: f32) -> Self {
        Self::new(v, v, v, v)
    }

    /// `h` on the left and right, `v` on the top and bottom.
    pub const fn symmetric(h: f32, v: f32) -> Self {
        Self::new(h, v, h, v)
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    /// `rect` minus the margins; never negative.
    pub fn shrink(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.x + self.left,
            rect.y + self.top,
            (rect.w - self.horizontal()).max(0.0),
            (rect.h - self.vertical()).max(0.0),
        )
    }

    /// A content size plus the margins.
    pub fn grow(&self, size: Vec2) -> Vec2 {
        Vec2::new(size.x + self.horizontal(), size.y + self.vertical())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_and_grow_are_inverse_on_size() {
        let m = Margins::new(1.0, 2.0, 3.0, 4.0);
        let r = m.shrink(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(r, Rect::new(1.0, 2.0, 96.0, 44.0));
        assert_eq!(m.grow(r.size()), Vec2::new(100.0, 50.0));
    }

    #[test]
    fn shrink_never_goes_negative() {
        let r = Margins::all(30.0).shrink(Rect::new(0.0, 0.0, 40.0, 10.0));
        assert_eq!((r.w, r.h), (0.0, 0.0));
    }
}
