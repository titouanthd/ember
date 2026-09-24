//! Game state machine and turn flow.

use ember_core::rng::Rng;
use ember_stdlib::input::Input;
use macroquad::prelude::KeyCode;

use crate::ai;
use crate::components::{ClaimKind, GangSource, Meld, Player, Tile};
use crate::config::GameContext;
use crate::hand;
use crate::hand_summary::HandSnapshot;
use crate::scoring::{self, HandBreakdown, JiInfo};
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

pub const DEFAULT_MATCH_LENGTH: u32 = 16;
pub const MATCH_EXTENSION: u32 = 16;

pub struct Game {
    pub phase: Phase,
    pub wall: Vec<Tile>,
    pub players: [Player; NUM_PLAYERS],
    pub turn: usize,
    pub dealer: usize,
    pub rng: Rng,
    pub pending_human_claim: Option<ClaimKind>,
    pub hands_played: u32,
    pub match_length: u32,
    pub last_ji: Option<JiInfo>,
    pub turn_timer: f32,
    pub first_discard: Option<(usize, Tile)>,
    pub hand_history: Vec<[i32; NUM_PLAYERS]>,
    pub last_breakdown: Option<HandBreakdown>,
    pub last_hand_snapshot: Option<HandSnapshot>,
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
            match_length: DEFAULT_MATCH_LENGTH,
            last_ji: None,
            turn_timer: 0.0,
            first_discard: None,
            hand_history: Vec::new(),
            last_breakdown: None,
            last_hand_snapshot: None,
        }
    }

    pub fn last_hand_deltas(&self) -> Option<[i32; NUM_PLAYERS]> {
        let n = self.hand_history.len();
        if n == 0 {
            return None;
        }
        let cur = self.hand_history[n - 1];
        let prev = if n >= 2 {
            self.hand_history[n - 2]
        } else {
            [0; NUM_PLAYERS]
        };
        Some(std::array::from_fn(|i| cur[i] - prev[i]))
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
                    if self.dealer == HUMAN_SEAT {
                        self.turn_timer = ctx.layout.turn_window;
                    }
                    events.push(GameEvent::DealComplete);
                } else {
                    self.phase = Phase::Deal { t: new_t };
                }
            }

            Phase::AwaitingDraw { player } => {
                if self.wall.is_empty() {
                    self.on_huangzhuang(&mut events);
                } else if let Some(tile) = self.wall.pop() {
                    if let Some(prev) = self.players[player].drawn.take() {
                        self.players[player].concealed.push(prev);
                        self.players[player].concealed.sort();
                    }
                    self.players[player].drawn = Some(tile);
                    events.push(GameEvent::Drew { player });
                    self.phase = Phase::DrawAnim { player, t: 0.0 };
                }
            }

            Phase::DrawAnim { player, t } => {
                let new_t = t + dt;
                if new_t >= ctx.layout.draw_duration {
                    if player == HUMAN_SEAT {
                        self.phase = Phase::AwaitingDiscard { player };
                        self.turn_timer = ctx.layout.turn_window;
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
                if player == HUMAN_SEAT {
                    self.turn_timer -= dt;
                    if self.turn_timer <= 0.0 {
                        let idx = if self.players[HUMAN_SEAT].drawn.is_some() {
                            self.players[HUMAN_SEAT].concealed.len()
                        } else {
                            0
                        };
                        self.human_discard(idx, ctx);
                    }
                } else {
                    let all = self.players[player].all_tiles();
                    if hand::is_winning_hand(&all, &self.players[player].melds) {
                        let method = HuMethod::Zimo;
                        self.on_hu(player, method, &mut events);
                    } else if let Some(tile) =
                        ai::decide_an_gang(&all, &self.players[player].melds)
                    {
                        self.apply_an_gang(player, tile, &mut events);
                    } else {
                        if let Some(d) = self.players[player].drawn.take() {
                            self.players[player].concealed.push(d);
                            self.players[player].concealed.sort();
                        }
                        let hand_ref = &self.players[player].concealed;
                        if !hand_ref.is_empty() {
                            let idx = ai::decide_discard(
                                hand_ref,
                                &self.players[player].melds,
                            );
                            let tile = self.players[player].concealed.remove(idx);
                            self.players[player].discards.push(tile);
                            self.record_discard(player, tile);
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
                let close = new_t <= 0.0 || !human_has_opts || human_decided;
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
                    if claimer == HUMAN_SEAT {
                        self.turn_timer = ctx.layout.turn_window;
                    }
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
                    self.extend_match();
                    events.push(GameEvent::DealComplete);
                } else if input.is_key_pressed(KeyCode::R) {
                    self.reset_match();
                    events.push(GameEvent::DealComplete);
                }
            }
        }
        events
    }

    fn capture_snapshot(&self, ji: JiInfo) -> HandSnapshot {
        let concealed: [Vec<Tile>; NUM_PLAYERS] =
            std::array::from_fn(|i| self.players[i].concealed.clone());
        let melds: [Vec<Meld>; NUM_PLAYERS] =
            std::array::from_fn(|i| self.players[i].melds.clone());
        let discards: [Vec<Tile>; NUM_PLAYERS] =
            std::array::from_fn(|i| self.players[i].discards.clone());
        HandSnapshot::capture(&concealed, &melds, &discards, ji)
    }

    fn on_hu(&mut self, winner: usize, method: HuMethod, events: &mut Vec<GameEvent>) {
        if let Some(d) = self.players[winner].drawn.take() {
            self.players[winner].concealed.push(d);
            self.players[winner].concealed.sort();
        }
        let ji = if let Some(flipped) = self.wall.pop() {
            scoring::determine_ji(flipped)
        } else {
            JiInfo::none()
        };
        self.last_ji = Some(ji);
        self.last_hand_snapshot = Some(self.capture_snapshot(ji));

        let breakdown = scoring::apply_hand_scores(
            &mut self.players,
            Some((winner, method)),
            &ji,
            self.dealer,
            self.first_discard,
        );
        self.last_breakdown = Some(breakdown);
        self.hand_history.push(std::array::from_fn(|i| self.players[i].score));

        self.hands_played += 1;
        events.push(GameEvent::Hu { player: winner, method });

        if self.hands_played >= self.match_length {
            let best = self.leading_player();
            self.phase = Phase::MatchOver { winner: best };
            events.push(GameEvent::MatchOver { winner: best });
        } else {
            self.phase = Phase::Hu { winner, method };
        }
    }

    fn on_huangzhuang(&mut self, events: &mut Vec<GameEvent>) {
        let tenpai: [bool; NUM_PLAYERS] = std::array::from_fn(|i| {
            hand::is_tenpai(
                &self.players[i].all_tiles(),
                &self.players[i].melds,
            )
        });
        let ji = self.last_ji.unwrap_or(JiInfo::none());
        self.last_hand_snapshot = Some(self.capture_snapshot(ji));
        self.last_breakdown = None;

        scoring::apply_huangzhuang_scores(&mut self.players, &tenpai);
        self.hand_history.push(std::array::from_fn(|i| self.players[i].score));

        self.hands_played += 1;
        events.push(GameEvent::HuangZhuang);

        if self.hands_played >= self.match_length {
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
            p.drawn = None;
            p.melds.clear();
            p.discards.clear();
        }
        self.turn = self.dealer;
        self.pending_human_claim = None;
        self.last_ji = None;
        self.turn_timer = 0.0;
        self.first_discard = None;
        self.last_breakdown = None;
        self.last_hand_snapshot = None;
        self.phase = Phase::Deal { t: 0.0 };
    }

    fn extend_match(&mut self) {
        self.match_length += MATCH_EXTENSION;
        self.start_next_hand();
    }

    fn reset_match(&mut self) {
        for p in &mut self.players {
            p.score = 0;
        }
        self.hands_played = 0;
        self.match_length = DEFAULT_MATCH_LENGTH;
        self.dealer = NUM_PLAYERS - 1;
        self.hand_history.clear();
        self.start_next_hand();
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

    fn record_discard(&mut self, player: usize, tile: Tile) {
        if self.first_discard.is_none() {
            self.first_discard = Some((player, tile));
        }
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
                let p = &mut self.players[player];
                if let Some(d) = p.drawn.take() {
                    p.concealed.push(d);
                    p.concealed.sort();
                }
                let mut removed = 0;
                p.concealed.retain(|&t| {
                    if t == discard && removed < 2 { removed += 1; false } else { true }
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
                let p = &mut self.players[player];
                if let Some(d) = p.drawn.take() {
                    p.concealed.push(d);
                    p.concealed.sort();
                }
                let mut removed = 0;
                p.concealed.retain(|&t| {
                    if t == discard && removed < 3 { removed += 1; false } else { true }
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
        self.players[player].all_tiles().iter().filter(|&&t| t == tile).count() >= 2
    }

    fn can_ming_gang(&self, player: usize, tile: Tile) -> bool {
        self.players[player].all_tiles().iter().filter(|&&t| t == tile).count() >= 3
    }

    fn can_hu(&self, player: usize, tile: Tile) -> bool {
        let mut test = self.players[player].all_tiles();
        test.push(tile);
        test.sort();
        hand::is_winning_hand(&test, &self.players[player].melds)
    }

    pub fn an_gang_tile(&self, player: usize) -> Option<Tile> {
        let all = self.players[player].all_tiles();
        let mut counts: std::collections::HashMap<Tile, usize> =
            std::collections::HashMap::new();
        for &t in &all { *counts.entry(t).or_insert(0) += 1; }
        counts.iter().find(|&(_, &c)| c == 4).map(|(&t, _)| t)
    }

    pub fn can_zimo(&self, player: usize) -> bool {
        hand::is_winning_hand(
            &self.players[player].all_tiles(),
            &self.players[player].melds,
        )
    }

    pub fn human_claim_options(
        &self,
        discard: Tile,
        from: usize,
    ) -> Option<Vec<ClaimKind>> {
        if from == HUMAN_SEAT { return None; }
        let mut kinds = Vec::new();
        if self.can_hu(HUMAN_SEAT, discard) { kinds.push(ClaimKind::Hu); }
        if self.can_ming_gang(HUMAN_SEAT, discard) { kinds.push(ClaimKind::Gang); }
        else if self.can_peng(HUMAN_SEAT, discard) { kinds.push(ClaimKind::Peng); }
        if kinds.is_empty() { None } else { Some(kinds) }
    }

    pub fn human_claim(&mut self, kind: ClaimKind) -> bool {
        let Phase::AwaitingClaims { discard, from, .. } = self.phase else {
            return false;
        };
        let Some(opts) = self.human_claim_options(discard, from) else {
            return false;
        };
        if !opts.contains(&kind) { return false; }
        self.pending_human_claim = Some(kind);
        true
    }

    pub fn human_pass(&mut self, ctx: &GameContext) -> bool {
        let Phase::AwaitingClaims { discard, from, .. } = self.phase else {
            return false;
        };
        let claim = self.resolve_claims(discard, from);
        self.pending_human_claim = None;
        let mut events = Vec::new();
        self.apply_claim(discard, from, claim, ctx, &mut events);
        true
    }

    fn resolve_claims(&self, discard: Tile, from: usize) -> Option<(usize, ClaimKind)> {
        let mut claims: Vec<(usize, ClaimKind)> = Vec::new();
        for p in 0..NUM_PLAYERS {
            if p == from { continue; }
            if p == HUMAN_SEAT {
                if let Some(kind) = self.pending_human_claim {
                    claims.push((p, kind));
                }
                continue;
            }
            if self.can_hu(p, discard) {
                claims.push((p, ClaimKind::Hu));
            } else if ai::decide_ming_gang(
                &self.players[p].all_tiles(),
                discard,
            ) {
                claims.push((p, ClaimKind::Gang));
            } else if self.can_peng(p, discard)
                && ai::decide_claim(
                    &self.players[p].all_tiles(),
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
        let p = &mut self.players[player];
        if let Some(d) = p.drawn.take() {
            p.concealed.push(d);
            p.concealed.sort();
        }
        let mut removed = 0;
        p.concealed.retain(|&t| {
            if t == tile && removed < 4 { removed += 1; false } else { true }
        });
        p.melds.push(Meld::Gang { tile, from: GangSource::An });
        events.push(GameEvent::Claimed {
            player,
            kind: ClaimKind::Gang,
            tile,
        });
        self.phase = Phase::AwaitingDraw { player };
    }

    pub fn human_discard(&mut self, idx: usize, ctx: &GameContext) -> Option<Tile> {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return None;
        }
        let p = &mut self.players[HUMAN_SEAT];
        let tile = if idx < p.concealed.len() {
            let t = p.concealed.remove(idx);
            if let Some(d) = p.drawn.take() {
                p.concealed.push(d);
                p.concealed.sort();
            }
            t
        } else if idx == p.concealed.len() {
            p.drawn.take()?
        } else {
            return None;
        };
        self.players[HUMAN_SEAT].discards.push(tile);
        self.record_discard(HUMAN_SEAT, tile);
        self.after_discard(HUMAN_SEAT, tile, ctx);
        Some(tile)
    }

    pub fn human_zimo(&mut self) -> bool {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return false;
        }
        if !self.can_zimo(HUMAN_SEAT) { return false; }
        let mut events = Vec::new();
        self.on_hu(HUMAN_SEAT, HuMethod::Zimo, &mut events);
        true
    }

    pub fn human_an_gang(&mut self, tile: Tile) -> bool {
        if !matches!(self.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
            return false;
        }
        if self.an_gang_tile(HUMAN_SEAT) != Some(tile) { return false; }
        let mut events = Vec::new();
        self.apply_an_gang(HUMAN_SEAT, tile, &mut events);
        true
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
            mouse_left_pressed: false, mouse_left_down: false, mouse_left_released: false,
            mouse_right_pressed: false, mouse_right_down: false, mouse_right_released: false,
            mouse_middle_pressed: false, mouse_middle_down: false, mouse_middle_released: false,
            keys_pressed: Vec::new(), keys_down: Vec::new(), keys_released: Vec::new(),
        }
    }

    fn key_input(k: KeyCode) -> Input {
        let mut i = empty_input();
        i.keys_pressed = vec![k];
        i
    }

    fn ctx() -> GameContext { GameContext::default_hermetic() }

    fn drive_to_discard(g: &mut Game, c: &GameContext, max_frames: usize) -> usize {
        for _ in 0..max_frames {
            if let Phase::AwaitingDiscard { player } = g.phase { return player; }
            g.update(&empty_input(), c, 1.0 / 60.0);
        }
        panic!("never reached AwaitingDiscard, phase = {:?}", g.phase);
    }

    fn drive_to_human_discard(g: &mut Game, c: &GameContext, max_frames: usize) {
        for _ in 0..max_frames {
            if matches!(g.phase, Phase::AwaitingDiscard { player: HUMAN_SEAT }) {
                return;
            }
            if matches!(g.phase, Phase::AwaitingClaims { .. }) {
                g.human_pass(c);
                continue;
            }
            if matches!(
                g.phase,
                Phase::Hu { .. } | Phase::HuangZhuang | Phase::MatchOver { .. }
            ) {
                panic!("drive_to_human_discard: game ended, phase = {:?}", g.phase);
            }
            g.update(&empty_input(), c, 1.0 / 60.0);
        }
        panic!("never reached human discard, phase = {:?}", g.phase);
    }

    fn winning_hand() -> Vec<Tile> {
        use crate::components::Suit;
        vec![
            Tile::new(Suit::Wan, 1), Tile::new(Suit::Wan, 2), Tile::new(Suit::Wan, 3),
            Tile::new(Suit::Wan, 4), Tile::new(Suit::Wan, 5), Tile::new(Suit::Wan, 6),
            Tile::new(Suit::Wan, 7), Tile::new(Suit::Wan, 8), Tile::new(Suit::Wan, 9),
            Tile::new(Suit::Tiao, 3), Tile::new(Suit::Tiao, 4), Tile::new(Suit::Tiao, 5),
            Tile::new(Suit::Tong, 5), Tile::new(Suit::Tong, 5),
        ]
    }

    #[test]
    fn test_new_game_starts_in_deal() {
        let g = Game::new(1);
        assert!(matches!(g.phase, Phase::Deal { .. }));
        assert_eq!(g.wall.len(), 108);
        assert_eq!(g.match_length, 16);
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
        assert_eq!(g.players[0].all_tiles().len(), 14);
    }

    #[test]
    fn test_human_discard_from_hand_merges_drawn() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        let discarded = g.human_discard(0, &c).expect("legal discard");
        assert_eq!(g.players[0].concealed.len(), 13);
        assert!(g.players[0].drawn.is_none());
        assert_eq!(g.players[0].discards[0], discarded);
    }

    #[test]
    fn test_human_discard_drawn_tile_directly() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        let drawn = g.players[0].drawn.expect("drawn");
        let concealed_len = g.players[0].concealed.len();
        let discarded = g.human_discard(concealed_len, &c).expect("legal discard");
        assert_eq!(discarded, drawn);
        assert!(g.players[0].drawn.is_none());
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
    fn test_ai_thinks_before_discarding() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.human_discard(0, &c).unwrap();
        let mut saw = false;
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AiThinking { .. }) { saw = true; break; }
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
            if matches!(g.phase, Phase::AiThinking { .. }) { break; }
        }
        assert_eq!(g.players[1].discards.len(), 0);
        for _ in 0..120 {
            g.update(&empty_input(), &c, 1.0 / 60.0);
            if matches!(g.phase, Phase::AwaitingDiscard { player: 1 }) { break; }
        }
        g.update(&empty_input(), &c, 1.0 / 60.0);
        assert_eq!(g.players[1].discards.len(), 1);
    }

    #[test]
    fn test_space_in_hu_starts_next_hand() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert!(matches!(g.phase, Phase::Hu { winner: 0, .. }));
        g.update(&key_input(KeyCode::Space), &c, 1.0 / 60.0);
        assert!(matches!(g.phase, Phase::Deal { .. }));
        assert_eq!(g.dealer, 1);
    }

    #[test]
    fn test_match_over_after_sixteen_hands() {
        let c = ctx();
        let mut g = Game::new(1);
        g.hands_played = 15;
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert!(matches!(g.phase, Phase::MatchOver { .. }));
    }

    #[test]
    fn test_match_over_space_extends_match() {
        let c = ctx();
        let mut g = Game::new(1);
        g.phase = Phase::MatchOver { winner: 0 };
        g.players[0].score = 42;
        g.match_length = 16;
        g.update(&key_input(KeyCode::Space), &c, 1.0 / 60.0);
        assert_eq!(g.players[0].score, 42);
        assert_eq!(g.match_length, 32);
        assert!(matches!(g.phase, Phase::Deal { .. }));
    }

    #[test]
    fn test_match_over_r_key_full_reset() {
        let c = ctx();
        let mut g = Game::new(1);
        g.phase = Phase::MatchOver { winner: 0 };
        g.players[0].score = 42;
        g.match_length = 32;
        g.update(&key_input(KeyCode::R), &c, 1.0 / 60.0);
        assert_eq!(g.players[0].score, 0);
        assert_eq!(g.match_length, 16);
    }

    #[test]
    fn test_turn_timer_reset_on_human_discard_enter() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        assert!((g.turn_timer - c.layout.turn_window).abs() < 1e-3);
    }

    #[test]
    fn test_turn_timer_decrements() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        let before = g.turn_timer;
        g.update(&empty_input(), &c, 1.0);
        assert!(g.turn_timer < before);
    }

    #[test]
    fn test_turn_timer_expires_auto_discards() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.turn_timer = 0.001;
        g.update(&empty_input(), &c, 1.0);
        assert_eq!(g.players[0].discards.len(), 1);
    }

    #[test]
    fn test_human_pass_resolves_immediately() {
        let c = ctx();
        let mut g = Game::new(1);
        let mut hand = vec![
            Tile::new(crate::components::Suit::Wan, 5),
            Tile::new(crate::components::Suit::Wan, 5),
        ];
        hand.extend(winning_hand().into_iter().take(11));
        g.players[0].concealed = hand;
        g.players[0].melds.clear();
        let discard = Tile::new(crate::components::Suit::Wan, 5);
        g.players[1].discards.push(discard);
        g.phase = Phase::AwaitingClaims {
            discard,
            from: 1,
            t: c.layout.claim_window,
        };
        assert!(g.human_pass(&c));
        assert!(!matches!(g.phase, Phase::AwaitingClaims { .. }));
    }

    #[test]
    fn test_hand_history_starts_empty() {
        let g = Game::new(1);
        assert!(g.hand_history.is_empty());
    }

    #[test]
    fn test_hand_history_records_after_hu() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert_eq!(g.hand_history.len(), 1);
    }

    #[test]
    fn test_hand_history_cleared_on_reset() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert_eq!(g.hand_history.len(), 1);

        g.phase = Phase::MatchOver { winner: 0 };
        g.update(&key_input(KeyCode::R), &c, 1.0 / 60.0);
        assert!(g.hand_history.is_empty());
    }

    #[test]
    fn test_hand_history_snapshots_are_zero_sum() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        let snap = g.hand_history[0];
        let sum: i32 = snap.iter().sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_last_hand_deltas_none_when_empty() {
        let g = Game::new(1);
        assert!(g.last_hand_deltas().is_none());
    }

    #[test]
    fn test_last_hand_deltas_after_first_hand() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        let deltas = g.last_hand_deltas().unwrap();
        let sum: i32 = deltas.iter().sum();
        assert_eq!(sum, 0);
        for (i, &d) in deltas.iter().enumerate() {
            assert_eq!(d, g.players[i].score);
        }
    }

    #[test]
    fn test_last_hand_deltas_are_incremental() {
        let mut g = Game::new(1);
        g.hand_history.push([10, -3, -3, -4]);
        g.hand_history.push([12, -6, 2, -8]);
        let deltas = g.last_hand_deltas().unwrap();
        assert_eq!(deltas, [2, -3, 5, -4]);
    }

    #[test]
    fn test_last_breakdown_none_initially() {
        let g = Game::new(1);
        assert!(g.last_breakdown.is_none());
    }

    #[test]
    fn test_last_breakdown_after_hu() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert!(g.last_breakdown.is_some());
    }

    #[test]
    fn test_last_breakdown_cleared_on_next_hand() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert!(g.last_breakdown.is_some());
        g.update(&key_input(KeyCode::Space), &c, 1.0 / 60.0);
        assert!(g.last_breakdown.is_none());
    }

    #[test]
    fn test_last_breakdown_matches_score_deltas() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        let bd = g.last_breakdown.unwrap();
        let deltas = g.last_hand_deltas().unwrap();
        for (i, &d) in deltas.iter().enumerate() {
            assert_eq!(bd.total(i), d);
        }
    }

    #[test]
    fn test_last_hand_snapshot_none_initially() {
        let g = Game::new(1);
        assert!(g.last_hand_snapshot.is_none());
    }

    #[test]
    fn test_last_hand_snapshot_captured_after_hu() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert!(g.last_hand_snapshot.is_some());
        let snap = g.last_hand_snapshot.as_ref().unwrap();
        // Winner's concealed should be the 14-tile winning hand.
        assert_eq!(snap.concealed[0].len(), 14);
    }

    #[test]
    fn test_last_hand_snapshot_cleared_on_next_hand() {
        let c = ctx();
        let mut g = Game::new(1);
        drive_to_discard(&mut g, &c, 600);
        g.players[0].concealed = winning_hand();
        g.players[0].drawn = None;
        g.human_zimo();
        assert!(g.last_hand_snapshot.is_some());
        g.update(&key_input(KeyCode::Space), &c, 1.0 / 60.0);
        assert!(g.last_hand_snapshot.is_none());
    }
}