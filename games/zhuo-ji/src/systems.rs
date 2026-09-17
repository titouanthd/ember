//! Game state machine and turn flow.
//!
//! Session 1 scope: Deal → AwaitingDraw → DrawAnim → AwaitingDiscard,
//! rotating through 4 seats. The human plays seat 0 via mouse clicks;
//! the other 3 seats auto-discard the tile they just drew (placeholder
//! opponent, replaced by the real AI in Session 3).
//!
//! No claims, no scoring, no win detection yet.

use ember_core::rng::Rng;
use ember_stdlib::input::Input;

use crate::components::{Player, Tile};
use crate::config::GameContext;
use crate::wall;
use crate::ai;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Initial deal animation. Tiles are distributed at the end.
    Deal { t: f32 },
    /// A player is about to draw. The draw happens on the next update.
    AwaitingDraw { player: usize },
    /// Short slide-in animation after a draw.
    DrawAnim { player: usize, t: f32 },
    /// AI seat is deciding what to discard. `t` counts down from
    /// `ai_think_duration`; when it reaches 0, we move to AwaitingDiscard.
    AiThinking { player: usize, t: f32 },
    /// A player must act (discard, tsumo, or kan). Only discard is wired.
    AwaitingDiscard { player: usize },
}

/// Events emitted by one `Game::update`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    DealComplete,
    Drew { player: usize },
    Discarded { player: usize, tile: Tile },
}

/// Number of seats at the table.
pub const NUM_PLAYERS: usize = 4;

/// Human seat. Always seat 0.
pub const HUMAN_SEAT: usize = 0;

/// Full game state.
pub struct Game {
    pub phase: Phase,
    pub wall: Vec<Tile>,
    pub players: [Player; NUM_PLAYERS],
    pub turn: usize,
    pub dealer: usize,
    pub rng: Rng,
}

impl Game {
    pub fn new(seed: u32) -> Self {
        let mut wall = wall::build_wall();
        let mut rng = Rng::new(seed);
        wall::shuffle(&mut wall, &mut rng);
        Self {
            phase: Phase::Deal { t: 0.0 },
            wall,
            players: [
                Player::new(false),
                Player::new(true),
                Player::new(true),
                Player::new(true),
            ],
            turn: 0,
            dealer: 0,
            rng,
        }
    }

    /// Advance the game by `dt` seconds and return events.
    pub fn update(
        &mut self,
        input: &Input,
        ctx: &GameContext,
        dt: f32,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let phase = self.phase;
        match phase {
            Phase::Deal { t } => {
                let new_t = t + dt;
                if new_t >= ctx.layout.deal_duration {
                    wall::deal(&mut self.wall, &mut self.players, self.dealer);
                    self.turn = self.dealer;
                    self.phase = Phase::AwaitingDiscard { player: self.dealer };
                    events.push(GameEvent::DealComplete);
                } else {
                    self.phase = Phase::Deal { t: new_t };
                }
            }

            Phase::AwaitingDraw { player } => {
                if let Some(tile) = self.wall.pop() {
                    self.players[player].concealed.push(tile);
                    self.players[player].concealed.sort();
                    events.push(GameEvent::Drew { player });
                }
                self.phase = Phase::DrawAnim { player, t: 0.0 };
            }

            Phase::DrawAnim { player, t } => {
                let new_t = t + dt;
                if new_t >= ctx.layout.draw_duration {
                    if player == HUMAN_SEAT {
                        self.phase = Phase::AwaitingDiscard { player };
                    } else {
                        self.phase = Phase::AiThinking {
                            player,
                            t: ctx.layout.ai_think_duration,
                        };
                    }
                } else {
                    self.phase = Phase::DrawAnim { player, t: new_t };
                }
            }

            Phase::AiThinking { player, t } => {
                let new_t = t - dt;
                if new_t <= 0.0 {
                    self.phase = Phase::AwaitingDiscard { player };
                } else {
                    self.phase = Phase::AiThinking { player, t: new_t };
                }
            }

            Phase::AwaitingDiscard { player } => {
                if player != HUMAN_SEAT {
                    let hand = &self.players[player].concealed;
                    if !hand.is_empty() {
                        let idx = ai::decide_discard(hand, &self.players[player].melds);
                        let tile = self.players[player].concealed.remove(idx);
                        self.players[player].discards.push(tile);
                        events.push(GameEvent::Discarded { player, tile });
                        self.advance_turn();
                    }
                }
            }
        }
        let _ = input; // reserved for Session 4 (claim buttons)
        events
    }

