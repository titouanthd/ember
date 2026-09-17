//! Game state machine and turn flow.
//!
//! Session 4 scope: Peng and Hu claims on a discard, priority
//! resolution, and the human claim-prompt window. Gang arrives in 4.5.

use ember_core::rng::Rng;
use ember_stdlib::input::Input;

use crate::ai;
use crate::components::{ClaimKind, Meld, Player, Tile};
use crate::config::GameContext;
use crate::hand;
use crate::wall;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Deal { t: f32 },
    AwaitingDraw { player: usize },
    DrawAnim { player: usize, t: f32 },
    AiThinking { player: usize, t: f32 },
    AwaitingDiscard { player: usize },
    AwaitingClaims { discard: Tile, from: usize, t: f32 },
    ClaimAnim { claimer: usize, meld: Meld, t: f32 },
    Hu { winner: usize, method: HuMethod },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HuMethod {
    Zimo,
    Hu { from: usize },
}

/// What one player can do with the current discard.
#[derive(Debug, Clone)]
pub struct ClaimOption {
    pub player: usize,
    pub kinds: Vec<ClaimKind>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    DealComplete,
    Drew { player: usize },
    Discarded { player: usize, tile: Tile },
    Claimed { player: usize, kind: ClaimKind, tile: Tile },
    Hu { player: usize, method: HuMethod },
}

pub const NUM_PLAYERS: usize = 4;
pub const HUMAN_SEAT: usize = 0;

