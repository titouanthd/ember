//! Integration scenarios for Simon.
//!
//! These exercise the full `GameWorld` through `tick_playback` and
//! `handle_click`, simulating a play session. They complement the unit
//! tests inside `systems.rs` by running multiple rounds and transitions.

use simon::components::SimonColor;
use simon::systems::{
    handle_click, tick_playback, ClickResult, GameWorld, PlayPhase,
    ELEMENT_GAP_SECS, ELEMENT_LIT_SECS, POST_SEQUENCE_PAUSE_SECS, PRE_ROUND_DELAY_SECS,
};

/// Build a fresh `GameWorld` in a known state (Playing, sequence started).
fn fresh_world(seed: u32) -> GameWorld {
    let mut w = GameWorld::new(seed, 0);
    w.restart();
    w
}

/// Tick playback until the phase becomes `Waiting`, then return.
/// Caps iterations to avoid infinite loops if timers are 0.
fn tick_until_waiting(world: &mut GameWorld) {
    for _ in 0..1000 {
        tick_playback(world, 0.05);
        if matches!(world.phase, PlayPhase::Waiting { .. }) {
            return;
        }
    }
    panic!("phase never reached Waiting");
}

/// Play the entire current sequence correctly, returning the final result.
/// Panics if an intermediate click is not `Correct`.
fn play_current_sequence(world: &mut GameWorld) -> ClickResult {
    let expected = world.sequence.colors.clone();
    let mut last = ClickResult::Ignored;
    for color in expected {
        last = handle_click(world, color);
        if !matches!(last, ClickResult::Correct | ClickResult::RoundComplete) {
            panic!("expected Correct/RoundComplete, got {last:?}");
        }
    }
    last
}

// ============================================================================
// Full playthrough: start → pre-round → showing → waiting → click → round 2
// ============================================================================

#[test]
fn test_full_round_playthrough() {
    let mut w = fresh_world(42);

    // Initial phase: PreRound.
    assert!(matches!(w.phase, PlayPhase::PreRound { .. }));
    assert_eq!(w.sequence.len(), 1);

    // Tick past pre-round, then past showing, until Waiting.
    tick_until_waiting(&mut w);
    assert!(matches!(w.phase, PlayPhase::Waiting { .. }));

    // Play the sequence correctly.
    let result = play_current_sequence(&mut w);
    assert_eq!(result, ClickResult::RoundComplete);
    assert_eq!(w.score, 1);

    // Next round: sequence grows.
    w.next_round();
    assert_eq!(w.sequence.len(), 2);
    assert!(matches!(w.phase, PlayPhase::PreRound { .. }));
}

// ============================================================================
// Clicks during Showing are ignored
// ============================================================================

#[test]
fn test_click_during_showing_is_ignored() {
    let mut w = fresh_world(1);
    // Tick to reach Showing.
    tick_playback(&mut w, PRE_ROUND_DELAY_SECS + 0.01);
    assert!(matches!(w.phase, PlayPhase::Showing { .. }));

    // Click any color — should be ignored.
    let result = handle_click(&mut w, SimonColor::Green);
    assert_eq!(result, ClickResult::Ignored);
    assert_eq!(w.score, 0);
}

// ============================================================================
// Wrong click → GameOver (via best score update)
// ============================================================================

#[test]
fn test_wrong_click_ends_run() {
    let mut w = fresh_world(7);
    tick_until_waiting(&mut w);

    // Force a known sequence.
    w.sequence.colors = vec![SimonColor::Green];
    w.phase = PlayPhase::Waiting {
        expected_index: 0,
        lit_index: None,
        lit_timer: 0.0,
    };

    let result = handle_click(&mut w, SimonColor::Red);
    assert_eq!(result, ClickResult::Wrong);
    // Best is updated to the current score (0 here).
    assert_eq!(w.best, 0);
}

// ============================================================================
// Multi-round sequence growth
// ============================================================================

