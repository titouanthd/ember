// games/pong/tests/game_scenarios.rs
//!
//! Integration scenarios for Pong.
//!
//! Assertions behavior-focused : on vérifie direction, containment, et
//! transitions d'état plutôt que des positions f32 exactes.

use glam::Vec2;
use pong::GameState;
use pong::components::{Ball, Paddle};
use pong::systems::{self, GameContext, MatchState};

const EPS: f32 = 1e-3;
const SEED: u32 = 42;

// ============================================================================
// IA
// ============================================================================

#[test]
fn test_ai_moves_towards_ball() {
    let ctx = GameContext::default();
    let mut ai = Paddle::new(100.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let ball_y = 300.0;
    let dt = 0.1;
    let initial_y = ai.transform.position.y;

    systems::ai_move(&mut ai, ball_y, dt, ctx.screen_h);

    assert!(ai.transform.position.y > initial_y);
}

// ============================================================================
// Scoring
// ============================================================================

#[test]
fn test_score_increments_when_ball_leaves_left() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;
    let dt = 0.0;

    match_state.ball.transform.position.x = -50.0;
    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert_eq!(match_state.score_ai, 1);
    assert_eq!(match_state.score_player, 0);

    let expected_x = ctx.screen_w / 2.0 - ctx.ball_size / 2.0;
    assert!(
        (match_state.ball.transform.position.x - expected_x).abs() < EPS,
        "ball should be near center, got {} (expected ~{})",
        match_state.ball.transform.position.x,
        expected_x,
    );
}

#[test]
fn test_score_increments_when_ball_leaves_right() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;
    let dt = 0.0;

    match_state.ball.transform.position.x = ctx.screen_w + 50.0;
    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert_eq!(match_state.score_player, 1);
    assert_eq!(match_state.score_ai, 0);

    let expected_x = ctx.screen_w / 2.0 - ctx.ball_size / 2.0;
    assert!(
        (match_state.ball.transform.position.x - expected_x).abs() < EPS,
        "ball should be near center, got {} (expected ~{})",
        match_state.ball.transform.position.x,
        expected_x,
    );
}

#[test]
fn test_player_wins_at_5_points() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;
    match_state.score_player = 4;
    let dt = 0.0;

    match_state.ball.transform.position.x = ctx.screen_w + 50.0;
    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert_eq!(match_state.score_player, 5);
    assert_eq!(match_state.state, GameState::Win);
}

#[test]
fn test_ai_wins_at_5_points() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;
    match_state.score_ai = 4;
    let dt = 0.0;

    match_state.ball.transform.position.x = -50.0;
    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert_eq!(match_state.score_ai, 5);
    assert_eq!(match_state.state, GameState::GameOver);
}

// ============================================================================
// Collisions raquettes (containment + signe)
// ============================================================================

#[test]
fn test_ball_collides_with_player_paddle() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(
        ctx.screen_w - 30.0 - 15.0,
        100.0,
        15.0,
        80.0,
        100.0,
        ctx.ai_color,
    );
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    let player_aabb = player.aabb();
    match_state.ball = Ball::new(
        player_aabb.max.x - 5.0,
        120.0,
        20.0,
        Vec2::new(-100.0, 0.0),
        ctx.ball_color,
    );
    let dt = 0.1;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert!(
        match_state.ball.velocity.x > 0.0,
        "ball should bounce off the player paddle (vx > 0), got {}",
        match_state.ball.velocity.x,
    );

    let ball_aabb = match_state.ball.aabb();
    let player_aabb = player.aabb();
    assert!(
        ball_aabb.min.x >= player_aabb.max.x - EPS,
        "ball (left={}) should be right of paddle (right={})",
        ball_aabb.min.x,
        player_aabb.max.x,
    );
}

#[test]
fn test_ball_collides_with_ai_paddle() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(
        ctx.screen_w - 30.0 - 15.0,
        100.0,
        15.0,
        80.0,
        100.0,
        ctx.ai_color,
    );
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    let ai_aabb = ai.aabb();
    match_state.ball = Ball::new(
        ai_aabb.min.x - 15.0,
        120.0,
        20.0,
        Vec2::new(100.0, 0.0),
        ctx.ball_color,
    );
    let dt = 0.1;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert!(
        match_state.ball.velocity.x < 0.0,
        "ball should bounce off the AI paddle (vx < 0), got {}",
        match_state.ball.velocity.x,
    );

    let ball_aabb = match_state.ball.aabb();
    let ai_aabb = ai.aabb();
    assert!(
        ball_aabb.min.x <= ai_aabb.min.x + EPS,
        "ball (left={}) should be left of AI paddle (left={})",
        ball_aabb.min.x,
        ai_aabb.min.x,
    );
}

