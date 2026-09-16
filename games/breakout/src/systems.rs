// games/breakout/src/systems.rs
use crate::components::{Ball, Brick, Paddle};
use crate::levels::Level;
use ember_core::app::GameState;
use ember_core::io::load_from_file;
use glam::Vec2;
use macroquad::prelude::Color;
use std::path::PathBuf;

pub use crate::config::GameContext;

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

// ============================================================================
// BreakoutWorld — owns all mutable game state
// ============================================================================

/// All mutable state for one Breakout run.
///
/// Same convention as Pong's `MatchState`, Snake's `SnakeWorld`, Asteroids'
/// `GameWorld`, Bullet Hell's `World`, and Minesweeper's `Game`: one struct
/// owns everything that changes. `GameContext` and `dt` stay as `update`
/// parameters — they describe the world, they aren't part of it.
pub struct BreakoutWorld {
    pub paddle: Paddle,
    pub ball: Ball,
    pub bricks: Vec<Brick>,
    pub score: i32,
    pub lives: i32,
    pub current_level: usize,
    pub max_levels: usize,
    pub state: GameState,
}

impl BreakoutWorld {
    /// Build a world in `Start` state with the given first level.
    pub fn new(bricks: Vec<Brick>, ctx: &GameContext, max_levels: usize) -> Self {
        let paddle = Paddle::new(
            ctx.screen_w / 2.0 - ctx.paddle_width / 2.0,
            ctx.screen_h - 50.0,
            ctx.paddle_width,
            ctx.paddle_height,
            400.0,
            ctx.paddle_color,
        );
        let ball = Ball::new(
            ctx.screen_w / 2.0 - ctx.ball_size / 2.0,
            ctx.screen_h / 2.0,
            ctx.ball_size,
            ctx.ball_speed,
            ctx.ball_color,
        );
        Self {
            paddle,
            ball,
            bricks,
            score: 0,
            lives: 3,
            current_level: 0,
            max_levels,
            state: GameState::Start,
        }
    }

    /// Start a fresh game: reset score, lives, level, and load level 0.
    /// Does NOT set `state` (the caller decides).
    pub fn start_new_game(&mut self, bricks: Vec<Brick>, ctx: &GameContext) {
        self.score = 0;
        self.lives = 3;
        self.current_level = 0;
        self.bricks = bricks;
        self.reset_paddle(ctx);
        self.reset_ball(ctx);
    }

    /// Apply a loaded level without touching score or lives.
    pub fn apply_level(&mut self, bricks: Vec<Brick>, ctx: &GameContext) {
        self.bricks = bricks;
        self.reset_paddle(ctx);
        self.reset_ball(ctx);
    }

    /// Advance to the next level (bricks already loaded by caller).
    /// Increments `current_level` and repositions paddle / ball.
    pub fn advance_level(&mut self, bricks: Vec<Brick>, ctx: &GameContext) {
        self.current_level += 1;
        self.apply_level(bricks, ctx);
    }

    /// Handle the ball falling below the screen. Decrements lives; if zero,
    /// transitions to `GameOver`. Otherwise respawns paddle + ball.
    pub fn on_ball_lost(&mut self, ctx: &GameContext) {
        self.lives -= 1;
        if self.lives <= 0 {
            self.state = GameState::GameOver;
        } else {
            self.reset_ball(ctx);
            self.reset_paddle(ctx);
        }
    }

    /// Reset the paddle to horizontal center at the bottom.
    pub fn reset_paddle(&mut self, ctx: &GameContext) {
        self.paddle.transform.position.x = ctx.screen_w / 2.0 - ctx.paddle_width / 2.0;
    }

    /// Reset the ball to screen center with the base speed.
    pub fn reset_ball(&mut self, ctx: &GameContext) {
        self.ball.transform.position.x = ctx.screen_w / 2.0 - ctx.ball_size / 2.0;
        self.ball.transform.position.y = ctx.screen_h / 2.0;
        self.ball.vx = ctx.ball_speed;
        self.ball.vy = -ctx.ball_speed;
    }
}

// ============================================================================
// Level loading
// ============================================================================

