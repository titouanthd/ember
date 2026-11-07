// games/pong/src/components.rs
use ember_core::math::Aabb;
use ember_stdlib::{Sprite, Transform};
use glam::Vec2;
use macroquad::prelude::Color;

/// Raquette : rectangle qui bouge verticalement.
#[derive(Debug, Clone)]
pub struct Paddle {
    pub transform: Transform,
    pub sprite: Sprite,
    pub speed: f32,
    /// Vélocité Y effective **de la frame courante**, en px/s.
    /// Doit être remise à 0 en début de frame (`reset_velocity`) puis
    /// mise à jour par `move_up` / `move_down`. Sert à l'effet "english"
    /// (une raquette en mouvement transfère de la vitesse à la balle).
    pub velocity_y: f32,
}

impl Paddle {
    pub fn new(x: f32, y: f32, w: f32, h: f32, speed: f32, color: Color) -> Self {
        Self {
            transform: Transform::new(Vec2::new(x, y), 0.0, Vec2::new(w, h)),
            sprite: Sprite::new(color),
            speed,
            velocity_y: 0.0,
        }
    }

    /// AABB en coordonnées monde (convention top-left).
    pub fn aabb(&self) -> Aabb {
        self.transform.aabb()
    }

    /// Remet la vélocité à 0. À appeler en début de frame, **avant**
    /// `move_up` / `move_down`. Sans ça, une raquette immobile garderait
    /// la vélocité de la frame précédente et transférerait de l'énergie
    /// fantôme à la balle.
    pub fn reset_velocity(&mut self) {
        self.velocity_y = 0.0;
    }

    pub fn move_up(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let new_y = (self.transform.position.y - self.speed * dt).max(0.0);
        self.velocity_y = (new_y - self.transform.position.y) / dt;
        self.transform.position.y = new_y;
    }

    pub fn move_down(&mut self, dt: f32, screen_h: f32) {
        if dt <= 0.0 {
            return;
        }
        let max_y = screen_h - self.transform.scale.y;
        let new_y = (self.transform.position.y + self.speed * dt).min(max_y);
        self.velocity_y = (new_y - self.transform.position.y) / dt;
        self.transform.position.y = new_y;
    }
}

/// Balle : carré avec une vélocité (vecteur, pas de vx/vy séparés).
#[derive(Debug, Clone)]
pub struct Ball {
    pub transform: Transform,
    pub sprite: Sprite,
    /// Vélocité en pixels par seconde. `Vec2::ZERO` = immobile.
    pub velocity: Vec2,
}

impl Ball {
    pub fn new(x: f32, y: f32, size: f32, velocity: Vec2, color: Color) -> Self {
        Self {
            transform: Transform::new(Vec2::new(x, y), 0.0, Vec2::splat(size)),
            sprite: Sprite::new(color),
            velocity,
        }
    }

    pub fn aabb(&self) -> Aabb {
        self.transform.aabb()
    }

    pub fn update(&mut self, dt: f32) {
        self.transform.position += self.velocity * dt;
    }

    pub fn bounce_x(&mut self) {
        self.velocity.x = -self.velocity.x;
    }

    pub fn bounce_y(&mut self) {
        self.velocity.y = -self.velocity.y;
    }

    /// Clampe la **norme** de la vélocité à `max`, direction préservée.
    pub fn clamp_speed(&mut self, max: f32) {
        self.velocity = self.velocity.clamp_length_max(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

    #[test]
    fn test_ball_bounce_x_flips_vx() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, Vec2::new(10.0, 0.0), WHITE);
        ball.bounce_x();
        assert_eq!(ball.velocity.x, -10.0);
        assert_eq!(ball.velocity.y, 0.0);
    }

