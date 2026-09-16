// games/pong/tests/game_scenarios.rs
//!
//! Integration scenarios for Pong.
//!
//! Assertions are behavior-focused: we check direction, containment, and
//! state transitions rather than exact f32 positions. Exact-position
//! assertions were the previous source of fragility (they coupled the
//! tests to the current reset / rebond implementation).

use pong::GameState;
use pong::components::{Ball, Paddle};
use pong::systems::{self, GameContext, MatchState};

/// Tolerance for f32 position comparisons. Positions in these tests are
/// computed from a few additions/multiplications, so 1e-3 is plenty.
const EPS: f32 = 1e-3;

// ============================================================================
// Scénarios : IA
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
// Scénarios : scoring
// ============================================================================

#[test]
fn test_score_increments_when_ball_leaves_left() {
    let ctx = GameContext::default();
    let mut player = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.player_color);
    let mut ai = Paddle::new(0.0, 0.0, 15.0, 80.0, 100.0, ctx.ai_color);
    let mut match_state = MatchState::new(&ctx);
    match_state.state = GameState::Playing;
    let dt = 0.0; // bloque le déplacement pour que la balle reste à gauche

    // Balle sort à gauche (IA marque)
    match_state.ball.transform.position.x = -50.0;
    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert_eq!(match_state.score_ai, 1);
    assert_eq!(match_state.score_player, 0);

    // La balle est réinitialisée près du centre. On vérifie par proximité
    // (tolérance EPS) plutôt qu'égalité exacte : le test ne doit pas casser
    // si l'implémentation change la formule de reset.
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
    let mut match_state = MatchState::new(&ctx);
    match_state.state = GameState::Playing;
    let dt = 0.0;

    // Balle sort à droite (joueur marque)
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
    let mut match_state = MatchState::new(&ctx);
    match_state.state = GameState::Playing;
    match_state.score_player = 4; // prêt à gagner
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
    let mut match_state = MatchState::new(&ctx);
    match_state.state = GameState::Playing;
    match_state.score_ai = 4; // prêt à gagner
    let dt = 0.0;

    match_state.ball.transform.position.x = -50.0;
    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert_eq!(match_state.score_ai, 5);
    assert_eq!(match_state.state, GameState::GameOver);
}

// ============================================================================
// Scénarios : collisions raquettes
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
    let mut match_state = MatchState::new(&ctx);
    match_state.state = GameState::Playing;
    // Balle qui chevauche déjà le paddle de 5 px, allant vers la gauche
    match_state.ball = Ball::new(
        player.rect().x + player.rect().w - 5.0,
        120.0,
        20.0,
        400.0,
        ctx.ball_color,
    );
    match_state.ball.vx = -100.0;
    match_state.ball.vy = 0.0;
    let dt = 0.1;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    // Comportement : la balle a rebondi (vx > 0).
    assert!(
        match_state.ball.vx > 0.0,
        "ball should bounce off the player paddle (vx > 0), got {}",
        match_state.ball.vx,
    );

    // Contenance : la balle ne doit plus pénétrer dans le paddle.
    // Son bord droit doit être >= au bord droit du paddle (tolérance EPS).
    let ball_right = match_state.ball.transform.position.x + ctx.ball_size;
    let paddle_right = player.rect().x + ctx.paddle_w;
    assert!(
        ball_right >= paddle_right - EPS,
        "ball (right={ball_right}) should not be inside the paddle (right={paddle_right})",
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
    let mut match_state = MatchState::new(&ctx);
    match_state.state = GameState::Playing;
    // Balle qui chevauche déjà le paddle IA de 5 px, allant vers la droite
    match_state.ball = Ball::new(ai.rect().x + 5.0 - 20.0, 120.0, 20.0, 400.0, ctx.ball_color);
    match_state.ball.vx = 100.0;
    match_state.ball.vy = 0.0;
    let dt = 0.1;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    // Comportement : la balle a rebondi (vx < 0).
    assert!(
        match_state.ball.vx < 0.0,
        "ball should bounce off the AI paddle (vx < 0), got {}",
        match_state.ball.vx,
    );

    // Contenance : le bord gauche de la balle doit être <= au bord gauche
    // du paddle IA (tolérance EPS).
    let ball_left = match_state.ball.transform.position.x;
    let paddle_left = ai.rect().x;
    assert!(
        ball_left <= paddle_left + EPS,
        "ball (left={ball_left}) should not be inside the AI paddle (left={paddle_left})",
    );
}
