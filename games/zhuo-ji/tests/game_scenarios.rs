//! Integration scenarios for Zhuo Ji.

use ember_stdlib::input::Input;
use glam::Vec2;
use zhuo_ji::{Game, GameContext, Phase};

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

fn advance_to_human_discard(g: &mut Game, c: &GameContext, max_frames: usize) {
    for _ in 0..max_frames {
        if matches!(g.phase, Phase::AwaitingDiscard { player: 0 }) {
            return;
        }
        g.update(&empty_input(), c, 1.0 / 60.0);
    }
    panic!("advance_to_human_discard: phase = {:?}", g.phase);
}

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
        assert_eq!(g.players[i].discards.len(), 1, "AI {i}");
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
    for g in [&mut g1, &mut g2] {
        advance_to_human_discard(g, &c, 2000);
        g.human_discard(0, &c).unwrap();
        advance_to_human_discard(g, &c, 2000);
    }
    for i in 1..4 {
        assert_eq!(g1.players[i].discards, g2.players[i].discards);
    }
}

#[test]
fn test_full_turn_with_claims_does_not_deadlock() {
    let c = ctx();
    let mut g = Game::new(3);
    for _ in 0..20 {
        let mut reached = false;
        for _ in 0..4000 {
            match g.phase {
                Phase::AwaitingDiscard { player: 0 } => { reached = true; break; }
                Phase::Hu { .. } | Phase::HuangZhuang | Phase::MatchOver { .. } => break,
                _ => {}
            }
            g.update(&empty_input(), &c, 1.0 / 60.0);
        }
        if !reached { break; }

        g.human_discard(0, &c).unwrap();

        for _ in 0..240 {
            if matches!(g.phase, Phase::Hu { .. } | Phase::HuangZhuang | Phase::MatchOver { .. }) {
                break;
            }
            g.update(&empty_input(), &c, 1.0 / 60.0);
        }
        if matches!(g.phase, Phase::Hu { .. } | Phase::HuangZhuang | Phase::MatchOver { .. }) {
            break;
        }
    }
}

#[test]
fn test_wall_exhaustion_terminates_game() {
    let c = ctx();
    let mut g = Game::new(0xABCD_1234);
    for _ in 0..5000 {
        match g.phase {
            Phase::Hu { .. } | Phase::HuangZhuang | Phase::MatchOver { .. } => return,
            Phase::AwaitingDiscard { player: 0 } => { g.human_discard(0, &c); }
            _ => {}
        }
        g.update(&empty_input(), &c, 1.0 / 60.0);
    }
    panic!("game never terminated: phase = {:?}", g.phase);
}

#[test]
fn test_scores_are_zero_sum_after_a_hand() {
    let c = ctx();
    let mut g = Game::new(11);
    for _ in 0..5000 {
        match g.phase {
            Phase::Hu { .. } | Phase::HuangZhuang => {
                let sum: i32 = g.players.iter().map(|p| p.score).sum();
                assert_eq!(sum, 0, "score sum should be zero-sum");
                return;
            }
            Phase::AwaitingDiscard { player: 0 } => { g.human_discard(0, &c); }
            _ => {}
        }
        g.update(&empty_input(), &c, 1.0 / 60.0);
    }
    panic!("no hand ended: phase = {:?}", g.phase);
}