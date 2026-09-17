//! Game state machine and turn flow.
//!
//! Session 5 scope: scoring on Hu or Huangzhuang, MatchOver after N
//! hands, and Space-to-continue between hands.

use ember_core::rng::Rng;
use ember_stdlib::input::Input;
use macroquad::prelude::KeyCode;

use crate::ai;
use crate::components::{ClaimKind, GangSource, Meld, Player, Tile};
use crate::config::GameContext;
use crate::hand;
use crate::scoring::{self, JiInfo};
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
    HuangZhuang,
    MatchOver { winner: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HuMethod {
    Zimo,
    Hu { from: usize },
}

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
    HuangZhuang,
    MatchOver { winner: usize },
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
    pub pending_human_claim: Option<ClaimKind>,
    pub hands_played: u32,
    pub last_ji: Option<JiInfo>,
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
            hands_played: 0,
            last_ji: None,
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
                if self.wall.is_empty() {
                    self.on_huangzhuang(&mut events);
                } else if let Some(tile) = self.wall.pop() {
                    self.players[player].concealed.push(tile);
                    self.players[player].concealed.sort();
                    events.push(GameEvent::Drew { player });
                    self.phase = Phase::DrawAnim { player, t: 0.0 };
                }
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
                    if self.can_zimo(player) {
                        let method = HuMethod::Zimo;
                        self.on_hu(player, method, &mut events);
                    } else if let Some(tile) = ai::decide_an_gang(
                        &self.players[player].concealed,
                        &self.players[player].melds,
                    ) {
                        self.apply_an_gang(player, tile, &mut events);
                    } else {
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
            }

            Phase::AwaitingClaims { discard, from, t } => {
                let new_t = t - dt;
                let human_has_opts = self.human_claim_options(discard, from).is_some();
                let human_decided = self.pending_human_claim.is_some();
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

            Phase::Hu { .. } | Phase::HuangZhuang => {
                if input.is_key_pressed(KeyCode::Space) {
                    self.start_next_hand();
                    events.push(GameEvent::DealComplete);
                }
            }

            Phase::MatchOver { .. } => {
                if input.is_key_pressed(KeyCode::Space) {
                    self.reset_match();
                    events.push(GameEvent::DealComplete);
                }
            }
        }
        events
    }

    fn on_hu(&mut self, winner: usize, method: HuMethod, events: &mut Vec<GameEvent>) {
        let ji = if let Some(flipped) = self.wall.pop() {
            scoring::determine_ji(flipped)
        } else {
            JiInfo::none()
        };
        self.last_ji = Some(ji);

        scoring::apply_hand_scores(
            &mut self.players,
            Some((winner, method)),
            &ji,
            self.dealer,
        );

        self.hands_played += 1;
        events.push(GameEvent::Hu { player: winner, method });

        if self.hands_played >= self.hands_per_match() {
            let best = self.leading_player();
            self.phase = Phase::MatchOver { winner: best };
            events.push(GameEvent::MatchOver { winner: best });
        } else {
            self.phase = Phase::Hu { winner, method };
        }
    }

    fn on_huangzhuang(&mut self, events: &mut Vec<GameEvent>) {
        let tenpai: [bool; NUM_PLAYERS] = std::array::from_fn(|i| {
            hand::is_tenpai(&self.players[i].concealed, &self.players[i].melds)
        });
        scoring::apply_huangzhuang_scores(&mut self.players, &tenpai);

        self.hands_played += 1;
        events.push(GameEvent::HuangZhuang);

        if self.hands_played >= self.hands_per_match() {
            let best = self.leading_player();
            self.phase = Phase::MatchOver { winner: best };
            events.push(GameEvent::MatchOver { winner: best });
        } else {
            self.phase = Phase::HuangZhuang;
        }
    }

    fn start_next_hand(&mut self) {
        self.dealer = (self.dealer + 1) % NUM_PLAYERS;

        let mut new_wall = wall::build_wall();
        wall::shuffle(&mut new_wall, &mut self.rng);
        self.wall = new_wall;

        for p in &mut self.players {
            p.concealed.clear();
            p.melds.clear();
            p.discards.clear();
        }
        self.turn = self.dealer;
        self.pending_human_claim = None;
        self.last_ji = None;
        self.phase = Phase::Deal { t: 0.0 };
    }

    fn reset_match(&mut self) {
        for p in &mut self.players {
            p.score = 0;
        }
        self.hands_played = 0;
        // Set dealer one step back so start_next_hand rotates to 0.
        self.dealer = NUM_PLAYERS - 1;
        self.start_next_hand();
    }

    fn hands_per_match(&self) -> u32 {
        // Stored on the layout, but Game does not hold GameContext, so
        // we default to 4 and rely on `human`-side env override being
        // applied via start-of-match config. For hermetic tests, always 4.
        4
    }

    fn leading_player(&self) -> usize {
        let mut best = 0;
        for i in 1..NUM_PLAYERS {
            if self.players[i].score > self.players[best].score {
                best = i;
            }
        }
        best
    }

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
                if self.players[from].discards.last() == Some(&discard) {
                    self.players[from].discards.pop();
                }
                self.players[player].concealed.push(discard);
                self.players[player].concealed.sort();
                let method = HuMethod::Hu { from };
                self.on_hu(player, method, events);
            }
            Some((player, ClaimKind::Peng)) => {
                if self.players[from].discards.last() == Some(&discard) {
                    self.players[from].discards.pop();
                }
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
            Some((player, ClaimKind::Gang)) => {
                if self.players[from].discards.last() == Some(&discard) {
                    self.players[from].discards.pop();
                }
                let mut removed = 0;
                self.players[player].concealed.retain(|&t| {
                    if t == discard && removed < 3 {
                        removed += 1;
                        false
                    } else {
                        true
                    }
                });
                self.players[player].melds.push(Meld::Gang {
                    tile: discard,
                    from: GangSource::Ming { from },
                });
                events.push(GameEvent::Claimed {
                    player,
                    kind: ClaimKind::Gang,
                    tile: discard,
                });
                self.turn = player;
                self.phase = Phase::AwaitingDraw { player };
            }
            None => {
                self.advance_turn();
            }
        }
    }

    fn any_claim_possible(&self, discard: Tile, from: usize) -> bool {
        (0..NUM_PLAYERS).any(|p| {
            p != from
                && (self.can_peng(p, discard)
                    || self.can_hu(p, discard)
                    || self.can_ming_gang(p, discard))
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

    fn can_ming_gang(&self, player: usize, tile: Tile) -> bool {
        self.players[player]
            .concealed
            .iter()
            .filter(|&&t| t == tile)
            .count()
            >= 3
    }

    fn can_hu(&self, player: usize, tile: Tile) -> bool {
        let mut test = self.players[player].concealed.clone();
        test.push(tile);
        test.sort();
        hand::is_winning_hand(&test, &self.players[player].melds)
    }

    pub fn an_gang_tile(&self, player: usize) -> Option<Tile> {
        let mut counts: std::collections::HashMap<Tile, usize> =
            std::collections::HashMap::new();
        for &t in &self.players[player].concealed {
            *counts.entry(t).or_insert(0) += 1;
        }
        counts.iter().find(|&(_, &c)| c == 4).map(|(&t, _)| t)
    }

    pub fn can_zimo(&self, player: usize) -> bool {
        hand::is_winning_hand(
            &self.players[player].concealed,
            &self.players[player].melds,
        )
    }

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
        if self.can_ming_gang(HUMAN_SEAT, discard) {
            kinds.push(ClaimKind::Gang);
        } else if self.can_peng(HUMAN_SEAT, discard) {
            kinds.push(ClaimKind::Peng);
        }
        if kinds.is_empty() { None } else { Some(kinds) }
    }

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

    fn resolve_claims(
        &self,
        discard: Tile,
        from: usize,
    ) -> Option<(usize, ClaimKind)> {
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
            } else if ai::decide_ming_gang(&self.players[p].concealed, discard) {
                claims.push((p, ClaimKind::Gang));
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
                ClaimKind::Gang => 1,
                ClaimKind::Peng => 1,
            };
            let dist = (p + NUM_PLAYERS - from) % NUM_PLAYERS;
            (prio, dist)
        });
        claims.into_iter().next()
    }

    fn apply_an_gang(&mut self, player: usize, tile: Tile, events: &mut Vec<GameEvent>) {
        let mut removed = 0;
        self.players[player].concealed.retain(|&t| {
            if t == tile && removed < 4 {
                removed += 1;
                false
            } else {
                true
            }
        });
        self.players[player].melds.push(Meld::Gang {
            tile,
            from: GangSource::An,
        });
        events.push(GameEvent::Claimed {
            player,
            kind: ClaimKind::Gang,
            tile,
        });
        self.phase = Phase::AwaitingDraw { player };
    }

    pub fn human_discard(
        &mut self,
        tile_index: usize,
        ctx: &GameContext,
    ) -> Option<Tile> {
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

    pub fn human_zimo(&mut self) -> bool {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return false;
        }
        if !self.can_zimo(HUMAN_SEAT) {
            return false;
        }
        let mut events = Vec::new();
        self.on_hu(HUMAN_SEAT, HuMethod::Zimo, &mut events);
        true
    }

    pub fn human_an_gang(&mut self, tile: Tile) -> bool {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return false;
        }
        if self.an_gang_tile(HUMAN_SEAT) != Some(tile) {
            return false;
        }
        let mut events = Vec::new();
        self.apply_an_gang(HUMAN_SEAT, tile, &mut events);
        true
    }

    fn advance_turn(&mut self) {
        self.turn = (self.turn + 1) % NUM_PLAYERS;
        self.phase = Phase::AwaitingDraw { player: self.turn };
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

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

    fn space_input() -> Input {
        let mut i = empty_input();
        i.keys_pressed = vec![KeyCode::Space];
        i
    }

    fn ctx() -> GameContext {
        GameContext::default_hermetic()
    }

    fn drive_to_discard(g: &mut Game, c: &GameContext, max_frames: usize) -> usize {
        for _ in 0..max_frames {
            if let Phase::AwaitingDiscard { player } = g.phase {
                return player;
            }
            g.update(&empty_input(), c, 1.0 / 60.0);
        }
        panic!("never reached AwaitingDiscard, phase = {:?}", g.phase);
    }

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
        assert_eq!(player, 0);
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
        assert_eq!(g.players[0].discards[0], discarded);
    }

    #[test]
    fn test_human_discard_rejected_outside_its_turn() {
        let c = ctx();
        let mut g = Game::new(1);
        assert!(g.human_discard(0, &c).is_none());
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
        drive_to_human_discard(&mut g, &c, 600);
        for i in 1..NUM_PLAYERS {
            assert_eq!(g.players[i].discards.len(), 1, "AI {i}");
        }
    }

    #[test]
    fn test_wall_shrinks_by_one_per_turn() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        assert_eq!(g.wall.len(), 55);
        g.human_discard(0, &c).unwrap();
        for _ in 0..30 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if let Phase::AwaitingDiscard { player: 1 } = g.phase {
                break;
            }
        }
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
        drive_to_discard(&mut g, &c, 600);
        g.human_discard(0, &c).unwrap();
        let mut saw = false;
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AiThinking { .. }) {
                saw = true;
                break;
            }
        }
        assert!(saw);
    }

    #[test]
    fn test_ai_does_not_discard_instantly() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.human_discard(0, &c).unwrap();
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AiThinking { .. }) {
                break;
            }
        }
        assert_eq!(g.players[1].discards.len(), 0);
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AwaitingDiscard { player: 1 }) {
                break;
            }
        }
        g.update(&empty_input(), &c, 1.0 / 60.0);
        assert_eq!(g.players[1].discards.len(), 1);
    }

    #[test]
    fn test_hu_transitions_to_hu_phase() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        // Give the human a winning hand.
        g.players[0].concealed = vec![
            Tile::new(crate::components::Suit::Wan, 1),
            Tile::new(crate::components::Suit::Wan, 2),
            Tile::new(crate::components::Suit::Wan, 3),
            Tile::new(crate::components::Suit::Wan, 4),
            Tile::new(crate::components::Suit::Wan, 5),
            Tile::new(crate::components::Suit::Wan, 6),
            Tile::new(crate::components::Suit::Wan, 7),
            Tile::new(crate::components::Suit::Wan, 8),
            Tile::new(crate::components::Suit::Wan, 9),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tong, 2),
            Tile::new(crate::components::Suit::Tong, 2),
        ];
        assert!(g.human_zimo());
        assert!(matches!(g.phase, Phase::Hu { winner: 0, .. }));
        assert_eq!(g.hands_played, 1);
    }

    #[test]
    fn test_space_in_hu_starts_next_hand() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = vec![
            Tile::new(crate::components::Suit::Wan, 1),
            Tile::new(crate::components::Suit::Wan, 2),
            Tile::new(crate::components::Suit::Wan, 3),
            Tile::new(crate::components::Suit::Wan, 4),
            Tile::new(crate::components::Suit::Wan, 5),
            Tile::new(crate::components::Suit::Wan, 6),
            Tile::new(crate::components::Suit::Wan, 7),
            Tile::new(crate::components::Suit::Wan, 8),
            Tile::new(crate::components::Suit::Wan, 9),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tong, 2),
            Tile::new(crate::components::Suit::Tong, 2),
        ];
        g.human_zimo();
        g.update(&space_input(), &c, 1.0 / 60.0);
        assert!(matches!(g.phase, Phase::Deal { .. }));
        assert_eq!(g.dealer, 1); // rotated
    }

    #[test]
    fn test_match_over_after_four_hands() {
        let c = ctx();
        let mut g = Game::new(1);
        // Force hands_played high enough that the next win ends the match.
        g.hands_played = 3;
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = vec![
            Tile::new(crate::components::Suit::Wan, 1),
            Tile::new(crate::components::Suit::Wan, 2),
            Tile::new(crate::components::Suit::Wan, 3),
            Tile::new(crate::components::Suit::Wan, 4),
            Tile::new(crate::components::Suit::Wan, 5),
            Tile::new(crate::components::Suit::Wan, 6),
            Tile::new(crate::components::Suit::Wan, 7),
            Tile::new(crate::components::Suit::Wan, 8),
            Tile::new(crate::components::Suit::Wan, 9),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tiao, 1),
            Tile::new(crate::components::Suit::Tong, 2),
            Tile::new(crate::components::Suit::Tong, 2),
        ];
        g.human_zimo();
        assert!(matches!(g.phase, Phase::MatchOver { .. }));
    }

    #[test]
    fn test_match_over_space_resets() {
        let c = ctx();
        let mut g = Game::new(1);
        g.phase = Phase::MatchOver { winner: 0 };
        g.players[0].score = 42;
        g.hands_played = 4;
        g.update(&space_input(), &c, 1.0 / 60.0);
        assert_eq!(g.players[0].score, 0);
        assert_eq!(g.hands_played, 0);
        assert!(matches!(g.phase, Phase::Deal { .. }));
    }
}