pub struct Game {
    pub phase: Phase,
    pub wall: Vec<Tile>,
    pub players: [Player; NUM_PLAYERS],
    pub turn: usize,
    pub dealer: usize,
    pub rng: Rng,
    /// Set by `human_claim` during the claim window. Cleared on resolve.
    pub pending_human_claim: Option<ClaimKind>,
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
            pending_human_claim: None,
        }
    }

    pub fn update(&mut self, input: &Input, ctx: &GameContext, dt: f32) -> Vec<GameEvent> {
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
                    let hand_ref = &self.players[player].concealed;
                    if !hand_ref.is_empty() {
                        let idx =
                            ai::decide_discard(hand_ref, &self.players[player].melds);
                        let tile = self.players[player].concealed.remove(idx);
                        self.players[player].discards.push(tile);
                        events.push(GameEvent::Discarded { player, tile });
                        self.after_discard(player, tile, ctx);
                    }
                }
            }

            Phase::AwaitingClaims { discard, from, t } => {
                let new_t = t - dt;
                let human_has_opts = self.human_claim_options(discard, from).is_some();
                let human_decided = self.pending_human_claim.is_some();

                // Close the window when time runs out, or immediately
                // when the human has no options (nothing to wait for).
                let close = new_t <= 0.0 || (!human_has_opts && !human_decided);

                if close {
                    let claim = self.resolve_claims(discard, from);
                    self.pending_human_claim = None;
                    self.apply_claim(discard, from, claim, ctx, &mut events);
                } else {
                    self.phase = Phase::AwaitingClaims { discard, from, t: new_t };
                }
            }

            Phase::ClaimAnim { claimer, meld, t } => {
                let new_t = t - dt;
                if new_t <= 0.0 {
                    self.turn = claimer;
                    self.phase = Phase::AwaitingDiscard { player: claimer };
                    let _ = meld;
                } else {
                    self.phase = Phase::ClaimAnim { claimer, meld, t: new_t };
                }
            }

            Phase::Hu { .. } => {
                // Terminal — Session 5 handles scoring and restart.
            }
        }
        let _ = input;
        events
    }

    /// Entry point after any discard. Decides whether to open a claim
    /// window or advance the turn.
    fn after_discard(&mut self, player: usize, tile: Tile, ctx: &GameContext) {
        if self.any_claim_possible(tile, player) {
            self.phase = Phase::AwaitingClaims {
                discard: tile,
                from: player,
                t: ctx.layout.claim_window,
            };
        } else {
            self.advance_turn();
        }
    }

    fn apply_claim(
        &mut self,
        discard: Tile,
        from: usize,
        claim: Option<(usize, ClaimKind)>,
        ctx: &GameContext,
        events: &mut Vec<GameEvent>,
    ) {
        match claim {
            Some((player, ClaimKind::Hu)) => {
                // Pull the discarded tile into the winner's hand.
                if self.players[from].discards.last() == Some(&discard) {
                    self.players[from].discards.pop();
                }
                self.players[player].concealed.push(discard);
                self.players[player].concealed.sort();
                let method = HuMethod::Hu { from };
                self.phase = Phase::Hu { winner: player, method };
                events.push(GameEvent::Hu { player, method });
            }
            Some((player, ClaimKind::Peng)) => {
                if self.players[from].discards.last() == Some(&discard) {
                    self.players[from].discards.pop();
                }
                // Remove 2 copies from the claimer's hand.
                let mut removed = 0;
                self.players[player].concealed.retain(|&t| {
                    if t == discard && removed < 2 {
                        removed += 1;
                        false
                    } else {
                        true
                    }
                });
                let meld = Meld::Peng { tile: discard, from };
                self.players[player].melds.push(meld);
                self.phase = Phase::ClaimAnim {
                    claimer: player,
                    meld,
                    t: ctx.layout.claim_duration,
                };
                events.push(GameEvent::Claimed {
                    player,
                    kind: ClaimKind::Peng,
                    tile: discard,
                });
            }
            None => {
                self.advance_turn();
            }
        }
    }

    fn any_claim_possible(&self, discard: Tile, from: usize) -> bool {
        (0..NUM_PLAYERS).any(|p| {
            p != from && (self.can_peng(p, discard) || self.can_hu(p, discard))
        })
    }

    fn can_peng(&self, player: usize, tile: Tile) -> bool {
        self.players[player]
            .concealed
            .iter()
            .filter(|&&t| t == tile)
            .count()
            >= 2
    }

    fn can_hu(&self, player: usize, tile: Tile) -> bool {
        let mut test = self.players[player].concealed.clone();
        test.push(tile);
        test.sort();
        hand::is_winning_hand(&test, &self.players[player].melds)
    }

    /// What the human can claim on this discard, if anything.
    pub fn human_claim_options(
        &self,
        discard: Tile,
        from: usize,
    ) -> Option<Vec<ClaimKind>> {
        if from == HUMAN_SEAT {
            return None;
        }
        let mut kinds = Vec::new();
        if self.can_hu(HUMAN_SEAT, discard) {
            kinds.push(ClaimKind::Hu);
        }
        if self.can_peng(HUMAN_SEAT, discard) {
            kinds.push(ClaimKind::Peng);
        }
        if kinds.is_empty() { None } else { Some(kinds) }
    }

    /// Called from `main.rs` when the human clicks a claim button.
    pub fn human_claim(&mut self, kind: ClaimKind) -> bool {
        let Phase::AwaitingClaims { discard, from, .. } = self.phase else {
            return false;
        };
        let Some(opts) = self.human_claim_options(discard, from) else {
            return false;
        };
        if !opts.contains(&kind) {
            return false;
        }
        self.pending_human_claim = Some(kind);
        true
    }

    /// Highest-priority claim on the table, or `None`.
    fn resolve_claims(&self, discard: Tile, from: usize) -> Option<(usize, ClaimKind)> {
        let mut claims: Vec<(usize, ClaimKind)> = Vec::new();
        for p in 0..NUM_PLAYERS {
            if p == from {
                continue;
            }
            if p == HUMAN_SEAT {
                if let Some(kind) = self.pending_human_claim {
                    claims.push((p, kind));
                }
                continue;
            }
            if self.can_hu(p, discard) {
                claims.push((p, ClaimKind::Hu));
            } else if self.can_peng(p, discard)
                && ai::decide_claim(
                    &self.players[p].concealed,
                    &self.players[p].melds,
                    discard,
                ) == Some(ClaimKind::Peng)
            {
                claims.push((p, ClaimKind::Peng));
            }
        }

        claims.sort_by_key(|&(p, kind)| {
            let prio = match kind {
                ClaimKind::Hu => 0u8,
                ClaimKind::Peng => 1,
            };
            let dist = (p + NUM_PLAYERS - from) % NUM_PLAYERS;
            (prio, dist)
        });
        claims.into_iter().next()
    }

    /// Discard a tile from the human's hand by index.
    /// `ctx` supplies the claim-window duration for the follow-up phase.
    pub fn human_discard(&mut self, tile_index: usize, ctx: &GameContext) -> Option<Tile> {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return None;
        }
        if tile_index >= self.players[HUMAN_SEAT].concealed.len() {
            return None;
        }
        let tile = self.players[HUMAN_SEAT].concealed.remove(tile_index);
        self.players[HUMAN_SEAT].discards.push(tile);
        self.after_discard(HUMAN_SEAT, tile, ctx);
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

        let discarded = g.human_discard(0, &c).expect("legal discard");

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
        assert!(g.human_discard(0, &ctx()).is_none());
    }

    #[test]
    fn test_human_discard_rejected_when_index_out_of_range() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        assert!(g.human_discard(99, &c).is_none());
    }

    #[test]
    fn test_full_rotation_brings_turn_back_to_human() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.human_discard(0, &c).unwrap();

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

        g.human_discard(0, &c).unwrap();
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
        g.human_discard(0, &c).unwrap();

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
        g.human_discard(0, &c).unwrap();

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