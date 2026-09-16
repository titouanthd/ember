use breakout::components::Brick;

use breakout::GameState;
use breakout::systems::{self, BreakoutWorld, GameContext};
use macroquad::prelude::Color;

#[test]
fn test_brick_destroyed_on_ball_collision() {
    let ctx = GameContext::default();

    let bricks = vec![Brick::new(
        ctx.screen_w / 2.0 - ctx.brick_width / 2.0,
        100.0,
        ctx.brick_width,
        ctx.brick_height,
        Color::new(1.0, 0.0, 0.0, 1.0),
        1,
    )];

    let mut world = BreakoutWorld::new(bricks, &ctx, 3);
    world.state = GameState::Playing;

    // Place la balle juste en dessous de la brique, remontant vers le haut
    world.ball.transform.position.x = ctx.screen_w / 2.0 - ctx.ball_size / 2.0;
    world.ball.transform.position.y = 100.0 + ctx.brick_height / 2.0 + ctx.ball_size / 2.0 + 5.0;
    world.ball.vy = -100.0; // vers le haut
    world.ball.vx = 0.0;

    assert!(world.bricks[0].is_alive());

    // Simule un petit delta pour que la balle entre en collision
    let dt = 0.05;
    systems::update(&mut world, &ctx, dt);

    assert!(!world.bricks[0].is_alive());
    assert_eq!(world.score, 1);
}
