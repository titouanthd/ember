//! Simon game logic. Sequences, phases, input validation.

use crate::components::SimonColor;
use crate::rng::Rng;

/// Duration (seconds) each sequence element stays lit during playback.
pub const ELEMENT_LIT_SECS: f32 = 0.4;
/// Duration (seconds) each sequence element stays dark between elements.
pub const ELEMENT_GAP_SECS: f32 = 0.2;
/// Duration (seconds) of the delay before the first element of a round
/// is shown, so the player knows playback is starting.
pub const PRE_ROUND_DELAY_SECS: f32 = 0.8;
/// Duration (seconds) of the pause after a full sequence was shown,
/// before accepting input.
pub const POST_SEQUENCE_PAUSE_SECS: f32 = 0.3;
/// Duration (seconds) a button stays lit when the player clicks it.
pub const PLAYER_FLASH_SECS: f32 = 0.15;

/// The current sequence to reproduce. Grows by one each round.
#[derive(Debug, Clone, Default)]
pub struct Sequence {
    pub colors: Vec<SimonColor>,
}

impl Sequence {
    pub fn new() -> Self {
        Self { colors: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Append a random color.
    pub fn extend_random(&mut self, rng: &mut Rng) {
        let idx = rng.next_range(4);
        self.colors.push(SimonColor::from_index(idx).unwrap());
    }

    /// Clear the sequence.
    pub fn reset(&mut self) {
        self.colors.clear();
    }

    pub fn get(&self, i: usize) -> Option<SimonColor> {
        self.colors.get(i).copied()
    }
}

/// What the game is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayPhase {
    /// Waiting for the pre-round delay to elapse.
    PreRound { timer: f32 },
    /// Playing back the sequence. `index` is the element currently lit
    /// (or about to be shown). `lit_timer` counts down the current element's
    /// "on" phase; when it hits 0, we move to a gap and then to the next.
    Showing { index: usize, lit_timer: f32, gap_timer: f32 },
    /// Sequence fully shown, waiting for the player's first input.
    Waiting { expected_index: usize, lit_index: Option<usize>, lit_timer: f32 },
}

impl PlayPhase {
    /// True if the game is currently accepting player clicks.
    pub fn is_accepting_input(&self) -> bool {
        matches!(self, PlayPhase::Waiting { .. })
    }
}

/// The full game state, including the sequence, the phase, and scores.
pub struct GameWorld {
    pub sequence: Sequence,
    pub phase: PlayPhase,
    pub rng: Rng,
    /// Number of colors successfully reproduced in this run.
    pub score: u32,
    pub best: u32,
    /// Seconds since the current game started (used by nothing yet,
    /// kept for future "total time" tracking).
    pub elapsed: f32,
}

impl GameWorld {
    pub fn new(seed: u32, best: u32) -> Self {
        Self {
            sequence: Sequence::new(),
            phase: PlayPhase::PreRound { timer: PRE_ROUND_DELAY_SECS },
            rng: Rng::new(seed),
            score: 0,
            best,
            elapsed: 0.0,
        }
    }

    /// Begin a new game with the current RNG and best score.
    pub fn restart(&mut self) {
        self.sequence.reset();
        self.sequence.extend_random(&mut self.rng);
        self.score = 0;
        self.elapsed = 0.0;
        self.phase = PlayPhase::PreRound { timer: PRE_ROUND_DELAY_SECS };
    }

    /// Begin the next round: extend the sequence and start playback.
    pub fn next_round(&mut self) {
        self.sequence.extend_random(&mut self.rng);
        self.phase = PlayPhase::PreRound { timer: PRE_ROUND_DELAY_SECS };
    }

