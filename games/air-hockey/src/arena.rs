//! Arena geometry. Pure math, no macroquad, no rendering.
//!
//! Coordinate system: origin top-left, x grows right, y grows down.
//! Player 1 (Left) defends the left goal; Player 2 (Right) the right goal.

use ember_core::math::Aabb;
use glam::Vec2;

use crate::components::Side;

/// Static arena layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arena {
    /// Full table width in world units.
    pub w: f32,
    /// Full table height in world units.
    pub h: f32,
    /// Height of each goal opening (vertical extent on the left/right walls).
    pub goal_h: f32,
    /// Wall thickness used for the visible border and for wall bounces.
    pub wall: f32,
}

impl Arena {
    /// Default arena per DESIGN.md §12.
    pub const fn default_layout() -> Self {
        Self { w: 1280.0, h: 720.0, goal_h: 158.0, wall: 6.0 }
    }

    /// Horizontal centre line.
    #[inline]
    pub fn mid_x(&self) -> f32 {
        self.w * 0.5
    }

    /// Vertical centre.
    #[inline]
    pub fn mid_y(&self) -> f32 {
        self.h * 0.5
    }

    /// Top edge of the goal opening (both goals share the same y-range).
    #[inline]
    pub fn goal_top(&self) -> f32 {
        self.mid_y() - self.goal_h * 0.5
    }

    /// Bottom edge of the goal opening.
    #[inline]
    pub fn goal_bottom(&self) -> f32 {
        self.mid_y() + self.goal_h * 0.5
    }

    /// `true` if `y` is inside the goal opening (inclusive on both ends).
    #[inline]
    pub fn is_in_goal_y(&self, y: f32) -> bool {
        (self.goal_top()..=self.goal_bottom()).contains(&y)
    }

    /// `true` if the puck has fully crossed the left goal line.
    #[inline]
    pub fn is_past_left_goal_line(&self, x: f32, radius: f32) -> bool {
        x - radius <= 0.0
    }

    /// `true` if the puck has fully crossed the right goal line.
    #[inline]
    pub fn is_past_right_goal_line(&self, x: f32, radius: f32) -> bool {
        x + radius >= self.w
    }

    /// Playable half for the given side. Inset by `wall` so paddles
    /// cannot visually overlap the border.
    ///
    /// Left half:  x ∈ [wall + r, mid_x - r]
    /// Right half: x ∈ [mid_x + r, w - wall - r]
    ///
    /// `r` is the paddle radius; the caller passes it so the box is exact.
    pub fn half_for(&self, side: Side, r: f32) -> Aabb {
        let top = self.wall + r;
        let bottom = self.h - self.wall - r;
        match side {
            Side::Left => Aabb::from_corners(
                Vec2::new(self.wall + r, top),
                Vec2::new(self.mid_x() - r, bottom),
            ),
            Side::Right => Aabb::from_corners(
                Vec2::new(self.mid_x() + r, top),
                Vec2::new(self.w - self.wall - r, bottom),
            ),
        }
    }

    /// Clamp a paddle centre to its half, keeping `r` away from every edge.
    #[inline]
    pub fn clamp_to_half(&self, pos: Vec2, side: Side, r: f32) -> Vec2 {
        let half = self.half_for(side, r);
        Vec2::new(
            pos.x.clamp(half.min.x, half.max.x),
            pos.y.clamp(half.min.y, half.max.y),
        )
    }

    /// Top wall as an AABB (used for swept/collision helpers if ever needed).
    /// Left and right walls are **not** full height — they have a gap for
    /// the goal. Callers should use `is_in_goal_y` instead.
    pub fn wall_top(&self) -> Aabb {
        Aabb::from_corners(Vec2::ZERO, Vec2::new(self.w, self.wall))
    }

    pub fn wall_bottom(&self) -> Aabb {
        Aabb::from_corners(
            Vec2::new(0.0, self.h - self.wall),
            Vec2::new(self.w, self.h),
        )
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::default_layout()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_zone_bounds() {
        let a = Arena::default_layout();
        // Centred vertically.
        assert!((a.goal_top() - (720.0 - 158.0) * 0.5).abs() < 1e-4);
        assert!((a.goal_bottom() - (720.0 + 158.0) * 0.5).abs() < 1e-4);
        // Exact height.
        assert!((a.goal_bottom() - a.goal_top() - a.goal_h).abs() < 1e-4);
    }

    #[test]
    fn test_clamp_left_paddle_stays_left() {
        let a = Arena::default_layout();
        let r = 28.0;
        // Try to walk far past the midline and off every edge.
        let p = a.clamp_to_half(Vec2::new(9999.0, -9999.0), Side::Left, r);
        assert!(p.x <= a.mid_x() - r + 1e-4);
        assert!(p.x >= a.wall + r - 1e-4);
        assert!(p.y >= a.wall + r - 1e-4);
        // And the opposite corner.
        let p2 = a.clamp_to_half(Vec2::new(-9999.0, 9999.0), Side::Left, r);
        assert!(p2.x >= a.wall + r - 1e-4);
        assert!(p2.y <= a.h - a.wall - r + 1e-4);
    }

    #[test]
    fn test_clamp_right_paddle_stays_right() {
        let a = Arena::default_layout();
        let r = 28.0;
        let p = a.clamp_to_half(Vec2::new(-9999.0, 0.0), Side::Right, r);
        assert!(p.x >= a.mid_x() + r - 1e-4);
        assert!(p.x <= a.w - a.wall - r + 1e-4);
        let p2 = a.clamp_to_half(Vec2::new(9999.0, 0.0), Side::Right, r);
        assert!(p2.x <= a.w - a.wall - r + 1e-4);
    }

    #[test]
    fn test_goal_y_inside_and_outside() {
        let a = Arena::default_layout();
        assert!(a.is_in_goal_y(a.goal_top()));
        assert!(a.is_in_goal_y(a.goal_bottom()));
        assert!(a.is_in_goal_y(a.mid_y()));
        assert!(!a.is_in_goal_y(a.goal_top() - 1.0));
        assert!(!a.is_in_goal_y(a.goal_bottom() + 1.0));
        assert!(!a.is_in_goal_y(0.0));
        assert!(!a.is_in_goal_y(a.h));
    }

    #[test]
    fn test_goal_line_detection_both_sides() {
        let a = Arena::default_layout();
        let r = 14.0; // puck radius
        // Left: puck must fully cross x=0.
        assert!(!a.is_past_left_goal_line(r + 0.5, r));
        assert!(a.is_past_left_goal_line(r - 0.5, r));
        assert!(a.is_past_left_goal_line(0.0, r));
        // Right: symmetric.
        assert!(!a.is_past_right_goal_line(a.w - r - 0.5, r));
        assert!(a.is_past_right_goal_line(a.w - r + 0.5, r));
        assert!(a.is_past_right_goal_line(a.w, r));
    }
}