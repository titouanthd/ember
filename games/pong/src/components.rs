// games/pong/src/components.rs
use ember_stdlib::{Collider, Shape, Sprite, Transform};
use glam::Vec2;
use macroquad::prelude::{Color, Rect};

#[derive(Debug, Clone)]
pub struct Paddle {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub speed: f32,
}

impl Paddle {
    pub fn new(x: f32, y: f32, w: f32, h: f32, speed: f32, color: Color) -> Self {
        let transform = Transform::new(Vec2::new(x, y), 0.0, Vec2::new(w, h));
        let sprite = Sprite::new(color);
        let collider = Collider::new(Shape::Aabb {
            half_size: Vec2::new(w / 2.0, h / 2.0),
        });
        Self {
            transform,
            sprite,
            collider,
            speed,
        }
    }

    /// Centre du paddle en coordonnées monde.
    pub fn center(&self) -> Vec2 {
        self.transform.position + self.transform.scale / 2.0
    }

    pub fn rect(&self) -> Rect {
        Rect::new(
            self.transform.position.x,
            self.transform.position.y,
            self.transform.scale.x,
            self.transform.scale.y,
        )
    }

    pub fn move_up(&mut self, dt: f32) {
        self.transform.position.y -= self.speed * dt;
        if self.transform.position.y < 0.0 {
            self.transform.position.y = 0.0;
        }
    }

    pub fn move_down(&mut self, dt: f32, screen_h: f32) {
        self.transform.position.y += self.speed * dt;
        if self.transform.position.y + self.transform.scale.y > screen_h {
            self.transform.position.y = screen_h - self.transform.scale.y;
        }
    }
}

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
        Self {
            transform,
            sprite,
            collider,
            vx: speed,
            vy: speed,
        }
    }

    /// Centre de la balle en coordonnées monde.
    pub fn center(&self) -> Vec2 {
        self.transform.position + self.transform.scale / 2.0
    }

    pub fn rect(&self) -> Rect {
        Rect::new(
            self.transform.position.x,
            self.transform.position.y,
            self.transform.scale.x,
            self.transform.scale.y,
        )
    }

    pub fn bounce_x(&mut self) {
        self.vx = -self.vx;
    }

    pub fn bounce_y(&mut self) {
        self.vy = -self.vy;
    }

    pub fn update(&mut self, dt: f32) {
        self.transform.position.x += self.vx * dt;
        self.transform.position.y += self.vy * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_ball_update_moves_correctly() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ball.vx = 100.0;
        ball.vy = 50.0;
        let dt = 0.1;
        ball.update(dt);
        assert_eq!(ball.transform.position.x, 10.0);
        assert_eq!(ball.transform.position.y, 5.0);
    }

    #[test]
    fn test_paddle_move_up_does_not_go_below_zero() {
        let mut paddle = Paddle::new(0.0, 10.0, 10.0, 50.0, 100.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let dt = 0.5;
        paddle.move_up(dt);
        assert_eq!(paddle.transform.position.y, 0.0);
    }

    #[test]
    fn test_paddle_move_down_does_not_go_below_screen() {
        let mut paddle = Paddle::new(0.0, 500.0, 10.0, 50.0, 100.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let dt = 0.5;
        let screen_h = 600.0;
        paddle.move_down(dt, screen_h);
        assert_eq!(paddle.transform.position.y, 550.0);
    }

    #[test]
    fn test_paddle_center() {
        let paddle = Paddle::new(100.0, 200.0, 20.0, 80.0, 100.0, Color::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(paddle.center(), Vec2::new(110.0, 240.0));
    }

    #[test]
    fn test_ball_center() {
        let ball = Ball::new(100.0, 200.0, 20.0, 100.0, Color::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(ball.center(), Vec2::new(110.0, 210.0));
    }
}