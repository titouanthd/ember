//! `Anchors`: where a control sits inside its parent (spec §4.2).

use ember_core::math::Rect;

/// Fractions of the parent rect: `0.0` = left/top edge, `1.0` =
/// right/bottom edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchors {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Anchors {
    /// Fill the parent.
    pub const FULL_RECT: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    /// Zero-size point at the parent's top-left; the control uses its
    /// desired / minimum size.
    pub const TOP_LEFT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// Zero-size point at the parent's centre.
    pub const CENTER: Self = Self::new(0.5, 0.5, 0.5, 0.5);
    pub const TOP_RIGHT: Self = Self::new(1.0, 0.0, 1.0, 0.0);
    pub const BOTTOM_LEFT: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BOTTOM_RIGHT: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Full width, zero height at the top (a header bar sized by its
    /// minimum height).
    pub const TOP_WIDE: Self = Self::new(0.0, 0.0, 1.0, 0.0);
    pub const BOTTOM_WIDE: Self = Self::new(0.0, 1.0, 1.0, 1.0);

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Map the anchors into `parent` (spec §4.2):
    /// `x = parent.x + left * parent.w`, `w = (right - left) * parent.w`,
    /// same for y/h. Negative sizes (right < left) clamp to zero.
    pub fn compute_rect(self, parent: Rect) -> Rect {
        let left_x = parent.x + self.left * parent.w;
        let right_x = parent.x + self.right * parent.w;
        let top_y = parent.y + self.top * parent.h;
        let bottom_y = parent.y + self.bottom * parent.h;
        Rect::new(
            left_x,
            top_y,
            (right_x - left_x).max(0.0),
            (bottom_y - top_y).max(0.0),
        )
    }
}

impl Default for Anchors {
    fn default() -> Self {
        Self::TOP_LEFT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARENT: Rect = Rect::new(100.0, 50.0, 400.0, 200.0);

    #[test]
    fn full_rect_fills_the_parent() {
        assert_eq!(Anchors::FULL_RECT.compute_rect(PARENT), PARENT);
    }

    #[test]
    fn point_anchors_have_zero_size() {
        assert_eq!(
            Anchors::TOP_LEFT.compute_rect(PARENT),
            Rect::new(100.0, 50.0, 0.0, 0.0)
        );
        assert_eq!(
            Anchors::CENTER.compute_rect(PARENT),
            Rect::new(300.0, 150.0, 0.0, 0.0)
        );
        assert_eq!(
            Anchors::BOTTOM_RIGHT.compute_rect(PARENT),
            Rect::new(500.0, 250.0, 0.0, 0.0)
        );
    }

    #[test]
    fn wide_anchors_span_one_axis() {
        assert_eq!(
            Anchors::TOP_WIDE.compute_rect(PARENT),
            Rect::new(100.0, 50.0, 400.0, 0.0)
        );
    }

    #[test]
    fn fractional_anchors() {
        let right_half = Anchors::new(0.5, 0.0, 1.0, 1.0);
        assert_eq!(
            right_half.compute_rect(PARENT),
            Rect::new(300.0, 50.0, 200.0, 200.0)
        );
    }

    #[test]
    fn inverted_anchors_clamp_to_zero_size() {
        let r = Anchors::new(0.8, 0.8, 0.2, 0.2).compute_rect(PARENT);
        assert_eq!((r.w, r.h), (0.0, 0.0));
    }
}
