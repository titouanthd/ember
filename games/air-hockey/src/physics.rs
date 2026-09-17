//! Physics: integration and collision resolution.
//!
//! Pure with respect to the outside world: no macroquad, no rendering, no
//! audio. Only `Paddle` and `Puck` are mutated in place.
//!
//! Coordinate convention follows the rest of the game: origin top-left,
//! x grows right, y grows down. `Side::Left` attacks toward +x.
//!
//! Sub-step order (DESIGN.md §7.5):
//!   1. Integrate paddles
//!   2. Integrate puck
//!   3. Detect goal (early-return)
//!   4. Bounce off walls
//!   5. Bounce off paddles
//!   6. Clamp puck speed

use glam::Vec2;

use crate::arena::Arena;
use crate::components::{Paddle, Puck, Side};
use crate::config::GameContext;

/// Events emitted by one physics step. `systems.rs` turns these into score
/// changes; `main.rs` turns them into audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionEvent {
    /// Puck bounced off a wall (top, bottom, or a side outside the goal).
    Wall,
    /// Puck bounced off a paddle. `impact_speed` is the puck speed at the
    /// moment of contact, before the bounce.
    Paddle { impact_speed: f32 },
    /// A goal was scored. The `Side` is the *scorer*, not the goal.
    Goal(Side),
}

// ─── Integration ───────────────────────────────────────────────────────────

/// Integrate a paddle for `dt` seconds.
///
/// `input` is a desired direction; it is normalized internally, so callers
/// may pass raw `(dx, dy)` from the keyboard without pre-normalizing.
pub fn integrate_paddle(p: &mut Paddle, input: Vec2, ctx: &GameContext, dt: f32) {
    let t = &ctx.tuning;

    // In integrate_paddle, replace the target_v calculation:
    let target_v = if input.length_squared() > 1e-9 {
        input.clamp_length_max(1.0) * t.paddle_speed_max
    } else {
        Vec2::ZERO
    };

    let delta = target_v - p.vel;
    let max_delta = t.paddle_accel * dt;
    let clamped = if delta.length_squared() > max_delta * max_delta {
        delta.normalize() * max_delta
    } else {
        delta
    };

    p.vel += clamped;
    p.pos += p.vel * dt;

    let before = p.pos;
    p.pos = ctx.arena.clamp_to_half(p.pos, p.side, p.radius);
    // Zero the velocity component along any axis we were clamped on, so the
    // paddle doesn't wind up against a wall.
    if (before.x - p.pos.x).abs() > 1e-6 {
        p.vel.x = 0.0;
    }
    if (before.y - p.pos.y).abs() > 1e-6 {
        p.vel.y = 0.0;
    }
}

/// Integrate the puck for one sub-step.
///
/// The friction exponent is `dt_sub * 60.0` so the total loss per second is
/// independent of how many sub-steps we use.
pub fn integrate_puck_step(p: &mut Puck, ctx: &GameContext, dt_sub: f32) {
    let t = &ctx.tuning;
    p.vel *= t.puck_friction.powf(dt_sub * 60.0);
    p.pos += p.vel * dt_sub;
    p.spin += p.vel.length() * dt_sub * 0.01;
}

/// Hard clamp on puck speed. Called at the end of every sub-step.
pub fn clamp_puck_speed(p: &mut Puck, ctx: &GameContext) {
    let max = ctx.tuning.puck_speed_max;
    let speed_sq = p.vel.length_squared();
    if speed_sq > max * max {
        p.vel *= max / speed_sq.sqrt();
    }
}

// ─── Collisions ────────────────────────────────────────────────────────────

/// Bounce off top/bottom walls and off left/right walls where there is no
/// goal opening. Returns `true` if any bounce happened.
fn resolve_walls(p: &mut Puck, arena: &Arena) -> bool {
    let mut hit = false;
    let r = p.radius;

    // Top
    if p.pos.y - r < arena.wall {
        p.pos.y = arena.wall + r;
        if p.vel.y < 0.0 {
            p.vel.y = -p.vel.y;
        }
        hit = true;
    }
    // Bottom
    if p.pos.y + r > arena.h - arena.wall {
        p.pos.y = arena.h - arena.wall - r;
        if p.vel.y > 0.0 {
            p.vel.y = -p.vel.y;
        }
        hit = true;
    }

    // Left / right — skip when the puck is inside the goal opening, so it
    // can slide into the goal pocket instead of bouncing off the wall.
    if !arena.is_in_goal_y(p.pos.y) {
        if p.pos.x - r < arena.wall {
            p.pos.x = arena.wall + r;
            if p.vel.x < 0.0 {
                p.vel.x = -p.vel.x;
            }
            hit = true;
        }
        if p.pos.x + r > arena.w - arena.wall {
            p.pos.x = arena.w - arena.wall - r;
            if p.vel.x > 0.0 {
                p.vel.x = -p.vel.x;
            }
            hit = true;
        }
    }

    hit
}

