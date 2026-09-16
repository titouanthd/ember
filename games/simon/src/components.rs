//! Simon components: color enum + layout helper for the 4 buttons.

use glam::Vec2;

use crate::config::GameContext;

/// The 4 Simon colors, in the historical clockwise order starting top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimonColor {
    Green,  // top-left
    Red,    // top-right
    Yellow, // bottom-left
    Blue,   // bottom-right
}

impl SimonColor {
    pub const ALL: [SimonColor; 4] = [
        SimonColor::Green,
        SimonColor::Red,
        SimonColor::Yellow,
        SimonColor::Blue,
    ];

    pub fn index(self) -> usize {
        match self {
            SimonColor::Green => 0,
            SimonColor::Red => 1,
            SimonColor::Yellow => 2,
            SimonColor::Blue => 3,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Self::ALL.get(i).copied()
    }
}

/// Compute the 4 button centers, in SimonColor::ALL order.
pub fn button_centers(ctx: &GameContext) -> [Vec2; 4] {
    let center = ctx.playfield_center();
    let r = ctx.button_radius;
    let half_gap = r + ctx.button_gap * 0.5;

    [
        center + Vec2::new(-half_gap, -half_gap), // Green
        center + Vec2::new(half_gap, -half_gap),  // Red
        center + Vec2::new(-half_gap, half_gap),  // Yellow
        center + Vec2::new(half_gap, half_gap),   // Blue
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_index_roundtrip() {
        for (i, c) in SimonColor::ALL.iter().enumerate() {
            assert_eq!(c.index(), i);
            assert_eq!(SimonColor::from_index(i), Some(*c));
        }
    }

    #[test]
    fn test_from_index_out_of_bounds() {
        assert_eq!(SimonColor::from_index(4), None);
    }

    #[test]
    fn test_button_centers_are_distinct() {
        let ctx = crate::config::load_config();
        let centers = button_centers(&ctx);
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert!(centers[i] != centers[j]);
            }
        }
    }

    #[test]
    fn test_button_centers_are_centered_around_playfield() {
        let ctx = crate::config::load_config();
        let centers = button_centers(&ctx);
        let avg_x: f32 = centers.iter().map(|c| c.x).sum::<f32>() / 4.0;
        let avg_y: f32 = centers.iter().map(|c| c.y).sum::<f32>() / 4.0;
        let expected = ctx.playfield_center();
        assert!((avg_x - expected.x).abs() < 0.01);
        assert!((avg_y - expected.y).abs() < 0.01);
    }
}
