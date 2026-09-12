use breakout::components::{Ball, Paddle, Brick};
use breakout::systems::{self, GameContext};
use breakout::GameState;
use macroquad::prelude::Color;

#[test]
fn test_brick_destroyed_on_ball_collision() {
    let ctx = GameContext::default();
    let mut paddle = Paddle::new(
        ctx.screen_w / 2.0 - ctx.paddle_width / 2.0,
        ctx.screen_h - 50.0,
        ctx.paddle_width,
        ctx.paddle_height,
        300.0,
        ctx.paddle_color,
    );
    let mut ball = Ball::new(
        ctx.screen_w / 2.0 - ctx.ball_size / 2.0,
        ctx.screen_h / 2.0,
        ctx.ball_size,
        ctx.ball_speed,
        ctx.ball_color,
    );
    let mut bricks = vec![
        Brick::new(
            ctx.screen_w / 2.0 - ctx.brick_width / 2.0,
            100.0,
            ctx.brick_width,
            ctx.brick_height,
            Color::new(1.0, 0.0, 0.0, 1.0),
            1,
        ),
    ];

    let mut score = 0;
    let mut state = GameState::Playing;

    // Place la balle juste en dessous de la brique, remontant vers le haut
    ball.transform.position.x = ctx.screen_w / 2.0 - ctx.ball_size / 2.0;
    ball.transform.position.y = 100.0 + ctx.brick_height / 2.0 + ctx.ball_size / 2.0 + 5.0;
    ball.vy = -100.0; // vers le haut
    ball.vx = 0.0;

    assert!(bricks[0].is_alive());

    // Simule un petit delta pour que la balle entre en collision
    let dt = 0.05;
    systems::update(&mut paddle, &mut ball, &mut bricks, &mut score, &mut state, &ctx, dt);

    assert!(!bricks[0].is_alive());
    assert_eq!(score, 1);
}