// games/taiji/src/systems.rs
use crate::ai::AiController;
use crate::components::{Paddle, QiBall, Side};
use crate::config::GameContext;
use ember_core::rng::Rng;
use ember_stdlib::input::Input;
use glam::Vec2;
use macroquad::prelude::*;

pub const BASE_SPEED: f32 = 400.0;
pub const MAX_SPEED: f32 = 1200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreEvent {
    PlayerScored,
    AiScored,
}

pub struct MatchState {
    pub player: Paddle,
    pub ai: Paddle,
    pub ai_ctrl: AiController,
    pub ball: QiBall,
    pub rng: Rng,
    pub time_scale: f32,
    pub yang_burst_timer: f32,
    pub was_yin: bool,
}

impl MatchState {
    pub fn new(ctx: &GameContext) -> Self {
        let paddle_y = ctx.screen_h / 2.0 - ctx.paddle_h / 2.0;

        let player = Paddle::new(40.0, paddle_y, ctx.paddle_w, ctx.paddle_h, 600.0, ctx.color_player);
        let ai = Paddle::new(ctx.screen_w - 40.0 - ctx.paddle_w, paddle_y, ctx.paddle_w, ctx.paddle_h, 500.0, ctx.color_ai);

        let ball = QiBall::new(ctx.screen_w / 2.0, ctx.screen_h / 2.0, ctx.ball_radius, ctx.color_ball);

        let mut state = Self {
            player,
            ai,
            ai_ctrl: AiController::new(ctx.screen_h),
            ball,
            rng: Rng::new(42),
            time_scale: 1.0,
            yang_burst_timer: 0.0,
            was_yin: false,
        };
        state.reset_round(ctx, 1.0);
        state
    }

    pub fn reset_round(&mut self, ctx: &GameContext, direction: f32) {
        self.ball.transform.position = Vec2::new(ctx.screen_w / 2.0, ctx.screen_h / 2.0);
        self.ball.velocity = Vec2::new(BASE_SPEED * direction, 0.0);
        self.player.reset_velocity();
        self.ai.reset_velocity();
        self.ai_ctrl.reset(ctx.screen_h);
        self.time_scale = 1.0;
        self.yang_burst_timer = 0.0;
        self.was_yin = false;
    }
}

pub fn update(
    match_state: &mut MatchState,
    ctx: &GameContext,
    real_dt: f32,
    input: &Input,
) -> Option<ScoreEvent> {
    // --- 1. Le Souffle du Temps (Yin / Yang) ---
    let space_held = input.is_key_down(KeyCode::Space);

    if space_held {
        match_state.was_yin = true;
        match_state.yang_burst_timer = 0.0; // Annule le burst si on re-rentre en Yin
    } else if match_state.was_yin {
        // On vient de relâcher après un Yin : DÉCLENCHEMENT DU YANG
        match_state.was_yin = false;
        match_state.yang_burst_timer = 0.4; // 0.4 secondes de vitesse pure
    }

    if match_state.yang_burst_timer > 0.0 {
        match_state.yang_burst_timer -= real_dt;
    }

    let target_time_scale = if space_held {
        0.25 // Yin (Ralenti)
    } else if match_state.yang_burst_timer > 0.0 {
        2.0 // Yang (Accéléré x2)
    } else {
        1.0 // Normal
    };
    
    // Interpolation fluide entre les états
    match_state.time_scale += (target_time_scale - match_state.time_scale) * 15.0 * real_dt;
    let dt = real_dt * match_state.time_scale;

    // --- 2. Input joueur ---
    match_state.player.reset_velocity();
    if input.is_key_down(KeyCode::W) || input.is_key_down(KeyCode::Up) {
        match_state.player.move_up(dt);
    }
    if input.is_key_down(KeyCode::S) || input.is_key_down(KeyCode::Down) {
        match_state.player.move_down(dt, ctx.screen_h);
    }

    // --- 3. IA ---
    match_state.ai.reset_velocity();
    match_state.ai_ctrl.update(&mut match_state.ai, &match_state.ball, dt, ctx.screen_h, ctx.ball_radius);

    // --- 4. Mouvement de la balle ---
    match_state.ball.update(dt);

    // --- 5. Murs haut/bas ---
    if match_state.ball.transform.position.y < 0.0 {
        match_state.ball.transform.position.y = 0.0;
        match_state.ball.velocity.y = match_state.ball.velocity.y.abs();
    } else if match_state.ball.transform.position.y + match_state.ball.transform.scale.y > ctx.screen_h {
        match_state.ball.transform.position.y = ctx.screen_h - match_state.ball.transform.scale.y;
        match_state.ball.velocity.y = -match_state.ball.velocity.y.abs();
    }

    // --- 6. Collisions paddles ---
    handle_paddle_hit(match_state, Side::Player);
    handle_paddle_hit(match_state, Side::Ai);

    // --- 7. Scoring ---
    if match_state.ball.transform.position.x + match_state.ball.transform.scale.x < 0.0 {
        return Some(ScoreEvent::AiScored);
    } else if match_state.ball.transform.position.x > ctx.screen_w {
        return Some(ScoreEvent::PlayerScored);
    }

    None
}

fn handle_paddle_hit(match_state: &mut MatchState, side: Side) {
    let (paddle_aabb, paddle_center_y, paddle_half_h, paddle_vy) = match side {
        Side::Player => (match_state.player.aabb(), match_state.player.center().y, match_state.player.transform.scale.y / 2.0, match_state.player.velocity_y),
        Side::Ai => (match_state.ai.aabb(), match_state.ai.center().y, match_state.ai.transform.scale.y / 2.0, match_state.ai.velocity_y),
    };
    let ball_aabb = match_state.ball.aabb();

    if !ball_aabb.overlaps(&paddle_aabb) { return; }

    let toward = match side {
        Side::Player => match_state.ball.velocity.x < 0.0,
        Side::Ai => match_state.ball.velocity.x > 0.0,
    };
    if !toward { return; }

    if let Some((axis, depth)) = ball_aabb.penetration(&paddle_aabb) {
        match_state.ball.transform.position += axis * depth;
    }

    let ball_center_y = match_state.ball.transform.center().y;
    let relative_intersect = ((ball_center_y - paddle_center_y) / paddle_half_h).clamp(-1.0, 1.0);
    let bounce_angle = relative_intersect * 1.047;
    let english = paddle_vy * 0.001;

    let current_speed = match_state.ball.velocity.length();
    let new_speed = (current_speed * 1.05).min(MAX_SPEED);

    let dir_x = match side { Side::Player => 1.0, Side::Ai => -1.0 };
    let final_angle = (bounce_angle + english).clamp(-1.2, 1.2);

    match_state.ball.velocity = Vec2::new(
        dir_x * new_speed * final_angle.cos(),
        new_speed * final_angle.sin(),
    );
}