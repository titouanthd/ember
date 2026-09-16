//! End-to-end scenarios for Air Hockey.
//!
//! These tests do not open a window and do not touch macroquad's global
//! state: they build `GameContext::default_hermetic()`, construct a `Game`
//! directly, and drive `Game::update` with a synthetic `Input`.
//!
//! `Input` is built by hand (not via `from_macroquad*`) so the tests run
//! headless.
//!
//! Design rule for these tests (see recap piège #39): never advance the
//! game by wall-clock duration. Always drive to a phase predicate with a
//! hard frame cap. Duration-based helpers look correct for the common case
//! and fail at phase boundaries (round transitions, match transitions,
//! pauses).

use air_hockey::components::Side;
use air_hockey::{Game, GameContext, GameEvent, Phase};
use ember_stdlib::input::Input;
use glam::Vec2;
use macroquad::prelude::KeyCode;

// ─── Helpers ───────────────────────────────────────────────────────────────

fn ctx() -> GameContext {
    GameContext::default_hermetic()
}

fn empty_input() -> Input {
    Input {
        mouse_pos: Vec2::ZERO,
        mouse_left_pressed: false,
        mouse_left_down: false,
        mouse_left_released: false,
        mouse_right_pressed: false,
        mouse_right_down: false,
        mouse_right_released: false,
        mouse_middle_pressed: false,
        mouse_middle_down: false,
        mouse_middle_released: false,
        keys_pressed: Vec::new(),
        keys_down: Vec::new(),
        keys_released: Vec::new(),
    }
}

fn input_space() -> Input {
    let mut i = empty_input();
    i.keys_pressed = vec![KeyCode::Space];
    i
}

/// Drive the game until `pred(game.phase)` holds, or `max_frames` is hit.
/// Panics with the current phase if the target is never reached.
fn advance_until<F>(
    g: &mut Game,
    c: &GameContext,
    max_frames: usize,
    pred: F,
    label: &str,
) where
    F: Fn(Phase) -> bool,
{
    for _ in 0..max_frames {
        if pred(g.phase) {
            return;
        }
        g.update(&empty_input(), c, 1.0 / 60.0);
    }
    assert!(
        pred(g.phase),
        "advance_until({label}): phase = {:?}",
        g.phase
    );
}

/// A game already in `Phase::Playing`, puck at rest at centre.
fn game_in_playing(c: &GameContext) -> Game {
    let mut g = Game::new(c);
    g.update(&input_space(), c, 0.0);
    advance_until(&mut g, c, 600, |p| matches!(p, Phase::Playing), "Playing");
    g
}

/// Force a goal for `scorer` and drive the game until the Goal event fires.
/// Must be called while `phase == Playing`.
fn score_a_goal(g: &mut Game, c: &GameContext, scorer: Side) -> Vec<GameEvent> {
    assert!(
        matches!(g.phase, Phase::Playing),
        "score_a_goal: phase = {:?}",
        g.phase
    );

    let a = &c.arena;
    let y = a.mid_y();
    match scorer {
        // Left scores → puck enters the right goal.
        Side::Left => {
            g.puck.pos = Vec2::new(a.w - 20.0, y);
            g.puck.vel = Vec2::new(500.0, 0.0);
        }
        // Right scores → puck enters the left goal.
        Side::Right => {
            g.puck.pos = Vec2::new(20.0, y);
            g.puck.vel = Vec2::new(-500.0, 0.0);
        }
    }

    let mut all = Vec::new();
    for _ in 0..30 {
        all.extend(g.update(&empty_input(), c, 1.0 / 60.0));
        if all.iter().any(|e| matches!(e, GameEvent::Goal(_))) {
            break;
        }
    }
    assert!(
        all.iter().any(|e| matches!(e, GameEvent::Goal(_))),
        "score_a_goal: no Goal event for {:?} (phase = {:?})",
        scorer,
        g.phase
    );
    all
}

/// After a goal (round not over): drive through GoalPause and Countdown
/// until `Playing` again.
fn advance_to_next_playing(g: &mut Game, c: &GameContext) {
    advance_until(g, c, 600, |p| matches!(p, Phase::Playing), "Playing");
}

