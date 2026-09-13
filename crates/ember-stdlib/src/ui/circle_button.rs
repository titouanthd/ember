//! Circular button widget. Same interaction model as Button, but hit-tested
//! on a circle instead of a rectangle.
//!
//! Rendering note: `draw_circle` from macroquad tessellates with a fixed
//! segment count that doesn't scale with radius, and `draw_circle_lines`
//! uses a DIFFERENT count. At R=100 that shows up as faceted edges and a
//! visible gap between fill and border. We tessellate manually so fill and
//! border share the exact same vertices.

use glam::Vec2;
use macroquad::prelude::{draw_line, draw_triangle, Color};

use crate::input::Input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleButtonEvent {
    None,
    Hovered,
    Held,
    Clicked,
}

/// A circular button.
#[derive(Debug, Clone)]
pub struct CircleButton {
    pub center: Vec2,
    pub radius: f32,
    pub enabled: bool,
}

impl CircleButton {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self {
            center,
            radius,
            enabled: true,
        }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        (p - self.center).length_squared() <= self.radius * self.radius
    }

    /// Evaluate the button against this frame's input.
    pub fn update(&self, input: &Input) -> CircleButtonEvent {
        if !self.enabled {
            return CircleButtonEvent::None;
        }
        if !self.contains(input.mouse_pos) {
            return CircleButtonEvent::None;
        }
        if input.mouse_left_pressed {
            return CircleButtonEvent::Hovered;
        }
        if input.mouse_left_down {
            return CircleButtonEvent::Held;
        }
        if input.mouse_left_released {
            return CircleButtonEvent::Clicked;
        }
        CircleButtonEvent::Hovered
    }

    /// Draw the button. `lit` = true for the bright state, false for dim.
    ///
    /// Fill is a triangle fan from the center; border is a closed polyline
    /// through the same vertices. Both use `circle_segments(radius)` so
    /// they line up perfectly.
    pub fn draw(&self, color_lit: Color, color_dim: Color, lit: bool) {
        let color = if lit { color_lit } else { color_dim };
        let segments = circle_segments(self.radius);
        let step = std::f32::consts::TAU / segments as f32;
        let center = self.center;

        // Fill: fan of triangles.
        for i in 0..segments {
            let a0 = i as f32 * step;
            let a1 = (i + 1) as f32 * step;
            let p0 = center + Vec2::new(a0.cos(), a0.sin()) * self.radius;
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * self.radius;
            draw_triangle(center, p0, p1, color);
        }

        // Border: closed polyline through the same vertices.
        let border_color = Color::new(1.0, 1.0, 1.0, 0.15);
        for i in 0..segments {
            let a0 = i as f32 * step;
            let a1 = (i + 1) as f32 * step;
            let p0 = center + Vec2::new(a0.cos(), a0.sin()) * self.radius;
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * self.radius;
            draw_line(p0.x, p0.y, p1.x, p1.y, 1.5, border_color);
        }
    }
}

/// Number of segments for a smooth circle. Scales with radius so small
/// circles aren't over-tessellated and large ones don't look polygonal.
///
/// For R=100: circumference ≈ 628 px → ~157 segments → clamped to 128.
/// For R=20: circumference ≈ 126 px → ~31 segments → clamped to 32.
fn circle_segments(radius: f32) -> u32 {
    let circumference = 2.0 * std::f32::consts::PI * radius;
    ((circumference / 4.0) as u32).clamp(32, 128)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btn() -> CircleButton {
        CircleButton::new(Vec2::new(100.0, 100.0), 50.0)
    }

    #[test]
    fn test_contains_center() {
        assert!(btn().contains(Vec2::new(100.0, 100.0)));
    }

    #[test]
    fn test_contains_inside() {
        assert!(btn().contains(Vec2::new(130.0, 100.0)));
    }

    #[test]
    fn test_contains_edge() {
        assert!(btn().contains(Vec2::new(150.0, 100.0)));
    }

    #[test]
    fn test_contains_outside() {
        assert!(!btn().contains(Vec2::new(200.0, 100.0)));
    }

    #[test]
    fn test_no_event_outside() {
        let input = Input {
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_left_pressed: true,
            ..Default::default()
        };
        assert_eq!(btn().update(&input), CircleButtonEvent::None);
    }

    #[test]
    fn test_hover_only() {
        let input = Input {
            mouse_pos: Vec2::new(100.0, 100.0),
            ..Default::default()
        };
        assert_eq!(btn().update(&input), CircleButtonEvent::Hovered);
    }

    #[test]
    fn test_held() {
        let input = Input {
            mouse_pos: Vec2::new(100.0, 100.0),
            mouse_left_down: true,
            ..Default::default()
        };
        assert_eq!(btn().update(&input), CircleButtonEvent::Held);
    }

    #[test]
    fn test_clicked_on_release() {
        let input = Input {
            mouse_pos: Vec2::new(100.0, 100.0),
            mouse_left_released: true,
            ..Default::default()
        };
        assert_eq!(btn().update(&input), CircleButtonEvent::Clicked);
    }

    #[test]
    fn test_disabled() {
        let mut b = btn();
        b.enabled = false;
        let input = Input {
            mouse_pos: Vec2::new(100.0, 100.0),
            mouse_left_released: true,
            ..Default::default()
        };
        assert_eq!(b.update(&input), CircleButtonEvent::None);
    }

    #[test]
    fn test_circle_segments_scales() {
        // Small circle: hits the floor.
        assert_eq!(circle_segments(5.0), 32);
        // Large circle: hits the ceiling.
        assert_eq!(circle_segments(1000.0), 128);
        // Mid-size: proportional.
        let s = circle_segments(100.0);
        assert!(s >= 32 && s <= 128);
    }
}