/// Detect a goal. The puck is stopped (velocity zeroed) if a goal is
/// detected, so the phase machine has a stable state to read.
///
/// The returned side is the one that **scored**:
/// - puck entered the left goal  → `Side::Right` scored.
/// - puck entered the right goal → `Side::Left` scored.
fn detect_goal(p: &mut Puck, arena: &Arena) -> Option<Side> {
    if !arena.is_in_goal_y(p.pos.y) {
        return None;
    }
    if arena.is_past_left_goal_line(p.pos.x, p.radius) {
        p.vel = Vec2::ZERO;
        return Some(Side::Right);
    }
    if arena.is_past_right_goal_line(p.pos.x, p.radius) {
        p.vel = Vec2::ZERO;
        return Some(Side::Left);
    }
    None
}

/// Resolve one puck-vs-paddle contact, if any.
///
/// Formula (DESIGN.md §7.4):
///   n    = normalized(puck - paddle)
///   v_rel = puck.vel - paddle.vel
///   v_n   = v_rel · n
///   if v_n < 0:  reflect v_rel with restitution, then add paddle.vel * transfer
///   then add tangential component of (paddle.vel - puck.vel) * curve_factor
fn resolve_paddle(p: &mut Puck, pad: &Paddle, ctx: &GameContext) -> Option<CollisionEvent> {
    let t = &ctx.tuning;
    let d = p.pos - pad.pos;
    let r_sum = p.radius + pad.radius;
    let dist_sq = d.length_squared();
    if dist_sq >= r_sum * r_sum {
        return None;
    }

    let dist = dist_sq.sqrt();
    // Degenerate case: puck exactly at the paddle centre. Push it out along
    // the paddle's attack direction so the game doesn't lock up.
    let n = if dist > 1e-6 {
        d / dist
    } else {
        Vec2::new(pad.side.attack_sign(), 0.0)
    };
    let penetration = r_sum - dist;
    p.pos += n * penetration;

    let impact_speed = p.vel.length();

    let v_rel = p.vel - pad.vel;
    let v_n = v_rel.dot(n);

    if v_n < 0.0 {
        // Reflect relative velocity with restitution.
        let v_rel_new = v_rel - (1.0 + t.restitution) * v_n * n;
        // Recompose in world frame, with partial transfer of paddle velocity.
        p.vel = v_rel_new + pad.vel * t.transfer;
    }

    // Directional control: tangential component of relative motion.
    // This is what lets the player "aim" rather than just mirror.
    let tangent = Vec2::new(-n.y, n.x);
    let rel_t = (pad.vel - p.vel).dot(tangent);
    p.vel += tangent * rel_t * t.curve_factor;

    Some(CollisionEvent::Paddle { impact_speed })
}

// ─── World step ────────────────────────────────────────────────────────────

