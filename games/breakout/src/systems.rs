// systems.rs
use crate::components::{Ball, Brick, Paddle};
use ember_core::app::GameState;
use glam::Vec2;
use macroquad::prelude::Color;

pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub paddle_width: f32,
    pub paddle_height: f32,
    pub ball_size: f32,
    pub ball_speed: f32,
    pub brick_rows: usize,
    pub brick_cols: usize,
    pub brick_width: f32,
    pub brick_height: f32,
    pub brick_padding: f32,
    pub win_score: i32,
    pub paddle_color: Color,
    pub ball_color: Color,
    pub brick_colors: Vec<Color>,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 800.0,
            screen_h: 600.0,
            paddle_width: 80.0,
            paddle_height: 20.0,
            ball_size: 20.0,
            ball_speed: 400.0,
            brick_rows: 5,
            brick_cols: 8,
            brick_width: 70.0,
            brick_height: 25.0,
            brick_padding: 10.0,
            win_score: 10,
            paddle_color: Color::new(1.0, 1.0, 1.0, 1.0),
            ball_color: Color::new(1.0, 0.65, 0.0, 1.0),
            brick_colors: vec![
                Color::new(1.0, 0.0, 0.0, 1.0),
                Color::new(1.0, 0.5, 0.0, 1.0),
                Color::new(1.0, 1.0, 0.0, 1.0),
                Color::new(0.0, 1.0, 0.0, 1.0),
                Color::new(0.0, 0.0, 1.0, 1.0),
            ],
        }
    }
}

/// Maximum speed magnitude for the ball. Prevents runaway acceleration
/// and keeps substepping cheap.
const MAX_BALL_SPEED: f32 = 1200.0;

/// Result of one `update` step, so `main.rs` can react without duplicating
/// state transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateEvent {
    None,
    BallLost,
    LevelCleared,
}

pub fn reset_ball(ball: &mut Ball, ctx: &GameContext) {
    ball.transform.position.x = ctx.screen_w / 2.0 - ctx.ball_size / 2.0;
    ball.transform.position.y = ctx.screen_h / 2.0;
    ball.vx = ctx.ball_speed;
    ball.vy = -ctx.ball_speed;
}

/// Resolve ball vs. one brick. Returns true if a hit occurred.
///
/// Uses center-to-center overlap test, then repositions the ball's
/// *center* against the brick's edge and converts back to top-left.
fn resolve_brick_collision(ball: &mut Ball, brick: &mut Brick, ctx: &GameContext) -> bool {
    let ball_center = ball.center();
    let ball_half = ctx.ball_size / 2.0;
    let brick_center = brick.center();
    let brick_half = Vec2::new(ctx.brick_width / 2.0, ctx.brick_height / 2.0);

    let dx = ball_center.x - brick_center.x;
    let dy = ball_center.y - brick_center.y;

    let overlap_x = (ball_half + brick_half.x) - dx.abs();
    let overlap_y = (ball_half + brick_half.y) - dy.abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return false;
    }

    // Resolve along the axis of least penetration.
    if overlap_x < overlap_y {
        // Push ball center to just outside brick on X.
        let new_center_x = if dx < 0.0 {
            brick_center.x - brick_half.x - ball_half
        } else {
            brick_center.x + brick_half.x + ball_half
        };
        ball.transform.position.x = new_center_x - ball_half;
        ball.bounce_x();
    } else {
        let new_center_y = if dy < 0.0 {
            brick_center.y - brick_half.y - ball_half
        } else {
            brick_center.y + brick_half.y + ball_half
        };
        ball.transform.position.y = new_center_y - ball_half;
        ball.bounce_y();
    }

    brick.hit();
    true
}

/// AABB overlap test using top-left + scale. Replaces the opaque
/// `ember_stdlib::collides` call so the convention is explicit here.
fn aabb_overlap(a_pos: Vec2, a_size: Vec2, b_pos: Vec2, b_size: Vec2) -> bool {
    a_pos.x < b_pos.x + b_size.x
        && a_pos.x + a_size.x > b_pos.x
        && a_pos.y < b_pos.y + b_size.y
        && a_pos.y + a_size.y > b_pos.y
}

/// One physics substep. Returns an event if something terminal happened.
fn step_ball(
    paddle: &mut Paddle,
    ball: &mut Ball,
    bricks: &mut [Brick],
    score: &mut i32,
    ctx: &GameContext,
    dt: f32,
) -> UpdateEvent {
    ball.update(dt);

    let ball_size = ball.size();

    // --- Walls ---
    if ball.transform.position.x < 0.0 {
        ball.transform.position.x = 0.0;
        ball.bounce_x();
    }
    if ball.transform.position.x + ball_size > ctx.screen_w {
        ball.transform.position.x = ctx.screen_w - ball_size;
        ball.bounce_x();
    }
    if ball.transform.position.y < 0.0 {
        ball.transform.position.y = 0.0;
        ball.bounce_y();
    }
    if ball.transform.position.y + ball_size > ctx.screen_h {
        return UpdateEvent::BallLost;
    }

    // --- Paddle ---
    if aabb_overlap(
        paddle.transform.position,
        paddle.transform.scale,
        ball.transform.position,
        ball.transform.scale,
    ) {
        // Angle-based bounce: offset from paddle center drives the direction.
        let paddle_center_x = paddle.center().x;
        let ball_center_x = ball.center().x;
        let half_w = paddle.transform.scale.x / 2.0;
        // Normalized impact position in [-1, 1].
        let rel = ((ball_center_x - paddle_center_x) / half_w).clamp(-1.0, 1.0);

        let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt().max(ctx.ball_speed);
        let max_angle = std::f32::consts::FRAC_PI_3; // 60°
        let angle = rel * max_angle;
        ball.vx = speed * angle.sin();
        ball.vy = -speed * angle.cos();

        // Reposition on top of the paddle.
        ball.transform.position.y = paddle.transform.position.y - ball_size;
    }

    // --- Bricks ---
    for brick in bricks.iter_mut() {
        if !brick.is_alive() {
            continue;
        }
        if resolve_brick_collision(ball, brick, ctx) {
            *score += 1;
            break;
        }
    }

    if bricks.iter().all(|b| !b.is_alive()) {
        return UpdateEvent::LevelCleared;
    }

    UpdateEvent::None
}

