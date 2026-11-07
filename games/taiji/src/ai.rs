// games/taiji/src/ai.rs
use crate::components::{QiBall, Paddle};

pub struct AiController {
    pub target_y: f32,
}

impl AiController {
    pub fn new(screen_h: f32) -> Self {
        Self {
            target_y: screen_h / 2.0,
        }
    }

    pub fn update(
        &mut self,
        paddle: &mut Paddle,
        ball: &QiBall,
        dt: f32,
        screen_h: f32,
        ball_radius: f32,
    ) {
        self.target_y = predict_ball_y(ball, paddle.transform.position.x, screen_h, ball_radius);

        let center = paddle.transform.center().y;
        let dead_zone = 10.0;
        if center < self.target_y - dead_zone {
            paddle.move_down(dt, screen_h);
        } else if center > self.target_y + dead_zone {
            paddle.move_up(dt);
        }
    }

    pub fn reset(&mut self, screen_h: f32) {
        self.target_y = screen_h / 2.0;
    }
}

pub fn predict_ball_y(ball: &QiBall, target_x: f32, screen_h: f32, ball_radius: f32) -> f32 {
    if ball.velocity.x.abs() < 0.01 {
        return ball.transform.center().y;
    }
    let dx = target_x - ball.transform.position.x;
    let t = dx / ball.velocity.x;
    if t <= 0.0 {
        return ball.transform.center().y;
    }
    let y = ball.transform.center().y + ball.velocity.y * t;
    let min_y = ball_radius;
    let max_y = screen_h - ball_radius;
    let period = max_y - min_y;
    if period <= 1.0 {
        return y.clamp(min_y, max_y);
    }
    let mut y_rel = (y - min_y).rem_euclid(2.0 * period);
    if y_rel > period {
        y_rel = 2.0 * period - y_rel;
    }
    y_rel + min_y
}