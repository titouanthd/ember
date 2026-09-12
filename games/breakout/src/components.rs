// components.rs
use ember_stdlib::{Collider, Shape, Sprite, Transform};
use glam::Vec2;
use macroquad::prelude::Color;

// ---- Paddle ----

#[derive(Debug, Clone)]
pub struct Paddle {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub speed: f32,
}

impl Paddle {
    pub fn new(x: f32, y: f32, width: f32, height: f32, speed: f32, color: Color) -> Self {
        let transform = Transform::new(Vec2::new(x, y), 0.0, Vec2::new(width, height));
        let sprite = Sprite::new(color);
        // position == top-left. Collider size matches transform.scale.
        let collider = Collider::new(Shape::Aabb {
            half_size: Vec2::new(width / 2.0, height / 2.0),
        });
        Self { transform, sprite, collider, speed }
    }

    /// Center of the paddle in world space.
    pub fn center(&self) -> Vec2 {
        self.transform.position + self.transform.scale / 2.0
    }

    pub fn move_left(&mut self, dt: f32) {
        self.transform.position.x -= self.speed * dt;
        if self.transform.position.x < 0.0 {
            self.transform.position.x = 0.0;
        }
    }

    pub fn move_right(&mut self, dt: f32, screen_w: f32) {
        self.transform.position.x += self.speed * dt;
        let max_x = screen_w - self.transform.scale.x;
        if self.transform.position.x > max_x {
            self.transform.position.x = max_x;
        }
    }
}

// ---- Ball ----

#[derive(Debug, Clone)]
pub struct Ball {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub vx: f32,
    pub vy: f32,
}

impl Ball {
    pub fn new(x: f32, y: f32, size: f32, speed: f32, color: Color) -> Self {
        let transform = Transform::new(Vec2::new(x, y), 0.0, Vec2::new(size, size));
        let sprite = Sprite::new(color);
        let collider = Collider::new(Shape::Aabb {
            half_size: Vec2::new(size / 2.0, size / 2.0),
        });
        Self { transform, sprite, collider, vx: speed, vy: -speed }
    }

    pub fn size(&self) -> f32 { self.transform.scale.x }

    pub fn center(&self) -> Vec2 {
        self.transform.position + self.transform.scale / 2.0
    }

    pub fn bounce_x(&mut self) { self.vx = -self.vx; }
    pub fn bounce_y(&mut self) { self.vy = -self.vy; }

    pub fn update(&mut self, dt: f32) {
        self.transform.position.x += self.vx * dt;
        self.transform.position.y += self.vy * dt;
    }
}

// ---- Brick ----

#[derive(Debug, Clone)]
pub struct Brick {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub health: i32,
}

impl Brick {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: Color, health: i32) -> Self {
        let transform = Transform::new(Vec2::new(x, y), 0.0, Vec2::new(width, height));
        let sprite = Sprite::new(color);
        let collider = Collider::new(Shape::Aabb {
            half_size: Vec2::new(width / 2.0, height / 2.0),
        });
        Self { transform, sprite, collider, health }
    }

    pub fn center(&self) -> Vec2 {
        self.transform.position + self.transform.scale / 2.0
    }

    pub fn hit(&mut self) {
        self.health -= 1;
        if self.health <= 0 {
            self.sprite.hide();
            self.collider.set_active(false);
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paddle_move_left_stays_within_bounds() {
        let mut paddle = Paddle::new(50.0, 100.0, 80.0, 20.0, 200.0, Color::new(1.0, 1.0, 1.0, 1.0));
        paddle.move_left(0.5);
        assert_eq!(paddle.transform.position.x, 0.0);
    }

    #[test]
    fn test_paddle_move_right_stays_within_bounds() {
        let screen_w = 800.0;
        let mut paddle = Paddle::new(750.0, 100.0, 80.0, 20.0, 200.0, Color::new(1.0, 1.0, 1.0, 1.0));
        paddle.move_right(0.5, screen_w);
        assert_eq!(paddle.transform.position.x, screen_w - paddle.transform.scale.x);
    }

    #[test]
    fn test_ball_bounce_x_flips_vx() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ball.vx = 10.0;
        ball.bounce_x();
        assert_eq!(ball.vx, -10.0);
    }

    #[test]
    fn test_ball_bounce_y_flips_vy() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ball.vy = 7.0;
        ball.bounce_y();
        assert_eq!(ball.vy, -7.0);
    }

    #[test]
    fn test_ball_update_moves_position() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ball.vx = 100.0;
        ball.vy = 50.0;
        ball.update(0.1);
        assert_eq!(ball.transform.position.x, 10.0);
        assert_eq!(ball.transform.position.y, 5.0);
    }

    #[test]
    fn test_ball_center() {
        let ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(ball.center(), Vec2::new(5.0, 5.0));
    }

    #[test]
    fn test_brick_hit_reduces_health() {
        let mut brick = Brick::new(0.0, 0.0, 40.0, 20.0, Color::new(1.0, 0.0, 0.0, 1.0), 2);
        assert_eq!(brick.health, 2);
        assert!(brick.is_alive());
        brick.hit();
        assert_eq!(brick.health, 1);
        assert!(brick.is_alive());
        brick.hit();
        assert_eq!(brick.health, 0);
        assert!(!brick.is_alive());
        assert!(!brick.sprite.visible);
        assert!(!brick.collider.active);
    }
}