    #[test]
    fn test_ball_bounce_y_flips_vy() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, Vec2::new(0.0, 7.0), WHITE);
        ball.bounce_y();
        assert_eq!(ball.velocity.y, -7.0);
        assert_eq!(ball.velocity.x, 0.0);
    }

    #[test]
    fn test_ball_update_moves_correctly() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, Vec2::new(100.0, 50.0), WHITE);
        ball.update(0.1);
        assert_eq!(ball.transform.position.x, 10.0);
        assert_eq!(ball.transform.position.y, 5.0);
    }

    #[test]
    fn test_paddle_move_up_clamps_at_zero() {
        let mut paddle = Paddle::new(0.0, 10.0, 10.0, 50.0, 100.0, WHITE);
        paddle.move_up(0.5);
        assert_eq!(paddle.transform.position.y, 0.0);
    }

    #[test]
    fn test_paddle_move_down_clamps_at_screen() {
        let mut paddle = Paddle::new(0.0, 500.0, 10.0, 50.0, 100.0, WHITE);
        paddle.move_down(0.5, 600.0);
        assert_eq!(paddle.transform.position.y, 550.0);
    }

    #[test]
    fn test_ball_clamp_speed_caps_magnitude() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, Vec2::new(10000.0, 10000.0), WHITE);
        ball.clamp_speed(1000.0);
        let speed = ball.velocity.length();
        assert!((speed - 1000.0).abs() < 1e-3);
    }

    #[test]
    fn test_ball_clamp_speed_below_max_is_noop() {
        let mut ball = Ball::new(0.0, 0.0, 10.0, Vec2::new(100.0, 100.0), WHITE);
        let (vx0, vy0) = (ball.velocity.x, ball.velocity.y);
        ball.clamp_speed(1000.0);
        assert_eq!(ball.velocity.x, vx0);
        assert_eq!(ball.velocity.y, vy0);
    }

    #[test]
    fn test_paddle_aabb_top_left_convention() {
        let paddle = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, WHITE);
        let aabb = paddle.aabb();
        assert_eq!(aabb.min, Vec2::new(30.0, 100.0));
        assert_eq!(aabb.max, Vec2::new(45.0, 180.0));
    }

    #[test]
    fn test_ball_aabb_top_left_convention() {
        let ball = Ball::new(10.0, 20.0, 20.0, Vec2::ZERO, WHITE);
        let aabb = ball.aabb();
        assert_eq!(aabb.min, Vec2::new(10.0, 20.0));
        assert_eq!(aabb.max, Vec2::new(30.0, 40.0));
    }

    // -----------------------------------------------------------------------
    // Vélocité de raquette (english)
    // -----------------------------------------------------------------------

    #[test]
    fn test_paddle_move_up_sets_negative_velocity() {
        let mut paddle = Paddle::new(0.0, 100.0, 10.0, 50.0, 200.0, WHITE);
        paddle.reset_velocity();
        paddle.move_up(0.1);
        // 200 px/s * 0.1s = 20 px de déplacement vers le haut.
        assert!((paddle.velocity_y + 200.0).abs() < 1e-3);
        assert!((paddle.transform.position.y - 80.0).abs() < 1e-3);
    }

    #[test]
    fn test_paddle_move_down_sets_positive_velocity() {
        let mut paddle = Paddle::new(0.0, 100.0, 10.0, 50.0, 200.0, WHITE);
        paddle.reset_velocity();
        paddle.move_down(0.1, 600.0);
        assert!((paddle.velocity_y - 200.0).abs() < 1e-3);
        assert!((paddle.transform.position.y - 120.0).abs() < 1e-3);
    }

    #[test]
    fn test_paddle_at_wall_has_zero_velocity() {
        // Paddle déjà collé en haut, on push encore vers le haut.
        let mut paddle = Paddle::new(0.0, 0.0, 10.0, 50.0, 200.0, WHITE);
        paddle.reset_velocity();
        paddle.move_up(0.1);
        assert_eq!(paddle.velocity_y, 0.0, "paddle pinned at wall has no velocity");
    }

    #[test]
    fn test_paddle_reset_clears_velocity() {
        let mut paddle = Paddle::new(0.0, 100.0, 10.0, 50.0, 200.0, WHITE);
        paddle.move_up(0.1);
        assert!(paddle.velocity_y != 0.0);
        paddle.reset_velocity();
        assert_eq!(paddle.velocity_y, 0.0);
    }

    #[test]
    fn test_paddle_move_with_zero_dt_is_noop() {
        let mut paddle = Paddle::new(0.0, 100.0, 10.0, 50.0, 200.0, WHITE);
        paddle.move_up(0.0);
        assert_eq!(paddle.transform.position.y, 100.0);
        assert_eq!(paddle.velocity_y, 0.0);
    }
}