// ============================================================================
// IA : re-target périodique (régression)
// ============================================================================

#[test]
fn test_ai_tracks_ball_after_reaction_delay() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(700.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    match_state.ball.transform.position.y = 500.0;
    match_state.ball.velocity = Vec2::ZERO;

    let initial_ai_y = ai.transform.position.y;
    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);
    }

    assert!(
        ai.transform.position.y > initial_ai_y + 1.0,
        "AI should move down toward ball at y=500 (was y={}, now y={})",
        initial_ai_y,
        ai.transform.position.y,
    );
}

// ============================================================================
// Réflexion miroir (raquette immobile = mur plat)
// ============================================================================

#[test]
fn test_stationary_paddle_preserves_angle() {
    // Balle à 45° (vx=-100, vy=50) qui touche un paddle immobile.
    // Réflexion miroir → vx s'inverse, vy est préservé (× increment).
    let ctx = GameContext::default();
    let mut player = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(700.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    match_state.ball = Ball::new(40.0, 130.0, 20.0, Vec2::new(-100.0, 50.0), ctx.ball_color);
    player.velocity_y = 0.0;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, 0.016);

    assert!(match_state.ball.velocity.x > 0.0);
    let expected_vy = 50.0 * ctx.ball_speed_increment;
    assert!(
        (match_state.ball.velocity.y - expected_vy).abs() < 1.0,
        "mirror should preserve vy (× increment), got vy={} (expected ~{})",
        match_state.ball.velocity.y,
        expected_vy,
    );
}

#[test]
fn test_impact_position_does_not_change_angle() {
    // Balle frappe le haut du paddle vs le bas, même vélocité incidente,
    // paddle immobile. La réflexion doit être identique : la position
    // d'impact n'influence pas l'angle (mur plat).
    let ctx = GameContext::default();

    let make_match = |ball_y: f32| {
        let mut ms = MatchState::new_with_seed(&ctx, SEED);
        ms.state = GameState::Playing;
        ms.ball = Ball::new(40.0, ball_y, 20.0, Vec2::new(-100.0, 0.0), ctx.ball_color);
        ms
    };

    let mut match_high = make_match(105.0); // bord haut du paddle
    let mut match_low = make_match(165.0); // bord bas du paddle

    let mut player_high = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut player_low = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(700.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);

    systems::update(&mut player_high, &mut ai, &mut match_high, &ctx, 0.016);
    systems::update(&mut player_low, &mut ai, &mut match_low, &ctx, 0.016);

    assert!(
        (match_high.ball.velocity - match_low.ball.velocity).length() < 1e-3,
        "impact position must not influence angle: high={:?}, low={:?}",
        match_high.ball.velocity,
        match_low.ball.velocity,
    );
}

// ============================================================================
// English : la vélocité du paddle influence la balle
// ============================================================================

#[test]
fn test_paddle_moving_down_sends_ball_down_on_center_hit() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(700.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    match_state.ball = Ball::new(40.0, 130.0, 20.0, Vec2::new(-100.0, 0.0), ctx.ball_color);
    player.velocity_y = 300.0;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, 0.016);

    assert!(
        match_state.ball.velocity.y > 30.0,
        "moving-down paddle should impart downward english, got vy={}",
        match_state.ball.velocity.y,
    );
}

#[test]
fn test_paddle_moving_up_sends_ball_up_on_center_hit() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(700.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    match_state.ball = Ball::new(40.0, 130.0, 20.0, Vec2::new(-100.0, 0.0), ctx.ball_color);
    player.velocity_y = -300.0;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, 0.016);

    assert!(
        match_state.ball.velocity.y < -30.0,
        "moving-up paddle should impart upward english, got vy={}",
        match_state.ball.velocity.y,
    );
}

#[test]
fn test_stationary_paddle_has_no_english() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(30.0, 100.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(700.0, 100.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new_with_seed(&ctx, SEED);
    match_state.state = GameState::Playing;

    match_state.ball = Ball::new(40.0, 130.0, 20.0, Vec2::new(-100.0, 0.0), ctx.ball_color);
    player.velocity_y = 0.0;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, 0.016);

    assert!(match_state.ball.velocity.x > 0.0);
    assert!(
        match_state.ball.velocity.y.abs() < 1.0,
        "stationary paddle + center hit should stay horizontal, got vy={}",
        match_state.ball.velocity.y,
    );
}