    /// Discard a tile from the current human player's hand by index.
    /// Returns the discarded tile if the action was legal, `None` otherwise.
    pub fn human_discard(&mut self, tile_index: usize) -> Option<Tile> {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return None;
        }
        if tile_index >= self.players[HUMAN_SEAT].concealed.len() {
            return None;
        }
        let tile = self.players[HUMAN_SEAT].concealed.remove(tile_index);
        self.players[HUMAN_SEAT].discards.push(tile);
        self.advance_turn();
        Some(tile)
    }

    fn advance_turn(&mut self) {
        self.turn = (self.turn + 1) % NUM_PLAYERS;
        self.phase = Phase::AwaitingDraw { player: self.turn };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

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

    fn ctx() -> GameContext {
        GameContext::default_hermetic()
    }

    /// Drive the game until we reach `AwaitingDiscard` and return who it is.
    fn drive_to_discard(g: &mut Game, c: &GameContext, max_frames: usize) -> usize {
        for _ in 0..max_frames {
            if let Phase::AwaitingDiscard { player } = g.phase {
                return player;
            }
            g.update(&empty_input(), c, 1.0 / 60.0);
        }
        panic!("never reached AwaitingDiscard, phase = {:?}", g.phase);
    }

    /// Drive until it is specifically the human's discard window.
    /// Used to loop past the three AI seats in a full rotation.
    fn drive_to_human_discard(g: &mut Game, c: &GameContext, max_frames: usize) {
        for _ in 0..max_frames {
            if let Phase::AwaitingDiscard { player: HUMAN_SEAT } = g.phase {
                return;
            }
            g.update(&empty_input(), c, 1.0 / 60.0);
        }
        panic!("never reached human discard, phase = {:?}", g.phase);
    }

    #[test]
    fn test_new_game_starts_in_deal_with_full_wall() {
        let g = Game::new(1);
        assert!(matches!(g.phase, Phase::Deal { .. }));
        assert_eq!(g.wall.len(), 108);
        for p in &g.players {
            assert_eq!(p.concealed.len(), 0);
        }
    }

    #[test]
    fn test_deal_transitions_out_of_deal_phase() {
        let c = ctx();
        let mut g = Game::new(1);
        g.update(&empty_input(), &c, c.layout.deal_duration + 0.1);
        assert!(!matches!(g.phase, Phase::Deal { .. }));
    }

    #[test]
    fn test_dealer_reaches_discard_with_14_tiles() {
        let c = ctx();
        let mut g = Game::new(1);
        let player = drive_to_discard(&mut g, &c, 600);
        assert_eq!(player, 0, "dealer is seat 0");
        assert_eq!(g.players[0].concealed.len(), 14);
        assert_eq!(g.wall.len(), 108 - 52 - 1);
    }

    #[test]
    fn test_human_discard_moves_tile_to_river() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);

        let discarded = g.human_discard(0).expect("legal discard");
        assert_eq!(g.players[0].concealed.len(), 13);
        assert_eq!(g.players[0].discards.len(), 1);
        assert_eq!(g.players[0].discards[0], discarded);
        assert_eq!(g.turn, 1);
    }

    #[test]
    fn test_human_discard_rejected_outside_its_turn() {
        let mut g = Game::new(1);
        // Still in Deal.
        assert!(matches!(g.phase, Phase::Deal { .. }));
        assert!(g.human_discard(0).is_none());
    }

    #[test]
    fn test_human_discard_rejected_when_index_out_of_range() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        assert!(g.human_discard(99).is_none());
    }

    #[test]
    fn test_full_rotation_brings_turn_back_to_human() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.human_discard(0).unwrap();

        // Drive through the three AI seats and back to human.
        drive_to_human_discard(&mut g, &c, 600);

        // Each AI discarded exactly once.
        for i in 1..NUM_PLAYERS {
            assert_eq!(g.players[i].discards.len(), 1, "AI {i} discarded");
        }
        // Human discarded exactly once (the explicit call above).
        assert_eq!(g.players[0].discards.len(), 1);
    }

    #[test]
    fn test_wall_shrinks_by_one_per_turn() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        // After deal: 108 - 52 - 1 = 55.
        assert_eq!(g.wall.len(), 55);

        g.human_discard(0).unwrap();
        // Drive until AI-1 has drawn.
        for _ in 0..30 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if let Phase::AwaitingDiscard { player: 1 } = g.phase {
                break;
            }
        }
        // AI-1 drew one tile → 54.
        assert_eq!(g.wall.len(), 54);
    }

    #[test]
    fn test_hands_remain_sorted_after_draws_and_discards() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        for p in &g.players {
            let mut sorted = p.concealed.clone();
            sorted.sort();
            assert_eq!(p.concealed, sorted);
        }
    }

    #[test]
    fn test_ai_thinks_before_discarding() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);       // human 1st turn
        g.human_discard(0).unwrap();

        // Drive until we see AiThinking.
        let mut saw_thinking = false;
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AiThinking { .. }) {
                saw_thinking = true;
                break;
            }
        }
        assert!(saw_thinking, "AI should enter AiThinking, phase = {:?}", g.phase);
    }

    #[test]
    fn test_ai_does_not_discard_instantly() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.human_discard(0).unwrap();

        // Advance just to the start of AiThinking.
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AiThinking { .. }) {
                break;
            }
        }
        // AI-1 has not discarded yet.
        assert_eq!(g.players[1].discards.len(), 0);

        // Wait one full think duration. Now it should.
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AwaitingDiscard { player: 1 }) {
                break;
            }
        }
        g.update(&empty_input(), &c, 1.0 / 60.0);
        assert_eq!(g.players[1].discards.len(), 1);
    }
}