/// Advance the ball for `dt`, substepping so fast motion can't tunnel
/// through bricks. Returns the first terminal event encountered.
pub fn update(
    paddle: &mut Paddle,
    ball: &mut Ball,
    bricks: &mut [Brick],
    score: &mut i32,
    _state: &mut GameState,
    ctx: &GameContext,
    dt: f32,
) -> UpdateEvent {
    // Clamp speed to avoid runaway acceleration from repeated hits.
    let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt();
    if speed > MAX_BALL_SPEED {
        let k = MAX_BALL_SPEED / speed;
        ball.vx *= k;
        ball.vy *= k;
    }

    // Substep so each step moves at most ~half a brick height.
    let current_speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt();
    let max_step = (ctx.brick_height.min(ctx.brick_width) * 0.5).max(1.0);
    let steps = ((current_speed * dt) / max_step).ceil().max(1.0) as usize;
    let sub_dt = dt / steps as f32;

    for _ in 0..steps {
        match step_ball(paddle, ball, bricks, score, ctx, sub_dt) {
            UpdateEvent::None => {}
            ev => return ev,
        }
    }
    UpdateEvent::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Ball, Brick, Paddle};

    fn ctx() -> GameContext {
        GameContext::default()
    }

    fn make_ball(cx: &GameContext) -> Ball {
        let mut b = Ball::new(
            cx.screen_w / 2.0,
            cx.screen_h / 2.0,
            cx.ball_size,
            cx.ball_speed,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        b.vx = 200.0;
        b.vy = -200.0;
        b
    }

    #[test]
    fn test_brick_hit_from_left_bounces_x() {
        let cx = ctx();
        let mut ball = make_ball(&cx);
        // Brique à (400, 100), taille 70x25 → occupe [400..470] x [100..125]
        // On place la balle (taille 20) juste à gauche, en chevauchant le bord :
        // balle de [385..405] x [100..120] → chevauche sur 5 px en X.
        ball.transform.position = Vec2::new(385.0, 100.0);
        ball.vx = 300.0;
        ball.vy = 0.0;
        let mut brick = Brick::new(
            400.0, 100.0,
            cx.brick_width, cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        );
        let hit = resolve_brick_collision(&mut ball, &mut brick, &cx);
        assert!(hit, "la balle doit toucher la brique");
        assert!(ball.vx < 0.0, "vx doit s'inverser sur un hit horizontal");
    }

    #[test]
    fn test_brick_hit_from_top_bounces_y() {
        let cx = ctx();
        let mut ball = make_ball(&cx);
        // Brique occupe [400..470] x [100..125]
        // Balle (taille 20) juste au-dessus, chevauchant le bord supérieur :
        // balle de [420..440] x [95..115] → chevauche sur 15 px en Y.
        ball.transform.position = Vec2::new(420.0, 95.0);
        ball.vx = 0.0;
        ball.vy = 300.0;
        let mut brick = Brick::new(
            400.0, 100.0,
            cx.brick_width, cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        );
        let hit = resolve_brick_collision(&mut ball, &mut brick, &cx);
        assert!(hit, "la balle doit toucher la brique");
        assert!(ball.vy < 0.0, "vy doit s'inverser sur un hit vertical");
    }

    #[test]
    fn test_ball_does_not_tunnel_through_brick() {
        let cx = ctx();
        let mut paddle = Paddle::new(
            0.0, 0.0,
            cx.paddle_width, cx.paddle_height,
            0.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        let mut ball = make_ball(&cx);
        // Configuration où, SANS substepping, la balle traverserait complètement.
        // Brique fine : on force une petite hauteur via une brique "logique" (mais
        // on garde la taille standard pour le test, car `resolve_brick_collision`
        // utilise ctx.brick_height).
        // On place la balle juste au-dessus, avec une vitesse telle que
        // le déplacement dépasse la hauteur de la brique + taille balle.
        // Brique [400..470] x [300..325]. Balle [410..430] x [270..290].
        // Vitesse 6000 px/s * 1/60 s = 100 px → la balle finirait à y=370,
        // donc complètement sous la brique. Sans substep, elle traverserait.
        ball.transform.position = Vec2::new(410.0, 270.0);
        ball.vx = 0.0;
        ball.vy = 6000.0;
        let mut bricks = vec![Brick::new(
            400.0, 300.0,
            cx.brick_width, cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let mut score = 0;
        let mut state = GameState::Playing;
        let _ = update(
            &mut paddle, &mut ball, &mut bricks,
            &mut score, &mut state, &cx,
            1.0 / 60.0,
        );
        assert_eq!(score, 1, "la balle très rapide ne doit PAS traverser la brique");
    }
}