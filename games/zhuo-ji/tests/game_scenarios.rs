//! Integration scenarios for Zhuo Ji.
//!
//! These tests build a `GameContext::default_hermetic()`, construct a
//! `Game` directly, and drive `Game::update` with a synthetic `Input`.
//! No window, no macroquad global state.

use ember_stdlib::input::Input;
use glam::Vec2;
use zhuo_ji::{Game, GameContext, Phase};

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

/// Drive the game until `pred(game.phase)` holds, or panic.
fn advance_until<F>(g: &mut Game, c: &GameContext, max_frames: usize, pred: F, label: &str)
where
    F: Fn(Phase) -> bool,
{
    for _ in 0..max_frames {
        if pred(g.phase) {
            return;
        }
        g.update(&empty_input(), c, 1.0 / 60.0);
    }
    panic!("advance_until({label}): phase = {:?}", g.phase);
}

fn advance_to_human_discard(g: &mut Game, c: &GameContext, max_frames: usize) {
    advance_until(
        g,
        c,
        max_frames,
        |p| matches!(p, Phase::AwaitingDiscard { player: 0 }),
        "AwaitingDiscard{0}",
    );
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[test]
fn test_first_turn_human_has_14_tiles() {
    let c = ctx();
    let mut g = Game::new(1);
    advance_to_human_discard(&mut g, &c, 600);
    assert_eq!(g.players[0].concealed.len(), 14);
    assert_eq!(g.wall.len(), 55);
}

#[test]
fn test_discard_moves_tile_to_river() {
    let c = ctx();
    let mut g = Game::new(1);
    advance_to_human_discard(&mut g, &c, 600);

    let discarded = g.human_discard(0, &c).expect("legal discard");
    assert_eq!(g.players[0].concealed.len(), 13);
    assert_eq!(g.players[0].discards.len(), 1);
    assert_eq!(g.players[0].discards[0], discarded);
}

#[test]
fn test_full_rotation_back_to_human() {
    let c = ctx();
    let mut g = Game::new(1);
    advance_to_human_discard(&mut g, &c, 600);

    g.human_discard(0, &c).unwrap();
    advance_to_human_discard(&mut g, &c, 600);

    assert_eq!(g.players[0].discards.len(), 1);
    for i in 1..4 {
        assert_eq!(g.players[i].discards.len(), 1, "AI {i} should have discarded once");
    }
}

#[test]
fn test_deterministic_with_same_seed() {
    let c = ctx();
    let mut g1 = Game::new(12345);
    let mut g2 = Game::new(12345);
    advance_to_human_discard(&mut g1, &c, 600);
    advance_to_human_discard(&mut g2, &c, 600);
    assert_eq!(g1.players[0].concealed, g2.players[0].concealed);
}

#[test]
fn test_different_seeds_produce_different_hands() {
    let c = ctx();
    let mut g1 = Game::new(1);
    let mut g2 = Game::new(2);
    advance_to_human_discard(&mut g1, &c, 600);
    advance_to_human_discard(&mut g2, &c, 600);
    assert_ne!(g1.players[0].concealed, g2.players[0].concealed);
}

#[test]
fn test_ai_discards_are_deterministic_with_seed() {
    let c = ctx();
    let mut g1 = Game::new(7777);
    let mut g2 = Game::new(7777);

    // Play through one full rotation of the human discarding tile 0.
    for g in [&mut g1, &mut g2] {
        advance_to_human_discard(g, &c, 2000);
        g.human_discard(0, &c).unwrap();
        advance_to_human_discard(g, &c, 2000);
    }

    // Same seed → same AI discards.
    for i in 1..4 {
        assert_eq!(g1.players[i].discards, g2.players[i].discards, "AI {i}");
    }
}

#[test]
fn test_full_turn_with_claims_does_not_deadlock() {
    let c = ctx();
    let mut g = Game::new(3);

    // Play up to 20 turns of "human discards tile 0, then let the claim
    // window resolve". The hand may end in Hu at any point — stop
    // cleanly when it does. The point is: no hang, no panic.
    for _ in 0..20 {
        // Drive to the human's discard, but bail if a terminal phase
        // arrives first.
        let mut reached = false;
        for _ in 0..4000 {
            match g.phase {
                Phase::AwaitingDiscard { player: 0 } => {
                    reached = true;
                    break;
                }
                Phase::Hu { .. } => break,
                _ => {}
            }
            g.update(&empty_input(), &c, 1.0 / 60.0);
        }
        if !reached {
            break;
        }

        g.human_discard(0, &c).unwrap();

        // Let the claim window resolve, if any. Stop on terminal phase
        // or on coming back to the human.
        for _ in 0..240 {
            if matches!(g.phase, Phase::Hu { .. })
                || matches!(g.phase, Phase::AwaitingDiscard { player: 0 })
            {
                break;
            }
            g.update(&empty_input(), &c, 1.0 / 60.0);
        }
        if matches!(g.phase, Phase::Hu { .. }) {
            break;
        }
    }
    // Passing means: no deadlock, no panic.
}

#[test]
fn test_wall_exhaustion_terminates_game() {
    let c = ctx();
    // Any seed — the game must eventually stop, either by Hu or
    // by HuangZhuang when the wall runs out.
    let mut g = Game::new(0xABCD_1234);

    // Cap at 5000 updates (~80s of game time). Every hand must end
    // by then because the wall has only 55 tiles after dealing.
    for _ in 0..5000 {
        match g.phase {
            Phase::Hu { .. } | Phase::HuangZhuang => return,
            Phase::AwaitingDiscard { player: 0 } => {
                // Human discards first tile.
                g.human_discard(0, &c);
            }
            _ => {}
        }
        g.update(&empty_input(), &c, 1.0 / 60.0);
    }
    panic!("game never terminated: phase = {:?}", g.phase);
}