/// Load a level from `levels/levelN.ron` (1-indexed file names).
/// Falls back to a procedurally-generated level if the file can't be read.
pub fn load_level(level_index: usize, ctx: &GameContext) -> Vec<Brick> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let level_path = manifest_dir.join(format!("levels/level{}.ron", level_index + 1));

    match load_from_file::<Level>(&level_path) {
        Ok(level) => {
            println!(
                "✅ Niveau {} chargé depuis {:?}",
                level_index + 1,
                level_path
            );
            let mut bricks = Vec::new();
            for data in level.bricks {
                let color = Color::new(data.color_r, data.color_g, data.color_b, data.color_a);
                bricks.push(Brick::new(
                    data.x,
                    data.y,
                    data.width,
                    data.height,
                    color,
                    data.health,
                ));
            }
            bricks
        }
        Err(e) => {
            println!("⚠️  Chargement niveau {} échoué : {}", level_index + 1, e);
            println!("🔨 Génération programmatique de secours...");
            generate_fallback_level(ctx)
        }
    }
}

/// Procedurally generate a fallback level if the RON file is missing.
pub fn generate_fallback_level(ctx: &GameContext) -> Vec<Brick> {
    let mut bricks = Vec::new();
    let total_width =
        ctx.brick_cols as f32 * (ctx.brick_width + ctx.brick_padding) - ctx.brick_padding;
    let start_x = (ctx.screen_w - total_width) / 2.0;
    let start_y = 60.0;

    for row in 0..ctx.brick_rows {
        for col in 0..ctx.brick_cols {
            let x = start_x + col as f32 * (ctx.brick_width + ctx.brick_padding);
            let y = start_y + row as f32 * (ctx.brick_height + ctx.brick_padding);
            let color = ctx.brick_colors[row % ctx.brick_colors.len()];
            let health = if row < 2 { 2 } else { 1 };
            bricks.push(Brick::new(
                x,
                y,
                ctx.brick_width,
                ctx.brick_height,
                color,
                health,
            ));
        }
    }
    bricks
}

// ============================================================================
// Physics
// ============================================================================

