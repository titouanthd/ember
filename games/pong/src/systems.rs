// games/pong/src/systems.rs
use crate::components::{Ball, Paddle};

use ember_core::app::GameState;
use ember_stdlib::collides;

/// Maximum speed magnitude for the ball. Prevents runaway acceleration
/// from repeated paddle hits (each hit multiplies vx by
/// `ctx.ball_speed_increment`). Same value as Breakout's MAX_BALL_SPEED.
const MAX_BALL_SPEED: f32 = 1200.0;

pub use crate::config::GameContext;

/// État d'un match Pong : la balle, les scores, l'état de la partie.
///
/// Les raquettes restent des paramètres d'`update` : ce sont des acteurs,
/// pas de l'état de match. `GameContext` et `dt` sont également passés
/// séparément : la config et le temps ne sont pas de l'état.
#[derive(Debug, Clone)]
pub struct MatchState {
    pub ball: Ball,
    pub score_player: i32,
    pub score_ai: i32,
    pub state: GameState,
}

impl MatchState {
    /// Crée un match frais : balle au centre, scores à 0, en `Start`.
    pub fn new(ctx: &GameContext) -> Self {
        Self {
            ball: Ball::new(
                ctx.screen_w / 2.0 - ctx.ball_size / 2.0,
                ctx.screen_h / 2.0 - ctx.ball_size / 2.0,
                ctx.ball_size,
                ctx.base_speed,
                ctx.ball_color,
            ),
            score_player: 0,
            score_ai: 0,
            state: GameState::Start,
        }
    }

    /// Remet la balle au centre et les scores à zéro.
    ///
    /// **Ne touche pas à `state`** : l'appelant décide de l'état
    /// (Playing après un démarrage, Start après un restart, etc.).
    pub fn reset(&mut self, ctx: &GameContext) {
        self.ball = Ball::new(
            ctx.screen_w / 2.0 - ctx.ball_size / 2.0,
            ctx.screen_h / 2.0 - ctx.ball_size / 2.0,
            ctx.ball_size,
            ctx.base_speed,
            ctx.ball_color,
        );
        self.score_player = 0;
        self.score_ai = 0;
    }

    /// Réinitialise la balle au centre en préservant les scores.
    /// `vx_sign` : +1.0 pour servir vers la droite, -1.0 vers la gauche.
    pub fn reset_ball(&mut self, ctx: &GameContext, vx_sign: f32) {
        self.ball = Ball::new(
            ctx.screen_w / 2.0 - ctx.ball_size / 2.0,
            ctx.screen_h / 2.0 - ctx.ball_size / 2.0,
            ctx.ball_size,
            ctx.base_speed,
            ctx.ball_color,
        );
        self.ball.vx = ctx.base_speed * vx_sign;
    }
}

/// Remet les raquettes au centre vertical et le match à zéro.
///
/// **Ne touche pas à `match_state.state`** : l'appelant positionne l'état
/// ensuite (`Playing` pour un démarrage, etc.).
pub fn reset_game(
    player: &mut Paddle,
    ai: &mut Paddle,
    match_state: &mut MatchState,
    ctx: &GameContext,
) {
    player.transform.position.y = ctx.screen_h / 2.0 - ctx.paddle_h / 2.0;
    ai.transform.position.y = ctx.screen_h / 2.0 - ctx.paddle_h / 2.0;

    match_state.reset(ctx);
    match_state.ball.vx = ctx.base_speed;
}

/// IA du paddle adverse.
pub fn ai_move(ai: &mut Paddle, ball_y: f32, dt: f32, screen_h: f32) {
    let center = ai.transform.center().y;
    if center < ball_y - 5.0 {
        ai.move_down(dt, screen_h);
    } else if center > ball_y + 5.0 {
        ai.move_up(dt);
    }
}

pub fn update(
    player: &mut Paddle,
    ai: &mut Paddle,
    match_state: &mut MatchState,
    ctx: &GameContext,
    dt: f32,
) {
    // Mouvements IA
    ai_move(
        ai,
        match_state.ball.transform.position.y + ctx.ball_size / 2.0,
        dt,
        ctx.screen_h,
    );

    // Mise à jour balle
    match_state.ball.update(dt);

    // Collisions murs haut/bas
    if match_state.ball.transform.position.y < 0.0
        || match_state.ball.transform.position.y + ctx.ball_size > ctx.screen_h
    {
        match_state.ball.bounce_y();
        match_state.ball.transform.position.y = match_state
            .ball
            .transform
            .position
            .y
            .clamp(0.0, ctx.screen_h - ctx.ball_size);
    }

    // Collision raquette joueur
    if collides(
        player.transform.center(),
        &player.collider,
        match_state.ball.transform.center(),
        &match_state.ball.collider,
    ) {
        match_state.ball.bounce_x();
        match_state.ball.transform.position.x = player.rect().x + ctx.paddle_w;
        match_state.ball.vx *= ctx.ball_speed_increment;
    }

    // Collision raquette IA
    if collides(
        ai.transform.center(),
        &ai.collider,
        match_state.ball.transform.center(),
        &match_state.ball.collider,
    ) {
        match_state.ball.bounce_x();
        match_state.ball.transform.position.x = ai.rect().x - ctx.ball_size;
        match_state.ball.vx *= ctx.ball_speed_increment;
    }

    // Score IA (balle sort à gauche)
    if match_state.ball.transform.position.x < -ctx.ball_size {
        match_state.score_ai += 1;
        if match_state.score_ai >= ctx.win_score {
            match_state.state = GameState::GameOver;
        } else {
            match_state.reset_ball(ctx, 1.0);
        }
    }

    // Score Joueur (balle sort à droite)
    if match_state.ball.transform.position.x > ctx.screen_w + ctx.ball_size {
        match_state.score_player += 1;
        if match_state.score_player >= ctx.win_score {
            match_state.state = GameState::Win;
        } else {
            match_state.reset_ball(ctx, -1.0);
        }
    }

    // Clamp once at the end — covers both paddle collisions and any other
    // path that could have modified vx/vy this frame.
    match_state.ball.clamp_speed(MAX_BALL_SPEED);
}