/// After a round-winning goal: drive through GoalPause until `RoundOver`
/// or `MatchOver`.
fn advance_to_round_end(g: &mut Game, c: &GameContext) {
    advance_until(
        g,
        c,
        600,
        |p| matches!(p, Phase::RoundOver { .. } | Phase::MatchOver { .. }),
        "RoundOver|MatchOver",
    );
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[test]
fn test_full_round_j1_scores_7_goals() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    let goals_to_win = c.tuning.goals_to_win_round;

    for i in 0..goals_to_win {
        let ev = score_a_goal(&mut g, &c, Side::Left);
        assert!(ev.contains(&GameEvent::Goal(Side::Left)), "goal {i}");

        // If this wasn't the winning goal, the game returns to Playing.
        if i + 1 < goals_to_win {
            advance_to_next_playing(&mut g, &c);
        }
    }

    assert_eq!(g.score.p1, goals_to_win);

    // The round ends when the winning goal's GoalPause expires.
    advance_to_round_end(&mut g, &c);
    assert_eq!(g.rounds.p1, 1);
    assert!(
        matches!(g.phase, Phase::RoundOver { winner: Side::Left, .. }),
        "phase = {:?}",
        g.phase
    );
}

#[test]
fn test_match_best_of_three() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    let wins_needed = c.tuning.rounds_to_win_match;

    for round in 0..wins_needed {
        for _ in 0..c.tuning.goals_to_win_round {
            score_a_goal(&mut g, &c, Side::Left);
            // After a non-winning goal, drive back to Playing so the next
            // `score_a_goal` has a live game to work with.
            if g.score.p1 < c.tuning.goals_to_win_round {
                advance_to_next_playing(&mut g, &c);
            }
        }
        // Round is over: drive to RoundOver or MatchOver.
        advance_to_round_end(&mut g, &c);

        if round + 1 < wins_needed {
            // RoundOver → Countdown → Playing for the next round.
            advance_until(&mut g, &c, 600, |p| matches!(p, Phase::Playing), "Playing");
        }
    }

    assert_eq!(g.rounds.p1, wins_needed);
    assert!(
        matches!(g.phase, Phase::MatchOver { winner: Side::Left }),
        "phase = {:?}",
        g.phase
    );
}

#[test]
fn test_no_tunneling_fast_puck() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    g.puck.pos = Vec2::new(c.arena.mid_x(), c.arena.mid_y());
    g.puck.vel = Vec2::new(c.tuning.puck_speed_max, 0.0);

    let mut max_x: f32 = g.puck.pos.x;
    for _ in 0..120 {
        g.update(&empty_input(), &c, 1.0 / 60.0);
        // Outside the goal opening, the puck must stay inside the playfield.
        if !c.arena.is_in_goal_y(g.puck.pos.y) {
            assert!(g.puck.pos.x >= c.arena.wall + g.puck.radius - 1e-3);
            assert!(g.puck.pos.x <= c.arena.w - c.arena.wall - g.puck.radius + 1e-3);
        }
        max_x = max_x.max(g.puck.pos.x);
    }
    assert!(max_x > c.arena.mid_x());
}

#[test]
fn test_paddle_confined_to_half() {
    let c = ctx();
    let mut g = game_in_playing(&c);

    // Push J1 to the right as hard as we can for a full second.
    let mut input = empty_input();
    input.keys_down = vec![KeyCode::D];
    for _ in 0..60 {
        g.update(&input, &c, 1.0 / 60.0);
    }
    assert!(
        g.paddles[0].pos.x <= c.arena.mid_x() - g.paddles[0].radius + 1e-3,
        "P1 pos = {:?}",
        g.paddles[0].pos
    );

    // Push J2 to the left as hard as we can.
    let mut input = empty_input();
    input.keys_down = vec![KeyCode::Left];
    for _ in 0..60 {
        g.update(&input, &c, 1.0 / 60.0);
    }
    assert!(
        g.paddles[1].pos.x >= c.arena.mid_x() + g.paddles[1].radius - 1e-3,
        "P2 pos = {:?}",
        g.paddles[1].pos
    );
}

