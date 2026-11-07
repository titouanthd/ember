// games/pong/src/systems.rs
use ember_core::rng::Rng;
use ember_core::time::Cooldown;
use ember_stdlib::app::GameState;
use glam::Vec2;

use crate::components::{Ball, Paddle};

pub use crate::config::GameContext;

/// État d'un match Pong : balle, scores, état, IA (réaction + cible), RNG.
///
/// Les raquettes restent des paramètres d'`update` : ce sont des acteurs,
/// pas de l'état de match. `GameContext` et `dt` sont également passés
/// séparément : config et temps ne sont pas de l'état.
#[derive(Debug, Clone)]
pub struct MatchState {
    pub ball: Ball,
    pub score_player: i32,
    pub score_ai: i32,
    pub state: GameState,
    /// Timer de re-target IA. Pattern périodique : `tick` + `is_ready` +
    /// `reset`. **Ne pas** utiliser `tick_returning` ici — cette méthode
    /// est pour les one-shots et ne fire jamais après un `clear()`.
    pub ai_reaction: Cooldown,
    /// Cible Y de l'IA, mise à jour à chaque cycle de `ai_reaction`.
    pub ai_target_y: f32,
    /// RNG pour varier l'angle du service.
    pub rng: Rng,
}

impl MatchState {
    /// Crée un match frais avec un seed dérivé du temps réel
    /// (services variés à chaque lancement).
    pub fn new(ctx: &GameContext) -> Self {
        Self::new_with_seed(ctx, time_seed())
    }

    /// Variante déterministe — à utiliser dans les tests.
    pub fn new_with_seed(ctx: &GameContext, seed: u32) -> Self {
        Self {
            ball: ball_at_center(ctx, Vec2::ZERO),
            score_player: 0,
            score_ai: 0,
            state: GameState::Start,
            ai_reaction: Cooldown::new(ctx.ai_reaction_time),
            ai_target_y: ctx.screen_h / 2.0,
            rng: Rng::new(seed),
        }
    }

    /// Place la balle au centre et lui donne une vélocité de service
    /// dans `direction` (±1 sur X) avec un angle aléatoire borné par
    /// `ctx.serve_angle_max`. **Ne touche ni aux scores ni à `state`.**
    pub fn serve(&mut self, ctx: &GameContext, direction: f32) {
        let angle = self
            .rng
            .next_f32_range(-ctx.serve_angle_max, ctx.serve_angle_max);
        let velocity = Vec2::new(
            direction * ctx.base_speed * angle.cos(),
            ctx.base_speed * angle.sin(),
        );
        self.ball = ball_at_center(ctx, velocity);
        // Ré-arme la réaction IA pour qu'elle réagisse au nouveau service.
        self.ai_reaction.clear();
        self.ai_target_y = ctx.screen_h / 2.0;
    }
}

fn ball_at_center(ctx: &GameContext, velocity: Vec2) -> Ball {
    Ball::new(
        ctx.screen_w / 2.0 - ctx.ball_size / 2.0,
        ctx.screen_h / 2.0 - ctx.ball_size / 2.0,
        ctx.ball_size,
        velocity,
        ctx.ball_color,
    )
}

fn time_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() as u32) ^ 0xA5A5_A5A5)
        .unwrap_or(0xDEAD_BEEF)
}

/// Remet les raquettes au centre, scores à zéro, et sert la balle vers la
/// droite. **Ne touche pas à `match_state.state`** — l'appelant décide.
pub fn reset_match(
    player: &mut Paddle,
    ai: &mut Paddle,
    match_state: &mut MatchState,
    ctx: &GameContext,
) {
    let paddle_y = ctx.screen_h / 2.0 - ctx.paddle_h / 2.0;
    player.transform.position.y = paddle_y;
    ai.transform.position.y = paddle_y;
    player.velocity_y = 0.0;
    ai.velocity_y = 0.0;
    match_state.score_player = 0;
    match_state.score_ai = 0;
    match_state.serve(ctx, 1.0);
}

