// games/pong/tests/game_scenarios.rs
use pong::components::{Ball, Paddle};
use pong::systems::{self, GameContext, MatchState};
use pong::GameState;

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
    // La balle est réinitialisée au centre
    assert_eq!(
        match_state.ball.transform.position.x,
        ctx.screen_w / 2.0 - ctx.ball_size / 2.0
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
    assert_eq!(
        match_state.ball.transform.position.x,
        ctx.screen_w / 2.0 - ctx.ball_size / 2.0
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
        player.rect().x + player.rect().w - 5.0, // = 40
        120.0,
        20.0,
        400.0,
        ctx.ball_color,
    );
    match_state.ball.vx = -100.0;
    match_state.ball.vy = 0.0; // reste à la même hauteur
    let dt = 0.1;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert!(match_state.ball.vx > 0.0, "la balle doit rebondir");
    assert_eq!(
        match_state.ball.transform.position.x,
        player.rect().x + ctx.paddle_w
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
    match_state.ball = Ball::new(
        ai.rect().x + 5.0 - 20.0, // = ai.x - 15, chevauche de 5 px
        120.0,
        20.0,
        400.0,
        ctx.ball_color,
    );
    match_state.ball.vx = 100.0;
    match_state.ball.vy = 0.0;
    let dt = 0.1;

    systems::update(&mut player, &mut ai, &mut match_state, &ctx, dt);

    assert!(match_state.ball.vx < 0.0, "la balle doit rebondir");
    assert_eq!(
        match_state.ball.transform.position.x,
        ai.rect().x - ctx.ball_size
    );
}