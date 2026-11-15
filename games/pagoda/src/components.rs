// games/pagoda/src/components.rs
use ember_stdlib::Transform;
use glam::Vec2;
use macroquad::prelude::Color;

/// A standard block in the tower.
#[derive(Debug, Clone)]
pub struct Block {
    pub transform: Transform,
    pub color: Color,
}

impl Block {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: Color) -> Self {
        Self {
            transform: Transform::new(Vec2::new(x, y), 0.0, Vec2::new(w, h)),
            color,
        }
    }
}

/// The block that is currently sliding back and forth at the top.
#[derive(Debug, Clone)]
pub struct SlidingBlock {
    pub x: f32,
    pub width: f32,
    pub direction: f32, // 1.0 or -1.0
    pub speed: f32,
    pub color: Color,
}