/// IA du paddle adverse. Se déplace vers `target_y` avec une dead-zone
/// de 5 px. La cible est périmée entre deux ticks de `MatchState::ai_reaction`,
/// ce qui donne à l'IA une latence réaliste.
pub fn ai_move(ai: &mut Paddle, target_y: f32, dt: f32, screen_h: f32) {
    let center = ai.transform.center().y;
    let dead_zone = 5.0;
    if center < target_y - dead_zone {
        ai.move_down(dt, screen_h);
    } else if center > target_y + dead_zone {
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
    // Reset la vélocité IA avant que `ai_move` ne la recalcule. Le player
    // est reset par le caller (main loop) avant les inputs.
    ai.reset_velocity();

    // --- IA : re-target périodique ---
    match_state.ai_reaction.tick(dt);
    if match_state.ai_reaction.is_ready() {
        match_state.ai_target_y = match_state.ball.transform.center().y;
        match_state.ai_reaction.reset();
    }
    ai_move(ai, match_state.ai_target_y, dt, ctx.screen_h);

    // --- Balle ---
    match_state.ball.update(dt);

    // --- Murs haut/bas : force la direction, pas de flip aveugle ---
    if match_state.ball.transform.position.y < 0.0 {
        match_state.ball.transform.position.y = 0.0;
        match_state.ball.velocity.y = match_state.ball.velocity.y.abs();
    } else if match_state.ball.transform.position.y + match_state.ball.transform.scale.y
        > ctx.screen_h
    {
        match_state.ball.transform.position.y = ctx.screen_h - match_state.ball.transform.scale.y;
        match_state.ball.velocity.y = -match_state.ball.velocity.y.abs();
    }

    // --- Collisions raquettes ---
    resolve_paddle_collision(&mut match_state.ball, player, ctx);
    resolve_paddle_collision(&mut match_state.ball, ai, ctx);

    // --- Scoring ---
    if match_state.ball.transform.position.x + match_state.ball.transform.scale.x < 0.0 {
        match_state.score_ai += 1;
        if match_state.score_ai >= ctx.win_score {
            match_state.state = GameState::GameOver;
        } else {
            match_state.serve(ctx, 1.0);
        }
    } else if match_state.ball.transform.position.x > ctx.screen_w {
        match_state.score_player += 1;
        if match_state.score_player >= ctx.win_score {
            match_state.state = GameState::Win;
        } else {
            match_state.serve(ctx, -1.0);
        }
    }

    // Ceinture + bretelles : couvre tous les chemins qui ont pu toucher
    // à la vélocité cette frame.
    match_state.ball.clamp_speed(ctx.max_ball_speed);
}

fn resolve_paddle_collision(ball: &mut Ball, paddle: &Paddle, ctx: &GameContext) {
    let ball_aabb = ball.aabb();
    let paddle_aabb = paddle.aabb();

    if let Some((axis, depth)) = ball_aabb.penetration(&paddle_aabb) {
        // 1. Désimbrique.
        ball.transform.position += axis * depth;

        // 2. Ne rebondit que si la balle va vers la raquette.
        let vn = ball.velocity.dot(axis);
        if vn >= 0.0 {
            return;
        }

        // 3. Réflexion miroir (mur plat) : v' = v − 2(v·n)n.
        //    L'angle d'incidence est préservé, seule la composante
        //    normale à la surface est inversée.
        ball.velocity -= 2.0 * vn * axis;

        // 4. English : la vélocité Y de la raquette s'ajoute à la balle.
        //    Raquette immobile → rebond miroir pur. Raquette en
        //    mouvement → trajectoire courbée en Y.
        ball.velocity.y += paddle.velocity_y * ctx.paddle_velocity_factor;

        // 5. Accélération à chaque hit (couvre aussi l'english ajouté
        //    ci-dessus : tout le vecteur sortant est scalé).
        ball.velocity *= ctx.ball_speed_increment;
    }
}