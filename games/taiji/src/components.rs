// games/taiji/src/components.rs
use ember_core::math::Aabb;
use ember_stdlib::Transform;
use glam::Vec2;
use macroquad::prelude::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Player,
    Ai,
}

#[derive(Debug, Clone)]
pub struct Paddle {
    pub transform: Transform,
    pub color: Color,
    pub speed: f32,
    pub velocity_y: f32, // Pour l'effet "English"
}

impl Paddle {
    pub fn new(x: f32, y: f32, w: f32, h: f32, speed: f32, color: Color) -> Self {
        Self {
            transform: Transform::new(Vec2::new(x, y), 0.0, Vec2::new(w, h)),
            color,
            speed,
            velocity_y: 0.0,
        }
    }

    pub fn reset_velocity(&mut self) {
        self.velocity_y = 0.0;
    }

    pub fn move_up(&mut self, dt: f32) {
        if dt <= 0.0 { return; }
        let new_y = (self.transform.position.y - self.speed * dt).max(0.0);
        self.velocity_y = (new_y - self.transform.position.y) / dt;
        self.transform.position.y = new_y;
    }

    pub fn move_down(&mut self, dt: f32, screen_h: f32) {
        if dt <= 0.0 { return; }
        let max_y = screen_h - self.transform.scale.y;
        let new_y = (self.transform.position.y + self.speed * dt).min(max_y);
        self.velocity_y = (new_y - self.transform.position.y) / dt;
        self.transform.position.y = new_y;
    }

    pub fn center(&self) -> Vec2 {
        self.transform.center()
    }

    pub fn aabb(&self) -> Aabb {
        self.transform.aabb()
    }
}

#[derive(Debug, Clone)]
pub struct QiBall {
    pub transform: Transform,
    pub velocity: Vec2,
    pub color: Color,
}

impl QiBall {
    pub fn new(x: f32, y: f32, radius: f32, color: Color) -> Self {
        Self {
            transform: Transform::new(Vec2::new(x, y), 0.0, Vec2::splat(radius * 2.0)),
            velocity: Vec2::ZERO,
            color,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.transform.position += self.velocity * dt;
    }

    pub fn aabb(&self) -> Aabb {
        self.transform.aabb()
    }
}