#[test]
fn test_sequence_grows_across_three_rounds() {
    let mut w = fresh_world(99);
    tick_until_waiting(&mut w);
    play_current_sequence(&mut w);
    assert_eq!(w.score, 1);
    assert_eq!(w.sequence.len(), 1);

    w.next_round();
    tick_until_waiting(&mut w);
    play_current_sequence(&mut w);
    assert_eq!(w.score, 2);
    assert_eq!(w.sequence.len(), 2);

    w.next_round();
    tick_until_waiting(&mut w);
    play_current_sequence(&mut w);
    assert_eq!(w.score, 3);
    assert_eq!(w.sequence.len(), 3);
}

// ============================================================================
// Restart after a wrong click resets state
// ============================================================================

#[test]
fn test_restart_after_game_over_resets() {
    let mut w = fresh_world(13);
    tick_until_waiting(&mut w);
    play_current_sequence(&mut w);
    w.next_round();
    tick_until_waiting(&mut w);

    // Wrong click.
    let expected = w.sequence.colors[0];
    let wrong = if expected == SimonColor::Green {
        SimonColor::Red
    } else {
        SimonColor::Green
    };
    assert_eq!(handle_click(&mut w, wrong), ClickResult::Wrong);

    // Restart.
    w.restart();
    assert_eq!(w.score, 0);
    assert_eq!(w.sequence.len(), 1);
    assert!(matches!(w.phase, PlayPhase::PreRound { .. }));
}

// ============================================================================
// lit_color tracks playback + player flash
// ============================================================================

#[test]
fn test_lit_color_during_full_round() {
    let mut w = fresh_world(5);
    w.sequence.colors = vec![SimonColor::Red];
    w.phase = PlayPhase::PreRound {
        timer: PRE_ROUND_DELAY_SECS,
    };

    // Pre-round: nothing lit.
    assert_eq!(w.lit_color(), None);

    // Tick into Showing: Red should be lit during the lit phase.
    tick_playback(&mut w, PRE_ROUND_DELAY_SECS + ELEMENT_LIT_SECS * 0.5);
    assert!(matches!(w.phase, PlayPhase::Showing { .. }));
    assert_eq!(w.lit_color(), Some(SimonColor::Red));

    // Tick through the gap: nothing lit.
    tick_playback(&mut w, ELEMENT_LIT_SECS);
    // Now we're in gap phase (or later). `lit_color` should be None.
    // (Depending on exact timing, we might already be in Waiting.)
    let lit_in_gap_or_waiting = w.lit_color();
    assert!(
        lit_in_gap_or_waiting.is_none(),
        "expected no lit color after gap, got {lit_in_gap_or_waiting:?}"
    );

    // Finish playback to Waiting.
    tick_playback(&mut w, ELEMENT_GAP_SECS + POST_SEQUENCE_PAUSE_SECS + 0.1);
    assert!(matches!(w.phase, PlayPhase::Waiting { .. }));
}

// ============================================================================
// Player flash: a correct click lights the clicked color briefly
// ============================================================================

#[test]
fn test_player_flash_after_correct_click() {
    let mut w = fresh_world(11);
    w.sequence.colors = vec![SimonColor::Green, SimonColor::Red];
    w.phase = PlayPhase::Waiting {
        expected_index: 0,
        lit_index: None,
        lit_timer: 0.0,
    };

    let result = handle_click(&mut w, SimonColor::Green);
    assert_eq!(result, ClickResult::Correct);
    // The clicked color is briefly lit.
    assert_eq!(w.lit_color(), Some(SimonColor::Green));
}

// ============================================================================
// Determinism: same seed, same sequence
// ============================================================================

#[test]
fn test_determinism_same_seed() {
    let mut a = fresh_world(123);
    let mut b = fresh_world(123);
    for _ in 0..10 {
        a.next_round();
        b.next_round();
    }
    assert_eq!(a.sequence.colors, b.sequence.colors);
}