#[test]
fn test_goal_detection_only_in_goal_zone() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    // Puck at the left wall but outside the goal y-range.
    g.puck.pos = Vec2::new(c.arena.wall + g.puck.radius, c.arena.goal_top() - 30.0);
    g.puck.vel = Vec2::new(-500.0, 0.0);

    let mut any_goal = false;
    for _ in 0..30 {
        let ev = g.update(&empty_input(), &c, 1.0 / 60.0);
        any_goal |= ev.iter().any(|e| matches!(e, GameEvent::Goal(_)));
        if any_goal {
            break;
        }
    }
    assert!(!any_goal, "puck was not inside the goal opening");
    assert!(
        g.puck.pos.x >= c.arena.wall + g.puck.radius - 1e-3,
        "puck penetrated the wall at x = {}",
        g.puck.pos.x
    );
}

#[test]
fn test_puck_reflects_off_paddle_at_rest() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    let pad = g.paddles[0];
    // Puck to the LEFT of the paddle, moving right toward it.
    g.puck.pos = Vec2::new(
        pad.pos.x - pad.radius - g.puck.radius + 0.5,
        pad.pos.y,
    );
    g.puck.vel = Vec2::new(500.0, 0.0);

    let speed_before = g.puck.vel.length();
    g.update(&empty_input(), &c, 1.0 / 240.0);

    assert!(g.puck.vel.x < 0.0, "v = {:?}", g.puck.vel);
    assert!(
        (g.puck.vel.length() - speed_before).abs() < 5.0,
        "speed before {}, after {}",
        speed_before,
        g.puck.vel.length()
    );
}

#[test]
fn test_puck_gets_paddle_velocity() {
    let c = ctx();
    let mut g = game_in_playing(&c);

    // Slide paddle 1 hard to the right to build up speed.
    let mut input = empty_input();
    input.keys_down = vec![KeyCode::D];
    for _ in 0..15 {
        g.update(&input, &c, 1.0 / 60.0);
    }
    assert!(g.paddles[0].vel.length() > 100.0);

    // Place the puck in front of the moving paddle.
    let pad = g.paddles[0];
    g.puck.pos = Vec2::new(
        pad.pos.x + pad.radius + g.puck.radius - 0.5,
        pad.pos.y,
    );
    g.puck.vel = Vec2::ZERO;

    // One step with the paddle still moving.
    g.update(&input, &c, 1.0 / 240.0);
    assert!(g.puck.vel.x > 0.0, "v = {:?}", g.puck.vel);
}

#[test]
fn test_pause_freezes_physics() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    g.puck.vel = Vec2::new(500.0, 0.0);
    let before = g.puck.pos;

    // Toggle pause.
    let mut p_input = empty_input();
    p_input.keys_pressed = vec![KeyCode::P];
    g.update(&p_input, &c, 1.0 / 60.0);
    assert!(g.paused);

    // A full second of frames; nothing should move.
    for _ in 0..60 {
        g.update(&empty_input(), &c, 1.0 / 60.0);
    }
    assert!((g.puck.pos - before).length() < 1e-3, "moved while paused");
}

#[test]
fn test_countdown_advances_to_playing() {
    let c = ctx();
    let mut g = Game::new(&c);
    g.update(&input_space(), &c, 0.0);
    assert!(matches!(g.phase, Phase::Countdown { .. }));
    advance_until(&mut g, &c, 600, |p| matches!(p, Phase::Playing), "Playing");
}

#[test]
fn test_rematch_after_match_over() {
    let c = ctx();
    let mut g = game_in_playing(&c);
    g.score.p1 = 3;
    g.score.p2 = 2;
    g.rounds.p1 = 1;
    g.rounds.p2 = 1;
    g.phase = Phase::MatchOver { winner: Side::Left };

    g.update(&input_space(), &c, 0.0);
    assert!(matches!(g.phase, Phase::Countdown { .. }));
    assert_eq!(g.score.p1, 0);
    assert_eq!(g.score.p2, 0);
    assert_eq!(g.rounds.p1, 0);
    assert_eq!(g.rounds.p2, 0);
}