    /// Which button is currently lit, if any.
    pub fn lit_color(&self) -> Option<SimonColor> {
        match &self.phase {
            PlayPhase::PreRound { .. } => None,
            PlayPhase::Showing { index, lit_timer, .. } => {
                if *lit_timer > 0.0 {
                    self.sequence.get(*index)
                } else {
                    None
                }
            }
            PlayPhase::Waiting { lit_index, lit_timer, .. } => {
                if *lit_timer > 0.0 {
                    lit_index.and_then(SimonColor::from_index)
                } else {
                    None
                }
            }
        }
    }
}

/// Advance the sequence playback. Call every frame while in Playing state.
/// Returns true if the phase transitioned from Showing to Waiting.
/// Advance the sequence playback by `dt` seconds. Loops until `dt` is
/// exhausted, so a large frame delta doesn't skip phase transitions.
///
/// Returns true if the phase transitioned from Showing to Waiting during
/// this call (used by the caller if it wants a "player's turn" event).
pub fn tick_playback(world: &mut GameWorld, dt: f32) -> bool {
    world.elapsed += dt;

    let mut remaining = dt;
    let mut transitioned_to_waiting = false;

    // Loop until the whole dt is consumed. Cap iterations to avoid an
    // infinite loop if a timer is 0 and dt is huge.
    let mut iterations = 0;
    while remaining > 0.0 && iterations < 1000 {
        iterations += 1;

        let phase = std::mem::replace(&mut world.phase, PlayPhase::PreRound { timer: 0.0 });

        match phase {
            PlayPhase::PreRound { timer } => {
                let next = timer - remaining;
                if next <= 0.0 {
                    remaining = -next;
                    // Pre-round finished, start showing first element.
                    if world.sequence.is_empty() {
                        // Nothing to show — shouldn't happen, but guard.
                        world.phase = PlayPhase::Waiting {
                            expected_index: 0,
                            lit_index: None,
                            lit_timer: 0.0,
                        };
                        return transitioned_to_waiting;
                    }
                    world.phase = PlayPhase::Showing {
                        index: 0,
                        lit_timer: ELEMENT_LIT_SECS,
                        gap_timer: 0.0,
                    };
                } else {
                    world.phase = PlayPhase::PreRound { timer: next };
                    remaining = 0.0;
                }
            }
            PlayPhase::Showing { index, lit_timer, gap_timer } => {
                if lit_timer > 0.0 {
                    let next = lit_timer - remaining;
                    if next <= 0.0 {
                        remaining = -next;
                        world.phase = PlayPhase::Showing {
                            index,
                            lit_timer: 0.0,
                            gap_timer: ELEMENT_GAP_SECS,
                        };
                    } else {
                        world.phase = PlayPhase::Showing {
                            index,
                            lit_timer: next,
                            gap_timer,
                        };
                        remaining = 0.0;
                    }
                } else {
                    // In gap phase.
                    let next = gap_timer - remaining;
                    if next <= 0.0 {
                        remaining = -next;
                        let next_index = index + 1;
                        if next_index >= world.sequence.len() {
                            world.phase = PlayPhase::Waiting {
                                expected_index: 0,
                                lit_index: None,
                                lit_timer: POST_SEQUENCE_PAUSE_SECS,
                            };
                            transitioned_to_waiting = true;
                        } else {
                            world.phase = PlayPhase::Showing {
                                index: next_index,
                                lit_timer: ELEMENT_LIT_SECS,
                                gap_timer: 0.0,
                            };
                        }
                    } else {
                        world.phase = PlayPhase::Showing {
                            index,
                            lit_timer: 0.0,
                            gap_timer: next,
                        };
                        remaining = 0.0;
                    }
                }
            }
            PlayPhase::Waiting { expected_index, lit_index, lit_timer } => {
                let next = lit_timer - remaining;
                if next <= 0.0 {
                    remaining = 0.0;
                    world.phase = PlayPhase::Waiting {
                        expected_index,
                        lit_index: None,
                        lit_timer: 0.0,
                    };
                } else {
                    world.phase = PlayPhase::Waiting {
                        expected_index,
                        lit_index,
                        lit_timer: next,
                    };
                    remaining = 0.0;
                }
            }
        }
    }

    transitioned_to_waiting
}

/// Result of a player click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    /// No effect — the game isn't accepting input right now.
    Ignored,
    /// Correct color. The player is still on track.
    Correct,
    /// Wrong color. Game over.
    Wrong,
    /// Correct color, and it was the last one in the sequence. Round complete.
    RoundComplete,
}

