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

    /// Clamp the ball's speed to `max` (preserving direction).
    ///
    /// Guards against runaway acceleration from repeated paddle hits.
    /// Breakout has the same pattern locally; we keep them separate until
    /// a third game needs it (Rule of Three).
    pub fn clamp_speed(&mut self, max: f32) {
        let speed_sq = self.vx * self.vx + self.vy * self.vy;
        if speed_sq > max * max {
            let k = max / speed_sq.sqrt();
            self.vx *= k;
            self.vy *= k;
        }
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
        let mut paddle = Paddle::new(
            0.0,
            500.0,
            10.0,
            50.0,
            100.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        let dt = 0.5;
        let screen_h = 600.0;
        paddle.move_down(dt, screen_h);
        assert_eq!(paddle.transform.position.y, 550.0);
    }

    #[test]
    fn test_ball_clamp_speed_caps_magnitude() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ball.vx = 10000.0;
        ball.vy = 10000.0;
        ball.clamp_speed(1000.0);
        let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt();
        assert!((speed - 1000.0).abs() < 1e-3);
    }

    #[test]
    fn test_ball_clamp_speed_below_max_is_noop() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, 5.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ball.vx = 100.0;
        ball.vy = 100.0;
        let (vx0, vy0) = (ball.vx, ball.vy);
        ball.clamp_speed(1000.0);
        assert_eq!(ball.vx, vx0);
        assert_eq!(ball.vy, vy0);
    }
}
