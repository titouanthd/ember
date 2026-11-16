//! Axis-aligned rectangle in `x, y, w, h` form.
//!
//! `Rect` is the "top-left + size" spelling; [`Aabb`](crate::math::Aabb)
//! is the "min/max" spelling. They round-trip losslessly. Use `Rect` for
//! UI and layout code (it matches macroquad's convention), `Aabb` for
//! physics and collision math (it exposes penetration and closest-point).

use crate::math::Aabb;
use glam::Vec2;

/// A rectangle with top-left corner `(x, y)` and size `(w, h)`.
///
/// Same conventions as `Aabb`: `+Y` is down; widths and heights are
/// expected non-negative, but the type does not enforce it (negative
/// sizes are useful for edge cases like a collapsed divider).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self::new(min.x, min.y, size.x, size.y)
    }

    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self::new(
            center.x - size.x / 2.0,
            center.y - size.y / 2.0,
            size.x,
            size.y,
        )
    }

    pub fn min(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn max(&self) -> Vec2 {
        Vec2::new(self.x + self.w, self.y + self.h)
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.w, self.h)
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Inclusive contains (borders belong to the rect).
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    /// True when the two rects overlap with positive area.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// The intersection of two rects, or `None` when they do not overlap
    /// with positive area. Edge-touching rects return `None` (zero-width
    /// intersections have no meaningful use in layout).
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        if x1 > x0 && y1 > y0 {
            Some(Self::new(x0, y0, x1 - x0, y1 - y0))
        } else {
            None
        }
    }

    pub fn to_aabb(&self) -> Aabb {
        Aabb::from_top_left_size(self.min(), self.size())
    }

    pub fn from_aabb(aabb: &Aabb) -> Self {
        Self::new(aabb.min.x, aabb.min.y, aabb.width(), aabb.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Rect {
        Rect::new(10.0, 20.0, 100.0, 50.0)
    }

    #[test]
    fn min_max_center_size() {
        let r = r();
        assert_eq!(r.min(), Vec2::new(10.0, 20.0));
        assert_eq!(r.max(), Vec2::new(110.0, 70.0));
        assert_eq!(r.size(), Vec2::new(100.0, 50.0));
        assert_eq!(r.center(), Vec2::new(60.0, 45.0));
    }

    #[test]
    fn contains_is_inclusive() {
        let r = r();
        assert!(r.contains(Vec2::new(10.0, 20.0)));
        assert!(r.contains(Vec2::new(110.0, 70.0)));
        assert!(!r.contains(Vec2::new(9.9, 45.0)));
    }

    #[test]
    fn intersect_positive_area() {
        let a = r();
        let b = Rect::new(50.0, 40.0, 100.0, 100.0);
        let hit = a.intersect(&b).expect("overlap");
        assert_eq!(hit, Rect::new(50.0, 40.0, 60.0, 30.0));
    }

    #[test]
    fn intersect_touching_edges_is_none() {
        let a = r();
        let b = Rect::new(110.0, 20.0, 50.0, 50.0);
        assert!(a.intersect(&b).is_none());
        assert!(!a.intersects(&b));
    }

    #[test]
    fn aabb_roundtrip() {
        let a = r();
        let aabb = a.to_aabb();
        assert_eq!(Rect::from_aabb(&aabb), a);
    }
}