/// Handle a click on a color. Only valid when the phase is Waiting.
pub fn handle_click(world: &mut GameWorld, clicked: SimonColor) -> ClickResult {
    let PlayPhase::Waiting { expected_index, .. } = &world.phase else {
        return ClickResult::Ignored;
    };
    let expected_index = *expected_index;

    let expected = match world.sequence.get(expected_index) {
        Some(c) => c,
        None => return ClickResult::Ignored, // shouldn't happen
    };

    if clicked != expected {
        // Wrong — game over.
        world.best = world.best.max(world.score);
        return ClickResult::Wrong;
    }

    // Correct. Flash the button briefly.
    let next_index = expected_index + 1;
    let is_last = next_index >= world.sequence.len();

    if is_last {
        world.score += 1;
        return ClickResult::RoundComplete;
    }

    world.phase = PlayPhase::Waiting {
        expected_index: next_index,
        lit_index: Some(clicked.index()),
        lit_timer: PLAYER_FLASH_SECS,
    };
    ClickResult::Correct
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn fresh_world() -> GameWorld {
        let mut w = GameWorld::new(42, 0);
        w.restart();
        w
    }

    #[test]
    fn test_sequence_starts_with_one() {
        let w = fresh_world();
        assert_eq!(w.sequence.len(), 1);
    }

    #[test]
    fn test_sequence_grows_each_round() {
        let mut w = fresh_world();
        let initial = w.sequence.len();
        w.next_round();
        assert_eq!(w.sequence.len(), initial + 1);
        w.next_round();
        assert_eq!(w.sequence.len(), initial + 2);
    }

    #[test]
    fn test_initial_phase_is_pre_round() {
        let w = fresh_world();
        assert!(matches!(w.phase, PlayPhase::PreRound { .. }));
    }

    #[test]
    fn test_pre_round_transitions_to_showing() {
        let mut w = fresh_world();
        // Tick past the pre-round delay.
        tick_playback(&mut w, PRE_ROUND_DELAY_SECS + 0.01);
        assert!(matches!(w.phase, PlayPhase::Showing { index: 0, .. }));
    }

    #[test]
    fn test_showing_transitions_to_waiting_after_all_elements() {
        let mut w = fresh_world();
        // Simulate enough time to show all elements of a length-1 sequence.
        let total = PRE_ROUND_DELAY_SECS
            + ELEMENT_LIT_SECS
            + ELEMENT_GAP_SECS
            + POST_SEQUENCE_PAUSE_SECS
            + 0.1;
        tick_playback(&mut w, total);
        assert!(matches!(w.phase, PlayPhase::Waiting { .. }));
    }

    #[test]
    fn test_click_wrong_color_returns_wrong() {
        let mut w = fresh_world();
        // Force a known sequence.
        w.sequence.colors = vec![SimonColor::Green];
        w.phase = PlayPhase::Waiting {
            expected_index: 0,
            lit_index: None,
            lit_timer: 0.0,
        };
        let result = handle_click(&mut w, SimonColor::Red);
        assert_eq!(result, ClickResult::Wrong);
    }

    #[test]
    fn test_click_right_color_on_last_returns_round_complete() {
        let mut w = fresh_world();
        w.sequence.colors = vec![SimonColor::Green];
        w.phase = PlayPhase::Waiting {
            expected_index: 0,
            lit_index: None,
            lit_timer: 0.0,
        };
        let result = handle_click(&mut w, SimonColor::Green);
        assert_eq!(result, ClickResult::RoundComplete);
        assert_eq!(w.score, 1);
    }

    #[test]
    fn test_click_right_color_on_non_last_returns_correct() {
        let mut w = fresh_world();
        w.sequence.colors = vec![SimonColor::Green, SimonColor::Red];
        w.phase = PlayPhase::Waiting {
            expected_index: 0,
            lit_index: None,
            lit_timer: 0.0,
        };
        let result = handle_click(&mut w, SimonColor::Green);
        assert_eq!(result, ClickResult::Correct);
        match w.phase {
            PlayPhase::Waiting { expected_index, .. } => assert_eq!(expected_index, 1),
            _ => panic!("expected Waiting"),
        }
    }

    #[test]
    fn test_click_ignored_when_showing() {
        let mut w = fresh_world();
        w.phase = PlayPhase::Showing {
            index: 0,
            lit_timer: 0.1,
            gap_timer: 0.0,
        };
        let result = handle_click(&mut w, SimonColor::Green);
        assert_eq!(result, ClickResult::Ignored);
    }

    #[test]
    fn test_best_updates_on_wrong() {
        let mut w = fresh_world();
        w.score = 7;
        w.best = 3;
        w.sequence.colors = vec![SimonColor::Green];
        w.phase = PlayPhase::Waiting {
            expected_index: 0,
            lit_index: None,
            lit_timer: 0.0,
        };
        handle_click(&mut w, SimonColor::Red);
        assert_eq!(w.best, 7);
    }

    #[test]
    fn test_lit_color_none_in_pre_round() {
        let w = fresh_world();
        assert_eq!(w.lit_color(), None);
    }

    #[test]
    fn test_lit_color_during_showing_lit_timer() {
        let mut w = fresh_world();
        w.sequence.colors = vec![SimonColor::Red];
        w.phase = PlayPhase::Showing {
            index: 0,
            lit_timer: 0.2,
            gap_timer: 0.0,
        };
        assert_eq!(w.lit_color(), Some(SimonColor::Red));
    }

    #[test]
    fn test_lit_color_none_during_gap() {
        let mut w = fresh_world();
        w.sequence.colors = vec![SimonColor::Red];
        w.phase = PlayPhase::Showing {
            index: 0,
            lit_timer: 0.0,
            gap_timer: 0.1,
        };
        assert_eq!(w.lit_color(), None);
    }

    #[test]
    fn test_full_playthrough_of_length_2_sequence() {
        let mut w = GameWorld::new(1, 0);
        w.sequence.colors = vec![SimonColor::Green, SimonColor::Blue];
        w.phase = PlayPhase::Waiting {
            expected_index: 0,
            lit_index: None,
            lit_timer: 0.0,
        };
        assert_eq!(handle_click(&mut w, SimonColor::Green), ClickResult::Correct);
        assert_eq!(handle_click(&mut w, SimonColor::Blue), ClickResult::RoundComplete);
        assert_eq!(w.score, 1);
    }

    #[test]
    fn test_restart_clears_score_and_resets_sequence() {
        let mut w = fresh_world();
        w.score = 15;
        w.sequence.colors = vec![SimonColor::Green; 10];
        w.restart();
        assert_eq!(w.score, 0);
        assert_eq!(w.sequence.len(), 1);
    }

    // Silence the unused Rng import warning in case it's only used in one test.
    #[allow(dead_code)]
    fn _use_rng() {
        let _ = Rng::new(0);
    }
}