/// Advance the world by `dt` seconds using fixed sub-steps.
///
/// `inputs` are the two players' raw movement directions, indexed by
/// `paddles` order (index 0 = player 1 / left, index 1 = player 2 / right).
///
/// Returns the list of collision events that occurred during the frame, in
/// order. If a goal is detected, the function returns immediately with the
/// `Goal` event as the final element.
pub fn step_world_substepped(
    paddles: &mut [Paddle; 2],
    puck: &mut Puck,
    inputs: [Vec2; 2],
    ctx: &GameContext,
    dt: f32,
) -> Vec<CollisionEvent> {
    let substeps = ctx.tuning.substeps.max(1);
    let sub_dt = dt / substeps as f32;
    let mut events = Vec::new();

    for _ in 0..substeps {
        // 1. Integrate paddles.
        for (i, pad) in paddles.iter_mut().enumerate() {
            integrate_paddle(pad, inputs[i], ctx, sub_dt);
        }

        // 2. Integrate puck.
        integrate_puck_step(puck, ctx, sub_dt);

        // 3. Detect goal — early-return, no further physics this frame.
        if let Some(scorer) = detect_goal(puck, &ctx.arena) {
            events.push(CollisionEvent::Goal(scorer));
            return events;
        }

        // 4. Walls.
        if resolve_walls(puck, &ctx.arena) {
            events.push(CollisionEvent::Wall);
        }

        // 5. Paddles — at most one per sub-step.
        for pad in paddles.iter() {
            if let Some(ev) = resolve_paddle(puck, pad, ctx) {
                events.push(ev);
                break;
            }
        }

        // 6. Clamp.
        clamp_puck_speed(puck, ctx);
    }

    events
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Paddle, Puck, Side};
    use crate::config::GameContext;
    use glam::Vec2;

    fn ctx() -> GameContext {
        GameContext::default_hermetic()
    }

    fn test_paddles(c: &GameContext) -> [Paddle; 2] {
        let a = &c.arena;
        let r = 28.0;
        let left = Paddle::new(Vec2::new(a.wall + r + 2.0, a.mid_y()), r, Side::Left);
        let right = Paddle::new(Vec2::new(a.w - a.wall - r - 2.0, a.mid_y()), r, Side::Right);
        [left, right]
    }

    fn test_puck(c: &GameContext) -> Puck {
        Puck::new(Vec2::new(c.arena.mid_x(), c.arena.mid_y()), 14.0)
    }

    // ── Step 4: integration ────────────────────────────────────────────

    #[test]
    fn test_paddle_reaches_max_speed_in_012s() {
        let c = ctx();
        let mut p = Paddle::new(
            Vec2::new(c.arena.mid_x() - 200.0, c.arena.mid_y()),
            28.0,
            Side::Left,
        );
        integrate_paddle(&mut p, Vec2::new(1.0, 0.0), &c, 0.12);
        let expected = c.tuning.paddle_speed_max;
        assert!((p.vel.x - expected).abs() < 1e-3, "vel.x = {}", p.vel.x);
    }

    #[test]
    fn test_paddle_decays_to_zero_when_input_zero() {
        let c = ctx();
        let mut p = Paddle::new(
            Vec2::new(c.arena.mid_x() - 200.0, c.arena.mid_y()),
            28.0,
            Side::Left,
        );
        p.vel = Vec2::new(c.tuning.paddle_speed_max, 0.0);
        integrate_paddle(&mut p, Vec2::ZERO, &c, 0.5);
        assert!(p.vel.length() < 1e-3, "vel = {:?}", p.vel);
    }

    #[test]
    fn test_paddle_never_exceeds_max_speed() {
        let c = ctx();
        // Already at max, ask for same direction: stays at max.
        let mut p = Paddle::new(
            Vec2::new(c.arena.mid_x() - 200.0, c.arena.mid_y()),
            28.0,
            Side::Left,
        );
        p.vel = Vec2::new(c.tuning.paddle_speed_max, 0.0);
        integrate_paddle(&mut p, Vec2::new(1.0, 0.0), &c, 0.1);
        assert!(p.vel.x <= c.tuning.paddle_speed_max + 1e-3);

        // At max, ask for perpendicular: stays within the disk.
        let mut p2 = Paddle::new(
            Vec2::new(c.arena.mid_x() - 200.0, c.arena.mid_y()),
            28.0,
            Side::Left,
        );
        p2.vel = Vec2::new(c.tuning.paddle_speed_max, 0.0);
        integrate_paddle(&mut p2, Vec2::new(0.0, 1.0), &c, 0.02);
        assert!(p2.vel.length() <= c.tuning.paddle_speed_max + 1e-3);
    }

    #[test]
    fn test_puck_friction_imperceptible_in_one_second() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.vel = Vec2::new(1000.0, 0.0);
        integrate_puck_step(&mut p, &c, 1.0);
        assert!(p.vel.length() > 950.0, "speed = {}", p.vel.length());
    }

    #[test]
    fn test_puck_friction_visible_in_20_seconds() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.vel = Vec2::new(1000.0, 0.0);
        for _ in 0..20 {
            integrate_puck_step(&mut p, &c, 1.0);
        }
        assert!(p.vel.length() < 700.0, "speed = {}", p.vel.length());
    }

    #[test]
    fn test_puck_spin_accumulates() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.vel = Vec2::new(100.0, 0.0);
        assert_eq!(p.spin, 0.0);
        integrate_puck_step(&mut p, &c, 0.5);
        assert!(p.spin > 0.0, "spin = {}", p.spin);
    }

    // ── Step 5: collisions ─────────────────────────────────────────────

    #[test]
    fn test_wall_bounce_top_inverts_vy() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.pos = Vec2::new(c.arena.mid_x(), c.arena.wall + p.radius - 1.0);
        p.vel = Vec2::new(100.0, -200.0);
        let mut paddles = test_paddles(&c);
        let events =
            step_world_substepped(&mut paddles, &mut p, [Vec2::ZERO; 2], &c, 1.0 / 240.0);
        assert!(events.iter().any(|e| matches!(e, CollisionEvent::Wall)));
        assert!(p.pos.y >= c.arena.wall + p.radius - 1e-3);
        assert!(p.vel.y > 0.0, "vy = {}", p.vel.y);
    }

    #[test]
    fn test_wall_bounce_repositions_puck() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.pos = Vec2::new(c.arena.mid_x(), 2.0); // deeply inside the top wall
        p.vel = Vec2::new(0.0, -100.0);
        let mut paddles = test_paddles(&c);
        step_world_substepped(&mut paddles, &mut p, [Vec2::ZERO; 2], &c, 1.0 / 240.0);
        assert!(p.pos.y >= c.arena.wall + p.radius - 1e-3);
    }

    #[test]
    fn test_goal_detection_inside_goal_y() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.pos = Vec2::new(c.arena.wall + p.radius, c.arena.mid_y());
        p.vel = Vec2::new(-500.0, 0.0);
        let mut paddles = test_paddles(&c);
        let events =
            step_world_substepped(&mut paddles, &mut p, [Vec2::ZERO; 2], &c, 1.0 / 60.0);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CollisionEvent::Goal(Side::Right))),
            "events = {:?}",
            events
        );
    }

    #[test]
    fn test_no_goal_outside_goal_y_bounces_instead() {
        let c = ctx();
        let mut p = test_puck(&c);
        p.pos = Vec2::new(c.arena.wall + p.radius - 1.0, c.arena.goal_top() - 20.0);
        p.vel = Vec2::new(-200.0, 0.0);
        let mut paddles = test_paddles(&c);
        let events =
            step_world_substepped(&mut paddles, &mut p, [Vec2::ZERO; 2], &c, 1.0 / 240.0);
        assert!(!events
            .iter()
            .any(|e| matches!(e, CollisionEvent::Goal(_))));
        assert!(p.pos.x >= c.arena.wall + p.radius - 1e-3);
        assert!(p.vel.x > 0.0);
    }

    #[test]
    fn test_paddle_head_on_reflection_preserves_speed() {
        let c = ctx();
        let pad = Paddle::new(Vec2::new(200.0, 300.0), 28.0, Side::Left);
        // Overlap by 1 px: distance 41, r_sum 42.
        let mut p = Puck::new(Vec2::new(200.0 + 41.0, 300.0), 14.0);
        p.vel = Vec2::new(-500.0, 0.0);
        let ev = resolve_paddle(&mut p, &pad, &c);
        assert!(ev.is_some());
        assert!(
            (p.vel.length() - 500.0).abs() < 1e-3,
            "v = {:?}",
            p.vel
        );
        assert!(p.vel.x > 0.0, "v = {:?}", p.vel);
    }

    #[test]
    fn test_paddle_side_hit_adds_tangent() {
        let c = ctx();
        // Paddle moving up; puck stationary beside it (slightly below).
        let pad = Paddle {
            pos: Vec2::new(200.0, 300.0),
            vel: Vec2::new(0.0, -500.0),
            radius: 28.0,
            side: Side::Left,
        };
        let mut p = Puck::new(Vec2::new(200.0 + 40.0, 300.0 + 5.0), 14.0);
        p.vel = Vec2::ZERO;
        let ev = resolve_paddle(&mut p, &pad, &c);
        assert!(ev.is_some());
        assert!(p.vel.length() > 0.0, "v = {:?}", p.vel);
        assert!(p.vel.y < 0.0, "v = {:?}", p.vel);
    }

    #[test]
    fn test_paddle_moving_transfers_velocity() {
        let c = ctx();
        let pad = Paddle {
            pos: Vec2::new(200.0, 300.0),
            vel: Vec2::new(500.0, 0.0),
            radius: 28.0,
            side: Side::Left,
        };
        let mut p = Puck::new(Vec2::new(200.0 + 40.0, 300.0), 14.0);
        p.vel = Vec2::ZERO;
        let ev = resolve_paddle(&mut p, &pad, &c);
        assert!(ev.is_some());
        assert!(p.vel.x > 0.0, "v = {:?}", p.vel);
    }

    #[test]
    fn test_puck_speed_clamped_after_compound_hit() {
        let c = ctx();
        let mut paddles = test_paddles(&c);
        // Absurd paddle velocity to force the puck over the cap.
        paddles[0].pos = Vec2::new(300.0, 300.0);
        paddles[0].vel = Vec2::new(5000.0, 0.0);
        let mut p = Puck::new(Vec2::new(300.0 + 40.0, 300.0), 14.0);
        p.vel = Vec2::ZERO;
        step_world_substepped(&mut paddles, &mut p, [Vec2::ZERO; 2], &c, 1.0 / 240.0);
        assert!(
            p.vel.length() <= c.tuning.puck_speed_max + 1e-3,
            "speed = {}",
            p.vel.length()
        );
    }

    // ── Step 6: anti-tunneling ─────────────────────────────────────────

    #[test]
    fn test_no_tunneling_puck_at_max_speed_against_wall() {
        let c = ctx();
        let mut paddles = test_paddles(&c);
        let mut p = test_puck(&c);
        // Straight up, at max speed, from the vertical centre.
        p.vel = Vec2::new(0.0, -c.tuning.puck_speed_max);

        for _ in 0..120 {
            step_world_substepped(
                &mut paddles,
                &mut p,
                [Vec2::ZERO; 2],
                &c,
                1.0 / 60.0,
            );
            // Invariant after every frame: puck centre stays clear of both
            // horizontal walls by exactly its own radius.
            assert!(
                p.pos.y >= c.arena.wall + p.radius - 1e-3,
                "penetrated top: y = {}",
                p.pos.y
            );
            assert!(
                p.pos.y <= c.arena.h - c.arena.wall - p.radius + 1e-3,
                "penetrated bottom: y = {}",
                p.pos.y
            );
        }
    }

    #[test]
    fn test_no_tunneling_puck_through_stationary_paddle() {
        let c = ctx();
        let mut paddles = test_paddles(&c);
        // Stationary paddle at x = 500, dead centre vertically.
        paddles[0].pos = Vec2::new(500.0, 360.0);
        paddles[0].vel = Vec2::ZERO;

        // Puck rushes right at max speed from x = 300.
        let mut p = test_puck(&c);
        p.pos = Vec2::new(300.0, 360.0);
        p.vel = Vec2::new(c.tuning.puck_speed_max, 0.0);

        let mut max_x = p.pos.x;
        for _ in 0..30 {
            step_world_substepped(
                &mut paddles,
                &mut p,
                [Vec2::ZERO; 2],
                &c,
                1.0 / 60.0,
            );
            max_x = max_x.max(p.pos.x);
        }
        // Contact would place the puck centre at 500 - (28 + 14) = 458.
        // Anything near or beyond 500 means it tunneled through.
        assert!(
            max_x < paddles[0].pos.x,
            "puck reached x = {} (paddle at {})",
            max_x,
            paddles[0].pos.x
        );
    }

    #[test]
    fn test_compound_events_in_single_frame() {
        let c = ctx();
        let mut paddles = test_paddles(&c);
        let mut p = test_puck(&c);
        // Corner shot: fast puck near the top-right, moving up and right.
        // Over one 1/30 s frame, it should bounce off both walls.
        p.pos = Vec2::new(1250.0, 22.0);
        p.vel = Vec2::new(400.0, -400.0);

        let events = step_world_substepped(
            &mut paddles,
            &mut p,
            [Vec2::ZERO; 2],
            &c,
            1.0 / 30.0,
        );

        let wall_count = events
            .iter()
            .filter(|e| matches!(e, CollisionEvent::Wall))
            .count();
        assert!(
            wall_count >= 2,
            "expected at least 2 wall events, got {}: {:?}",
            wall_count,
            events
        );
        // Puck is back inside the playable area on all sides.
        assert!(p.pos.x >= c.arena.wall + p.radius - 1e-3);
        assert!(p.pos.x <= c.arena.w - c.arena.wall - p.radius + 1e-3);
        assert!(p.pos.y >= c.arena.wall + p.radius - 1e-3);
    }
}