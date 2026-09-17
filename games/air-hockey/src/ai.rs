//! Goalkeeper AI for Player 2.
//!
//! Strategy: stay on a vertical line near the right goal, slide up and
//! down to block. Two fairness knobs:
//!
//! - REACTION_TIME: the AI only re-reads the puck every N seconds.
//!   Between reads it moves toward the last-seen y.
//! - AI_SPEED: fraction of `paddle_speed_max` the AI can reach, so the
//!   player can beat it to a corner.
//!
//! The AI never chases the puck into its own half: pushing a puck that
//! is behind the paddle just shoves it into the goal. When the puck is
//! behind, the AI recovers to the goal line instead.

use glam::Vec2;

use crate::arena::Arena;
use crate::components::{Paddle, Puck};
use crate::config::Tuning;

/// Seconds between puck "looks". Bigger = easier to beat.
const REACTION_TIME: f32 = 0.18;

/// Fraction of `paddle_speed_max` the AI is allowed to reach.
/// 1.0 = as fast as the player. 0.55 = clearly beatable.
const AI_SPEED: f32 = 0.55;

/// Distance from the inner wall to the goalie's resting x.
/// Smaller = hugs the goal, larger = plays a bit more forward.
const GOALIE_OFFSET: f32 = 70.0;

/// Minimum deadzone in world units. The effective deadzone is scaled by
/// how far the paddle travels in one frame so it never jitters at speed.
const MIN_DEADZONE: f32 = 6.0;

/// AI state, owned by `Game`.
pub struct AiState {
    /// Last y the AI saw when it "looked" at the puck.
    perceived_y: f32,
    /// Time remaining until the next look.
    timer: f32,
}

impl AiState {
    pub fn new() -> Self {
        Self { perceived_y: 0.0, timer: 0.0 }
    }

    /// Force a fresh look on the next update (e.g. at round start).
    pub fn reset(&mut self) {
        self.timer = 0.0;
    }
}

impl Default for AiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the AI's desired movement direction for Player 2's paddle.
/// The vector's magnitude is the desired speed as a fraction of
/// `paddle_speed_max` — see `integrate_paddle`.
pub fn direction(
    pad: &Paddle,
    puck: &Puck,
    arena: &Arena,
    tuning: &Tuning,
    state: &mut AiState,
    dt: f32,
) -> Vec2 {
    // 1. Reaction delay: only refresh perception every REACTION_TIME.
    state.timer -= dt;
    if state.timer <= 0.0 {
        state.perceived_y = puck.pos.y;
        state.timer = REACTION_TIME;
    }

    // 2. Goalie x-line. If we've been knocked off it, walk back.
    let goal_x = arena.w - arena.wall - pad.radius - GOALIE_OFFSET;

    // 3. Target y. If the puck is behind us in x, ignore it — chasing
    //    a puck on our own side just pushes it into the goal.
    let puck_behind = puck.pos.x > pad.pos.x;
    let target_y = if puck_behind {
        arena.mid_y()
    } else {
        state.perceived_y
    };

    // 4. Deadzone scales with per-frame travel so we don't jitter.
    let deadzone = (tuning.paddle_speed_max * AI_SPEED * dt * 0.5).max(MIN_DEADZONE);

    let dx = goal_x - pad.pos.x;
    let dy = target_y - pad.pos.y;

    let x_dir = if dx.abs() < deadzone { 0.0 } else { dx.signum() };
    let y_dir = if dy.abs() < deadzone { 0.0 } else { dy.signum() };

    let dir = Vec2::new(x_dir, y_dir);
    // Normalize then scale to the AI's speed budget. If dir is zero,
    // this stays zero (integrate_paddle will decelerate the paddle).
    dir.normalize_or_zero() * AI_SPEED
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Side;
    use crate::config::GameContext;

    fn setup() -> (GameContext, Paddle, Puck, AiState) {
        let ctx = GameContext::default_hermetic();
        let pad = Paddle::new(
            Vec2::new(
                ctx.arena.w - ctx.arena.wall - 28.0 - GOALIE_OFFSET,
                ctx.arena.mid_y(),
            ),
            28.0,
            Side::Right,
        );
        let puck = Puck::new(Vec2::new(ctx.arena.mid_x(), ctx.arena.mid_y()), 14.0);
        (ctx, pad, puck, AiState::new())
    }

    #[test]
    fn ai_returns_to_mid_when_puck_behind() {
        let (ctx, mut pad, mut puck, mut st) = setup();
        // Puck is on the right side, farther right than the paddle.
        puck.pos = Vec2::new(pad.pos.x + 30.0, 200.0);
        pad.pos.y = 200.0;
        // AI should want to go back to mid_y (down).
        let d = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert!(d.y > 0.0, "d = {:?}", d);
    }

    #[test]
    fn ai_tracks_perceived_y_with_delay() {
        let (ctx, mut pad, mut puck, mut st) = setup();
        pad.pos.y = 360.0;
        puck.pos = Vec2::new(ctx.arena.mid_x(), 100.0); // way up

        // First call: timer starts at 0, so it perceives immediately.
        let d1 = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert!(d1.y < 0.0, "expected up, got {:?}", d1);

        // Move the puck to the bottom. Within REACTION_TIME, the AI
        // should still believe the puck is up (still moving up).
        puck.pos = Vec2::new(ctx.arena.mid_x(), 600.0);
        let d2 = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert!(d2.y < 0.0, "AI should still be tracking the old y, got {:?}", d2);

        // After REACTION_TIME elapses, it should notice.
        for _ in 0..20 {
            direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        }
        let d3 = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert!(d3.y > 0.0, "AI should now target the new y, got {:?}", d3);
    }

    #[test]
    fn ai_returns_to_line_when_displaced() {
        let (ctx, mut pad, puck, mut st) = setup();
        // Push the paddle far from its line (to the left).
        pad.pos.x -= 200.0;
        let d = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert!(d.x > 0.0, "AI should walk right, got {:?}", d);
    }

    #[test]
    fn ai_does_not_move_when_aligned_and_idle() {
        let (ctx, pad, puck, mut st) = setup();
        let d = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert_eq!(d, Vec2::ZERO);
    }

    #[test]
    fn ai_output_is_within_speed_budget() {
        let (ctx, mut pad, mut puck, mut st) = setup();
        pad.pos = Vec2::new(pad.pos.x - 300.0, 100.0);
        puck.pos = Vec2::new(ctx.arena.mid_x(), 700.0);
        let d = direction(&pad, &puck, &ctx.arena, &ctx.tuning, &mut st, 1.0 / 60.0);
        assert!(d.length() <= AI_SPEED + 1e-5, "len = {}", d.length());
    }
}