/// Resolve ball vs. one brick. Returns true if a hit occurred.
///
/// Uses center-to-center overlap test, then repositions the ball's
/// *center* against the brick's edge and converts back to top-left.
fn resolve_brick_collision(ball: &mut Ball, brick: &mut Brick, ctx: &GameContext) -> bool {
    let ball_center = ball.center();
    let ball_half = ctx.ball_size / 2.0;
    let brick_center = brick.center();
    // Use the brick's own transform.scale, not the context. Bricks can have
    // custom sizes (see level3.ron), and the collision must respect them.
    let brick_half = brick.transform.scale / 2.0;

    let dx = ball_center.x - brick_center.x;
    let dy = ball_center.y - brick_center.y;

    let overlap_x = (ball_half + brick_half.x) - dx.abs();
    let overlap_y = (ball_half + brick_half.y) - dy.abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return false;
    }

    // Resolve along the axis of least penetration.
    if overlap_x < overlap_y {
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

        let speed = (ball.vx * ball.vx + ball.vy * ball.vy)
            .sqrt()
            .max(ctx.ball_speed);
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

/// Advance the world by `dt`. Substep the ball so fast motion can't tunnel
/// through bricks. Returns the first terminal event encountered.
pub fn update(world: &mut BreakoutWorld, ctx: &GameContext, dt: f32) -> UpdateEvent {
    // Clamp speed to avoid runaway acceleration from repeated hits.
    let speed = (world.ball.vx * world.ball.vx + world.ball.vy * world.ball.vy).sqrt();
    if speed > MAX_BALL_SPEED {
        let k = MAX_BALL_SPEED / speed;
        world.ball.vx *= k;
        world.ball.vy *= k;
    }

    // Substep so each step moves at most ~half a brick height.
    let current_speed = (world.ball.vx * world.ball.vx + world.ball.vy * world.ball.vy).sqrt();
    let max_step = (ctx.brick_height.min(ctx.brick_width) * 0.5).max(1.0);
    let steps = ((current_speed * dt) / max_step).ceil().max(1.0) as usize;
    let sub_dt = dt / steps as f32;

    for _ in 0..steps {
        match step_ball(
            &mut world.paddle,
            &mut world.ball,
            &mut world.bricks,
            &mut world.score,
            ctx,
            sub_dt,
        ) {
            UpdateEvent::None => {}
            ev => return ev,
        }
    }
    UpdateEvent::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Ball, Brick};

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
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
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
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
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
        let bricks = vec![Brick::new(
            400.0,
            300.0,
            cx.brick_width,
            cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let mut world = BreakoutWorld::new(bricks, &cx, 3);
        world.state = GameState::Playing;
        // Configuration où, SANS substepping, la balle traverserait complètement.
        // Brique [400..470] x [300..325]. Balle [410..430] x [270..290].
        // Vitesse 6000 px/s * 1/60 s = 100 px → la balle finirait à y=370,
        // donc complètement sous la brique. Sans substep, elle traverserait.
        world.ball.transform.position = Vec2::new(410.0, 270.0);
        world.ball.vx = 0.0;
        world.ball.vy = 6000.0;

        let _ = update(&mut world, &cx, 1.0 / 60.0);
        assert_eq!(
            world.score, 1,
            "la balle très rapide ne doit PAS traverser la brique"
        );
    }

    #[test]
    fn test_collision_uses_brick_transform_scale_not_context() {
        let cx = ctx();
        let brick_width = 200.0;
        let brick_height = 50.0;
        let mut brick = Brick::new(
            500.0 - brick_width / 2.0,  // top-left = 400
            300.0 - brick_height / 2.0, // top-left = 275
            brick_width,
            brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        );
        assert_eq!(brick.transform.scale, Vec2::new(brick_width, brick_height));

        // Balle centrée à (415, 300), taille 20 → occupe [405..425] x [290..310].
        //
        // Contre la brique CUSTOM (500, 300, demi-taille 100×25) :
        //   dx = -85, dy = 0
        //   overlap_x = (10 + 100) - 85 = 25
        //   overlap_y = (10 + 25)  - 0  = 35
        //   25 < 35 → axe X (horizontal) → bounce_x()
        //
        // Contre la brique CTX (demi-largeur 35, bord gauche 465) :
        //   La balle [405..425] ne touche pas [465..535] → pas de collision.
        //   Donc si le code utilisait ctx.brick_width, `hit` serait false.
        let mut ball = Ball::new(
            405.0, // top-left x → center = 415
            290.0, // top-left y → center = 300
            cx.ball_size,
            cx.ball_speed,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        ball.vx = 100.0; // va vers la droite
        ball.vy = 0.0;

        let hit = resolve_brick_collision(&mut ball, &mut brick, &cx);

        assert!(
            hit,
            "la balle doit toucher la brique custom (bord gauche = 400, balle 405..425)"
        );
        assert!(
            ball.vx < 0.0,
            "hit horizontal attendu : overlap_x (25) < overlap_y (35)"
        );
    }

    #[test]
    fn test_world_new_starts_at_start_state() {
        let cx = ctx();
        let bricks = vec![Brick::new(
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let world = BreakoutWorld::new(bricks, &cx, 3);
        assert_eq!(world.state, GameState::Start);
        assert_eq!(world.score, 0);
        assert_eq!(world.lives, 3);
        assert_eq!(world.current_level, 0);
        assert_eq!(world.max_levels, 3);
    }

    #[test]
    fn test_on_ball_lost_decrements_lives() {
        let cx = ctx();
        let bricks = vec![Brick::new(
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let mut world = BreakoutWorld::new(bricks, &cx, 3);
        world.state = GameState::Playing;
        world.on_ball_lost(&cx);
        assert_eq!(world.lives, 2);
        assert_eq!(world.state, GameState::Playing);
    }

    #[test]
    fn test_on_ball_lost_at_zero_lives_triggers_game_over() {
        let cx = ctx();
        let bricks = vec![Brick::new(
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let mut world = BreakoutWorld::new(bricks, &cx, 3);
        world.state = GameState::Playing;
        world.lives = 1;
        world.on_ball_lost(&cx);
        assert_eq!(world.lives, 0);
        assert_eq!(world.state, GameState::GameOver);
    }

    #[test]
    fn test_start_new_game_resets_state() {
        let cx = ctx();
        let bricks = vec![Brick::new(
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let mut world = BreakoutWorld::new(bricks.clone(), &cx, 3);
        world.score = 42;
        world.lives = 1;
        world.current_level = 2;
        world.start_new_game(bricks, &cx);
        assert_eq!(world.score, 0);
        assert_eq!(world.lives, 3);
        assert_eq!(world.current_level, 0);
    }

    #[test]
    fn test_advance_level_increments() {
        let cx = ctx();
        let bricks = vec![Brick::new(
            400.0,
            100.0,
            cx.brick_width,
            cx.brick_height,
            Color::new(1.0, 1.0, 1.0, 1.0),
            1,
        )];
        let mut world = BreakoutWorld::new(bricks.clone(), &cx, 3);
        world.state = GameState::Playing;
        world.advance_level(bricks, &cx);
        assert_eq!(world.current_level, 1);
    }
}
