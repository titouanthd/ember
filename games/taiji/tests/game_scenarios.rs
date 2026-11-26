//! Tàijí's match rules, without a window (first tests for this game).

use ember_input::{Input, Key};
use glam::Vec2;
use taiji::GameContext;
use taiji::ai::predict_ball_y;
use taiji::systems::{self, BASE_SPEED, MatchState, ScoreEvent};

const DT: f32 = 1.0 / 60.0;

fn holding(keys: &[Key]) -> Input {
    Input {
        keys_down: keys.to_vec(),
        ..Input::default()
    }
}

#[test]
fn a_round_serves_toward_the_ai_at_base_speed() {
    let ctx = GameContext::default();
    let m = MatchState::new(&ctx);
    assert_eq!(m.ball.transform.position, Vec2::new(400.0, 300.0));
    assert_eq!(m.ball.velocity, Vec2::new(BASE_SPEED, 0.0));
    assert_eq!(m.time_scale, 1.0);
}

#[test]
fn holding_space_slows_time_and_releasing_bursts_it() {
    let ctx = GameContext::default();
    let mut m = MatchState::new(&ctx);
    for _ in 0..30 {
        systems::update(&mut m, &ctx, DT, &holding(&[Key::Space]));
    }
    assert!(m.time_scale < 0.3, "Yin: {}", m.time_scale);
    assert!(m.was_yin);

    systems::update(&mut m, &ctx, DT, &Input::default());
    assert!(!m.was_yin);
    assert!(m.yang_burst_timer > 0.0);
    for _ in 0..10 {
        systems::update(&mut m, &ctx, DT, &Input::default());
    }
    assert!(m.time_scale > 1.5, "Yang: {}", m.time_scale);
}

#[test]
fn the_ball_bounces_off_the_top_wall() {
    let ctx = GameContext::default();
    let mut m = MatchState::new(&ctx);
    m.ball.transform.position = Vec2::new(400.0, 1.0);
    m.ball.velocity = Vec2::new(0.0, -300.0);
    systems::update(&mut m, &ctx, DT, &Input::default());
    assert_eq!(m.ball.transform.position.y, 0.0);
    assert!(m.ball.velocity.y > 0.0);
}

#[test]
fn a_ball_past_a_side_scores() {
    let ctx = GameContext::default();
    let mut m = MatchState::new(&ctx);
    m.ball.transform.position = Vec2::new(ctx.screen_w + 1.0, 300.0);
    assert_eq!(
        systems::update(&mut m, &ctx, DT, &Input::default()),
        Some(ScoreEvent::PlayerScored)
    );
    m.reset_round(&ctx, -1.0);
    m.ball.transform.position = Vec2::new(-100.0, 300.0);
    assert_eq!(
        systems::update(&mut m, &ctx, DT, &Input::default()),
        Some(ScoreEvent::AiScored)
    );
}

#[test]
fn the_player_paddle_returns_the_ball_faster() {
    let ctx = GameContext::default();
    let mut m = MatchState::new(&ctx);
    let paddle = m.player.transform.position;
    m.ball.transform.position = Vec2::new(paddle.x + 5.0, paddle.y + ctx.paddle_h / 2.0);
    m.ball.velocity = Vec2::new(-BASE_SPEED, 0.0);
    systems::update(&mut m, &ctx, DT, &Input::default());
    assert!(m.ball.velocity.x > 0.0);
    assert!(m.ball.velocity.length() > BASE_SPEED);
}

#[test]
fn the_player_moves_and_stays_on_the_court() {
    let ctx = GameContext::default();
    let mut m = MatchState::new(&ctx);
    let y0 = m.player.transform.position.y;
    systems::update(&mut m, &ctx, DT, &holding(&[Key::W]));
    assert!(m.player.transform.position.y < y0);
    for _ in 0..200 {
        systems::update(&mut m, &ctx, DT, &holding(&[Key::Down]));
    }
    assert_eq!(m.player.transform.position.y, ctx.screen_h - ctx.paddle_h);
}

#[test]
fn the_ai_predicts_a_bounce_off_the_wall() {
    let ctx = GameContext::default();
    let mut m = MatchState::new(&ctx);
    m.ball.transform.position = Vec2::new(400.0, 300.0);
    m.ball.velocity = Vec2::new(400.0, -400.0);
    let y = predict_ball_y(&m.ball, 740.0, ctx.screen_h, ctx.ball_radius);
    assert!((ctx.ball_radius..=ctx.screen_h - ctx.ball_radius).contains(&y));
    // Moving up 340 px from y≈312 reflects off the top wall.
    assert!(y > ctx.ball_radius && y < 312.0, "{y}");
}
